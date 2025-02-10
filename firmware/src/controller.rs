use core::fmt::{self, Display, Formatter, Write};

use arduino_hal::{
    default_serial, delay_ms, pins,
    prelude::_unwrap_infallible_UnwrapInfallible, Peripherals, Pins,
};
use heapless::String;
use nb::block;
use ufmt::{uWrite, uwriteln};
use ufmt_macros::uwrite;

use crate::{
    command::{self, Command, Move},
    machine::{Machine, MoveMode},
    readln,
    uno::UnoSerial,
};

/// Size of the buffer used to read from the UART.
const READ_BUFFER_SZ: usize = 256;

/// Size of the buffer used to write to the UART.
///
/// This is necessary for formatting strings.
const WRITE_BUFFER_SZ: usize = 256;

/// Write an error message, expanding its arguments.
macro_rules! error {
    ($self:expr, $($arg:tt)*) => {{
        $self.output_buffer.clear();
        let result = write!($self.output_buffer, "ERROR: {}", format_args!($($arg)*));
        if result.is_err() {
            $self.writeln("ERROR: Buffer overflow when formatting output!");
        } else {
            $self.writeln_buffer();
        }
    }};
}

/// Write an info message, expanding its arguments.
macro_rules! info {
    ($self:expr, $($arg:tt)*) => {{
        $self.output_buffer.clear();
        let result = write!($self.output_buffer, "INFO: {}", format_args!($($arg)*));
        if result.is_err() {
            $self.writeln("ERROR: Buffer overflow when formatting output!");
        } else {
            $self.writeln_buffer();
        }
    }};
}

pub struct Controller {
    serial: UnoSerial,
    machine: Option<Machine>,
    input_buffer: String<READ_BUFFER_SZ>,
    output_buffer: String<WRITE_BUFFER_SZ>,
}
impl Controller {
    const BAUD_RATE: u32 = 57600;

    pub fn new() -> Self {
        let peripherals: Peripherals = unsafe { Peripherals::steal() };
        let pins: Pins = pins!(peripherals);

        let serial = default_serial!(peripherals, pins, Self::BAUD_RATE);
        let machine = None;
        let input_buffer = String::new();
        let output_buffer = String::new();

        let mut controller = Self {
            serial,
            machine,
            input_buffer,
            output_buffer,
        };
        controller.writeln("WINDERBOT!");
        controller
    }

    pub fn command_step(&mut self) {
        let result = match self.read_command() {
            Command::Zero => self.zero(),
            Command::AbsolutePositioning => self.absolute_positioning(),
            Command::RelativePositioning => self.relative_positioning(),
            Command::Move(mv) => self.do_move(mv),
        };
        /*
        let result = match self.read_command() {
            Command::Zero => self.zero(),
            Command::AbsolutePositioning => self.absolute_positioning(),
            Command::RelativePositioning => self.relative_positioning(),
            Command::Move(mv) => self.do_move(mv),
        };
        */

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
        /*
        info!(
            self,
            "Starting move: X={} microns, A={} millidegrees.", x, a
        );
        */
        info!(self, "Starting move.");
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

    /// Block trying to read commands from the UART, until reading a command
    /// succeeds.
    fn read_command(&mut self) -> Command {
        self.read_line();
        loop {
            match Command::parse(&mut self.input_buffer.as_str()) {
                Err(command::Error::InvalidGCode) => {
                    error!(
                        self,
                        "Invalid GCode \"{}\"",
                        self.input_buffer.as_str()
                    );
                }
                Ok(cmd) => return cmd,
            }
        }
    }

    /// Keep trying to read a line of input from the UART, until it succeeds.
    ///
    /// The line that was reqd is stored in `self.serial_buffer`.
    fn read_line(&mut self) {
        loop {
            match readln::readln(&mut self.serial, &mut self.input_buffer) {
                Ok(()) => break,
                Err(readln::Error::BufferOverflow) => {
                    error!(self, "Buffer overflow.")
                }
            }
        }
    }

    /// Write a line to the UART.
    fn writeln(&mut self, s: &str) {
        self.output_buffer.clear();
        self.output_buffer.write_str(s).unwrap(); // TODO
        self.writeln_buffer();
    }

    /// Write the output buffer to the UART.
    fn writeln_buffer(&mut self) {
        self.serial
            .write_str(self.output_buffer.as_str())
            .unwrap_infallible();
        self.serial.write_char('\n').unwrap_infallible();
        self.serial.flush();
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
