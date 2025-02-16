use crate::commands::command::{Command, Move};
use multistepper::{
    gcode::{parse_gcodes, GCode, LinAxis, Linear, RotAxis, Rotary},
    Microns, MilliDegrees,
};
use ufmt_macros::uDebug;
use winnow::{combinator::alt, error::ContextError, token::one_of, Parser};

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
                alt((
                    parse_move_g,
                    parse_simple_g(28, Command::Home),
                    parse_simple_g(90, Command::AbsolutePositioning),
                    parse_simple_g(91, Command::RelativePositioning),
                ))
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

/// Parse a move command `G0 ...`.
fn parse_move_g<'s>(input: &mut &'s [GCode]) -> Result<Command, ContextError> {
    one_of(GCode::g(0)).parse_next(input)?;

    let mut mve = Move {
        x_amount: Microns::new(0),
        a_amount: MilliDegrees::new(0),
    };
    while !input.is_empty() {
        match parse_axis_amount(input) {
            Err(err) => return Err(err),
            Ok(AxisAmount::Linear(Linear {
                axis: LinAxis::X,
                amount: microns,
            })) => mve.x_amount = microns,
            Ok(AxisAmount::Rotary(Rotary {
                axis: RotAxis::A,
                amount: mdg,
            })) => mve.a_amount = mdg,
            _ => return Err(ContextError::new()),
        }
    }

    Ok(Command::Move(mve))
}

/// An amount to move along an axis.
enum AxisAmount {
    Linear(Linear),
    Rotary(Rotary),
}

/// Parse an axis amount command.
fn parse_axis_amount<'s>(
    input: &mut &'s [GCode],
) -> Result<AxisAmount, ContextError> {
    if let Some((first, rest)) = input.split_first() {
        *input = rest;
        match first {
            GCode::Linear(linear) => Ok(AxisAmount::Linear(*linear)),
            GCode::Rotary(rotary) => Ok(AxisAmount::Rotary(*rotary)),
            _ => Err(ContextError::new()),
        }
    } else {
        Err(ContextError::new())
    }
}
