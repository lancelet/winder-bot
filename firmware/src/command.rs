use ufmt_macros::uDebug;
use winnow::{
    ascii::{digit1, space1},
    combinator::{alt, opt},
    token::literal,
    Parser, Result,
};

#[derive(Debug, uDebug)]
pub enum Command {
    Zero,
    AbsolutePositioning,
    RelativePositioning,
    Move(Move),
}
impl Command {
    pub fn parse<'a>(
        input: &mut &'a str,
    ) -> core::result::Result<Command, Error> {
        let result = alt((
            Self::parse_zero,
            Self::parse_absolute_positioning,
            Self::parse_relative_positioning,
            Self::parse_move,
        ))
        .parse(input);

        match result {
            Ok(cmd) => Ok(cmd),
            Err(_) => Err(Error::InvalidGCode),
        }
    }

    fn parse_zero<'a>(input: &mut &'a str) -> Result<Command> {
        literal("Z").parse_next(input).map(|_| Command::Zero)
    }

    fn parse_absolute_positioning<'a>(input: &mut &'a str) -> Result<Command> {
        literal("G90")
            .parse_next(input)
            .map(|_| Command::AbsolutePositioning)
    }

    fn parse_relative_positioning<'a>(input: &mut &'a str) -> Result<Command> {
        literal("G91")
            .parse_next(input)
            .map(|_| Command::RelativePositioning)
    }

    fn parse_move<'a>(input: &mut &'a str) -> Result<Command> {
        literal("G0").parse_next(input)?;
        let x_microns = opt((space1, Self::parse_x))
            .map(|t| t.map(|(_, x)| x))
            .parse_next(input)?;
        let a_millidegrees = opt((space1, Self::parse_a))
            .map(|t| t.map(|(_, a)| a))
            .parse_next(input)?;
        Ok(Command::Move(Move {
            x_microns,
            a_millidegrees,
        }))
    }

    fn parse_x<'a>(input: &mut &'a str) -> Result<i32> {
        literal("X").parse_next(input)?;
        Self::parse_decimal_millis(input)
    }

    fn parse_a<'a>(input: &mut &'a str) -> Result<i32> {
        literal("A").parse_next(input)?;
        Self::parse_decimal_millis(input)
    }

    /// Parse a decmial value with thousandths precision.
    ///
    /// eg.
    ///   - 3      -> 3000
    ///   - 3.14   -> 3140
    ///   - 3.142  -> 3142
    ///   - 3.1428 -> 3142
    fn parse_decimal_millis<'a>(input: &mut &'a str) -> Result<i32> {
        let sign: i32 = opt(alt((literal("-"), literal("+"))))
            .parse_next(input)?
            .map(|s| if s == "-" { -1 } else { 1 })
            .unwrap_or(1);

        let before_decimal: i32 =
            digit1.try_map(str::parse).parse_next(input)?;

        let opt_decimal = opt(literal(".")).parse_next(input)?;
        let after_decimal: i32 = match opt_decimal {
            None => 0,
            Some(_) => {
                let mut s: &str = digit1(input)?;
                s = &s[..3.min(s.len())];
                let factor = 10_i32.pow(3 - s.len() as u32);
                str::parse::<i32>(s).unwrap() * factor
            }
        };

        let value = sign * (before_decimal * 1000 + after_decimal);

        Ok(value)
    }
}

pub enum Error {
    InvalidGCode,
}

#[derive(Debug, uDebug)]
pub struct Move {
    x_microns: Option<i32>,
    a_millidegrees: Option<i32>,
}
impl Move {
    pub fn x_microns(&self) -> i32 {
        self.x_microns.unwrap_or(0)
    }
    pub fn a_millidegrees(&self) -> i32 {
        self.a_millidegrees.unwrap_or(0)
    }
}
