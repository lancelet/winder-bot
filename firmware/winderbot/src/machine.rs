use crate::{
    commands::{
        command::{Command, Move},
        command_parser::{self, CommandParser},
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
use multistepper::{
    Direction, LimitedStepper, LinearConverter, MicroSeconds,
    PositionedStepper, RotaryConverter, Steps,
};
use ufmt::uwriteln;

pub struct Machine {
    x_axis: LimitedStepper<Stepper<D8, D9>, LimitSwitch<D13>, LimitSwitch<D12>>,
    a_axis: PositionedStepper<Stepper<D10, D11>>,
    x_params: LinearConverter,
    a_params: RotaryConverter,
    serial: Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>>,
    uart_input_buffer: heapless::String<80>,
    command_parser: CommandParser<8>,
    zeroed: bool,
    move_mode: MoveMode,
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

        // Axis parameters.
        let x_params = LinearConverter::new(6400, 5000);
        let a_params = RotaryConverter::new(6400);

        // UART
        let uart_input_buffer = heapless::String::new();

        // Command parser and GCode buffer.
        let command_parser = CommandParser::new();

        // Move mode.
        let move_mode = MoveMode::Absolute;

        Self {
            x_axis,
            a_axis,
            x_params,
            a_params,
            serial,
            uart_input_buffer,
            command_parser,
            zeroed: false,
            move_mode,
        }
    }

    pub fn next_command(&mut self) {
        use MoveMode::*;
        let result = match self.block_for_next_command() {
            Command::Move(mv) => self.run_move(&mv),
            Command::Home => self.run_home(),
            Command::AbsolutePositioning => self.set_move_mode(Absolute),
            Command::RelativePositioning => self.set_move_mode(Relative),
        };
        match result {
            Ok(()) => {
                self.print_position();
                uwriteln!(&mut self.serial, "Ok.").unwrap_infallible()
            }
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

    /// Sets the move mode.
    fn set_move_mode(&mut self, move_mode: MoveMode) -> Result<(), Error> {
        self.move_mode = move_mode;
        match self.move_mode {
            MoveMode::Absolute => {
                uwriteln!(&mut self.serial, "INFO: Move mode: Absolute.")
                    .unwrap_infallible()
            }
            MoveMode::Relative => {
                uwriteln!(&mut self.serial, "INFO: Move mode: Relative.")
                    .unwrap_infallible()
            }
        }
        Ok(())
    }

    /// Run the homing routine.
    fn run_home(&mut self) -> Result<(), Error> {
        let move_delay = MicroSeconds::new(50);
        let soft_safety_margin = Steps::new(6400);
        match self
            .x_axis
            .run_zeroing::<Delay>(move_delay, soft_safety_margin)
        {
            None => Err(Error::HomingFailed),
            Some(_steps) => {
                self.zeroed = true;
                Ok(())
            }
        }
    }

    /// Runs a move command.
    fn run_move(&mut self, mv: &Move) -> Result<(), Error> {
        // Must be zeroed before doing anything.
        if !self.zeroed {
            return Err(Error::NotZeroed);
        }

        let step_delay = MicroSeconds::new(50);

        match self.move_mode {
            MoveMode::Absolute => {
                let steps_x = self
                    .x_params
                    .to_steps(mv.x_amount)
                    .ok_or(Error::Overflow)?;
                let steps_a = self
                    .a_params
                    .to_steps(mv.a_amount)
                    .ok_or(Error::Overflow)?;

                self.sync_move_to(steps_x, steps_a, step_delay)
            }
            MoveMode::Relative => {
                let dx = self
                    .x_params
                    .to_steps(mv.x_amount)
                    .ok_or(Error::Overflow)?;
                let da = self
                    .a_params
                    .to_steps(mv.a_amount)
                    .ok_or(Error::Overflow)?;

                let steps_x = self
                    .x_axis
                    .get_position()
                    .get_value()
                    .checked_add(dx.get_value())
                    .map(Steps::new)
                    .ok_or(Error::Overflow)?;
                let steps_a = self
                    .a_axis
                    .get_position()
                    .get_value()
                    .checked_add(da.get_value())
                    .map(Steps::new)
                    .ok_or(Error::Overflow)?;
                uwriteln!(
                    &mut self.serial,
                    "INFO: Relative move: X{} A{}: Steps: X{} A{} -> X{} A{}",
                    mv.x_amount,
                    mv.a_amount,
                    self.x_axis.get_position().get_value(),
                    self.a_axis.get_position().get_value(),
                    steps_x.get_value(),
                    steps_a.get_value()
                )
                .unwrap_infallible();
                self.sync_move_to(steps_x, steps_a, step_delay)
            }
        }
    }

    fn sync_move_to(
        &mut self,
        steps_x: Steps,
        steps_a: Steps,
        step_delay: MicroSeconds,
    ) -> Result<(), Error> {
        let delta_x = steps_x
            .get_value()
            .checked_sub(self.x_axis.get_position().get_value())
            .ok_or(Error::Overflow)?;
        let delta_a = steps_a
            .get_value()
            .checked_sub(self.a_axis.get_position().get_value())
            .ok_or(Error::Overflow)?;

        let start_x = self.x_axis.get_position().get_value();
        let start_a = self.a_axis.get_position().get_value();

        let move_steps: i64 = i32::max(delta_x.abs(), delta_a.abs()) as i64;

        for i in 1..=move_steps {
            let target_x: i32 = (delta_x as i64)
                .checked_mul(i)
                .and_then(|q| q.checked_div(move_steps))
                .and_then(|q| q.try_into().ok())
                .and_then(|q: i32| q.checked_add(start_x))
                .ok_or(Error::Overflow)?;
            let target_a: i32 = (delta_a as i64)
                .checked_mul(i)
                .and_then(|q| q.checked_div(move_steps))
                .and_then(|q| q.try_into().ok())
                .and_then(|q: i32| q.checked_add(start_a))
                .ok_or(Error::Overflow)?;

            self.move_x_to(Steps::new(target_x), step_delay)?;
            self.move_a_to(Steps::new(target_a), step_delay)?;
        }

        Ok(())
    }

    fn move_x_to(
        &mut self,
        steps_x: Steps,
        step_delay: MicroSeconds,
    ) -> Result<(), Error> {
        use multistepper::Delay;

        let direction = if steps_x > self.x_axis.get_position() {
            Direction::Positive
        } else {
            Direction::Negative
        };

        while self.x_axis.get_position() != steps_x {
            if self.x_axis.do_try_step(direction).is_none() {
                return Err(Error::MoveNotCompleted);
            }

            crate::devices::delay::Delay::delay_us(step_delay);
        }

        Ok(())
    }

    fn move_a_to(
        &mut self,
        steps_a: Steps,
        step_delay: MicroSeconds,
    ) -> Result<(), Error> {
        use multistepper::Delay;

        let direction = if steps_a > self.a_axis.get_position() {
            Direction::Positive
        } else {
            Direction::Negative
        };

        while self.a_axis.get_position() != steps_a {
            if self.a_axis.do_try_step(direction).is_none() {
                return Err(Error::MoveNotCompleted);
            }
            crate::devices::delay::Delay::delay_us(step_delay);
        }

        Ok(())
    }

    /// Print the machine position.
    fn print_position(&mut self) {
        let x = match self.x_params.to_microns(self.x_axis.get_position()) {
            None => {
                uwriteln!(
                    &mut self.serial,
                    "ERROR: print_position: microns: Overflow!"
                )
                .unwrap_infallible();
                return;
            }
            Some(q) => q,
        };
        let a = match self.a_params.to_millidegrees(self.a_axis.get_position())
        {
            None => {
                uwriteln!(
                    &mut self.serial,
                    "ERROR: print_position: millidegrees: Overflow!"
                )
                .unwrap_infallible();
                return;
            }
            Some(q) => q,
        };

        uwriteln!(&mut self.serial, "X{} A{}", x, a).unwrap_infallible();
    }

    /// Print one of the errors from this module.
    fn print_error(&mut self, error: Error) {
        use Error::*;
        match error {
            HomingFailed => {
                uwriteln!(&mut self.serial, "ERROR: Homing failed.")
                    .unwrap_infallible()
            }
            NotZeroed => uwriteln!(&mut self.serial, "ERROR: Not zeroed.")
                .unwrap_infallible(),
            MoveNotCompleted => {
                uwriteln!(&mut self.serial, "ERROR: Move not completed.")
                    .unwrap_infallible()
            }
            Overflow => {
                uwriteln!(&mut self.serial, "ERROR: Overflow detected.")
                    .unwrap_infallible()
            }
        }
    }
}

enum Error {
    HomingFailed,
    NotZeroed,
    MoveNotCompleted,
    Overflow,
}

enum MoveMode {
    Absolute,
    Relative,
}
