use core::fmt::{self, Display, Formatter};

use arduino_hal::{
    default_serial, pins, prelude::_unwrap_infallible_UnwrapInfallible,
    Peripherals, Pins,
};
use heapless::String;
use ufmt::uWrite;

use crate::{
    command::{self, Command, Move},
    machine::{Machine, MoveMode},
    readln,
    uno::UnoSerial,
};

/// Write an error message, expanding its arguments.
macro_rules! error {
    ($self:expr, $($arg:tt)*) => {{
        use core::fmt::Write;
        let mut out: heapless::String<512> = heapless::String::new();
        write!(out, "ERROR: {}", format_args!($($arg)*)).unwrap();
        $self.writeln(out.as_str());
    }};
}

/// Write an info message, expanding its arguments.
macro_rules! info {
    ($self:expr, $($arg:tt)*) => {{
        use core::fmt::Write;
        let mut out: heapless::String<512> = heapless::String::new();
        write!(out, "INFO: {}", format_args!($($arg)*)).unwrap();
        $self.writeln(out.as_str());
    }};
}

pub struct Controller {
    serial: UnoSerial,
    machine: Option<Machine>,
}
impl Controller {
    const BAUD_RATE: u32 = 57600;

    pub fn new() -> Self {
        let peripherals: Peripherals = unsafe { Peripherals::steal() };
        let pins: Pins = pins!(peripherals);

        let serial = default_serial!(peripherals, pins, Self::BAUD_RATE);
        let machine = None;

        Self { serial, machine }
    }

    pub fn command_step(&mut self) {
        let result = match self.read_command() {
            Command::Zero => self.zero(),
            Command::AbsolutePositioning => self.absolute_positioning(),
            Command::RelativePositioning => self.relative_positioning(),
            Command::Move(mv) => self.do_move(mv),
        };

        match result {
            Ok(()) => self.writeln("Ok."),
            Err(error) => error!(self, "{}", error),
        }
    }

    fn zero(&mut self) -> Result<(), Error> {
        info!(self, "Starting to zero the machine.");
        self.machine = Some(Machine::new());
        info!(self, "Completed zeroing the machine.");
        Ok(())
    }

    fn absolute_positioning(&mut self) -> Result<(), Error> {
        self.machine()?.set_move_mode(MoveMode::Absolute);
        info!(self, "Set absolute positioning mode.");
        Ok(())
    }

    fn relative_positioning(&mut self) -> Result<(), Error> {
        self.machine()?.set_move_mode(MoveMode::Relative);
        info!(self, "Set relative positioning mode.");
        Ok(())
    }

    fn do_move(&mut self, mv: Move) -> Result<(), Error> {
        let x = mv.x_microns();
        let a = mv.a_millidegrees();
        info!(
            self,
            "Starting move: X={} microns, A={} millidegrees.", x, a
        );
        self.machine()?
            .move_millis(mv.x_microns(), mv.a_millidegrees());
        info!(self, "Completed move.");
        Ok(())
    }

    /// Return the zeroed machine, otherwise return an error indicating that
    /// the machine must still be zeroed.
    fn machine(&mut self) -> Result<&mut Machine, Error> {
        match &mut self.machine {
            None => Err(Error::NotZeroed),
            Some(m) => Ok(m),
        }
    }

    /// Keep trying to read commands from the UART, until reading a command
    /// succeeds.
    fn read_command(&mut self) -> Command {
        let mut serial_buffer = String::<512>::new();
        self.read_line(&mut serial_buffer);
        loop {
            match Command::parse(&mut serial_buffer.as_str()) {
                Err(command::Error::InvalidGCode) => {
                    error!(
                        self,
                        "Invalid GCode \"{}\"",
                        serial_buffer.as_str()
                    );
                }
                Ok(cmd) => return cmd,
            }
        }
    }

    /// Keep trying to read a line of input from the UART, until it succeeds.
    ///
    /// The line that was reqd is stored in `self.serial_buffer`.
    fn read_line<const SZ: usize>(&mut self, serial_buffer: &mut String<SZ>) {
        loop {
            match readln::readln(&mut self.serial, serial_buffer) {
                Ok(()) => break,
                Err(readln::Error::BufferOverflow) => {
                    error!(self, "Buffer overflow.")
                }
            }
        }
    }

    /// Write a line to the UART.
    fn writeln(&mut self, s: &str) {
        self.serial.write_str(s).unwrap_infallible();
        self.serial.write_char('\n').unwrap_infallible();
    }
}

enum Error {
    NotZeroed,
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Error::NotZeroed => write!(f, "Machine not zeroed."),
        }
    }
}
