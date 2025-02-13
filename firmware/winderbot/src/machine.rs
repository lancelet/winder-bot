use crate::{
    commands::{
        command::Command, command_parser, command_parser::CommandParser,
    },
    devices::{
        delay::Delay, limitswitch::LimitSwitch, read_uart, stepper::Stepper,
    },
};
use arduino_hal::{
    default_serial, delay_ms,
    hal::port::{PD0, PD1},
    pac::USART0,
    pins,
    port::{
        mode::{Input, Output},
        Pin, D10, D11, D12, D13, D8, D9,
    },
    prelude::_unwrap_infallible_UnwrapInfallible,
    Peripherals, Pins, Usart,
};
use multistepper::{LimitedStepper, MicroSeconds, PositionedStepper, Steps};
use ufmt::uwriteln;

pub struct Machine {
    x_axis: LimitedStepper<Stepper<D8, D9>, LimitSwitch<D13>, LimitSwitch<D12>>,
    a_axis: PositionedStepper<Stepper<D10, D11>>,
    serial: Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>>,
    uart_input_buffer: heapless::String<80>,
    command_parser: CommandParser<8>,
}

impl Machine {
    const BAUD_RATE: u32 = 57600;

    pub fn new() -> Self {
        let peripherals: Peripherals = unsafe { Peripherals::steal() };
        let pins: Pins = pins!(peripherals);
        let mut serial = default_serial!(peripherals, pins, Self::BAUD_RATE);

        // Announce the machine!
        delay_ms(100);
        uwriteln!(&mut serial, "WINDERBOT").unwrap_infallible();

        // Delays
        let delay_pulse = MicroSeconds::new(5);
        let delay_direction = MicroSeconds::new(10);

        // Axes
        let x_stepper = Stepper::new(
            pins.d8.into_output(),
            pins.d9.into_output(),
            delay_pulse,
            delay_direction,
        );
        let x_limitswitch_pos = LimitSwitch::new(pins.d13.into_pull_up_input());
        let x_limitswitch_neg = LimitSwitch::new(pins.d12.into_pull_up_input());
        let a_stepper = Stepper::new(
            pins.d10.into_output(),
            pins.d11.into_output(),
            delay_pulse,
            delay_direction,
        );

        let x_axis = LimitedStepper::from_stepper(
            x_stepper,
            x_limitswitch_pos,
            x_limitswitch_neg,
        );
        let a_axis = PositionedStepper::new(a_stepper);

        // UART
        let uart_input_buffer = heapless::String::new();

        // Command parser and GCode buffer.
        let command_parser = CommandParser::new();

        Self {
            x_axis,
            a_axis,
            serial,
            uart_input_buffer,
            command_parser,
        }
    }

    pub fn next_command(&mut self) {
        let result = match self.block_for_next_command() {
            Command::Home => self.run_home(),
        };
        match result {
            Ok(()) => uwriteln!(&mut self.serial, "Ok.").unwrap_infallible(),
            Err(err) => self.print_error(err),
        }
    }

    /// Block waiting for the next valid command.
    fn block_for_next_command(&mut self) -> Command {
        loop {
            match read_uart::readln(
                &mut self.serial,
                &mut self.uart_input_buffer,
            ) {
                Err(read_uart::Error::BufferOverflow) => {
                    uwriteln!(&mut self.serial, "ERROR: UART buffer overflow.")
                        .unwrap_infallible()
                }
                Ok(()) => {
                    match self.command_parser.parse(&self.uart_input_buffer) {
                        Err(command_parser::Error::BufferOverflow) => {
                            uwriteln!(
                                &mut self.serial,
                                "ERROR: GCode buffer overflow."
                            )
                            .unwrap_infallible()
                        }
                        Err(command_parser::Error::ParseError) => uwriteln!(
                            &mut self.serial,
                            "ERROR: Could not parse input: \"{}\".",
                            &self.uart_input_buffer as &str
                        )
                        .unwrap_infallible(),
                        Ok(cmd) => return cmd,
                    }
                }
            }
        }
    }

    /// Run the homing routine.
    fn run_home(&mut self) -> Result<(), Error> {
        let move_delay = MicroSeconds::new(50);
        let soft_safety_margin = Steps::new(1024);
        match self
            .x_axis
            .run_zeroing::<Delay>(move_delay, soft_safety_margin)
        {
            None => Err(Error::HomingFailed),
            Some(_steps) => Ok(()),
        }
    }

    /// Print one of the errors from this module.
    fn print_error(&mut self, error: Error) {
        use Error::*;
        match error {
            HomingFailed => {
                uwriteln!(&mut self.serial, "ERROR: Homing Failed.")
                    .unwrap_infallible()
            }
        }
    }
}

enum Error {
    HomingFailed,
}
