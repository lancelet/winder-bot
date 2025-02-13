use crate::commands::command::Command;
use multistepper::gcode::{parse_gcodes, GCode};
use ufmt_macros::uDebug;
use winnow::{error::ContextError, token::one_of, Parser};

/// Command Parser.
///
/// This is a struct because it statically owns a buffer used to read GCode.
pub struct CommandParser<const N_GCODES: usize> {
    buffer: heapless::Vec<GCode, N_GCODES>,
}
impl<const N_GCODES: usize> CommandParser<N_GCODES> {
    /// Creates a new command parser.
    pub fn new() -> Self {
        Self {
            buffer: heapless::Vec::new(),
        }
    }

    /// Parses a command.
    pub fn parse(&mut self, input: &str) -> Result<Command, Error> {
        self.buffer.clear();
        let mut input_ref: &str = &input;
        match parse_gcodes(&mut input_ref, &mut self.buffer) {
            Err(_) => Err(Error::ParseError),
            Ok(false) => Err(Error::BufferOverflow),
            Ok(true) => {
                let mut tok_input = self.buffer.as_slice();
                parse_simple_g(28, Command::Home)
                    .parse_next(&mut tok_input)
                    .map_err(|_| Error::ParseError)
            }
        }
    }
}

/// Possible errors that might occur during parsing.
#[derive(uDebug)]
pub enum Error {
    /// Input buffer overflowed.
    BufferOverflow,
    /// Parsing failed.
    ParseError,
}

/// Parse a simple `Gxxx` code that has nothing except its numeric part.
///
/// # Parameters
///
/// - `input`: The input slice of `GCode`s.
/// - `value`: The numeric `G<value>`.
/// - `command`: The corresponding `Command`.
fn parse_simple_g<'s>(
    value: u8,
    command: Command,
) -> impl Fn(&mut &'s [GCode]) -> Result<Command, ContextError> {
    move |input| one_of(GCode::g(value)).parse_next(input).map(|_| command)
}
