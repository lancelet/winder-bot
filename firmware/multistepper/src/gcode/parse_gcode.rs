use crate::{Microns, MilliDegrees};

use winnow::combinator::alt;
use winnow::token::{literal, take_while};
use winnow::{Parser, Result};

use super::parse_numbers::{
    parse_degrees_as_millidegrees, parse_digits_u8, parse_mm_as_microns,
};

/// GCode atoms.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum GCode {
    /// Linear axis position, like `X42.3`.
    Linear(Linear),
    /// Rotary axis position, like `A180`.
    Rotary(Rotary),
    /// G command, like `G0`.
    G(G),
    /// M command, like `M100`.
    M(M),
}

/// Linear axis move amount.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Linear {
    axis: LinAxis,
    amount: Microns,
}

/// Rotary axis move amount.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Rotary {
    axis: RotAxis,
    amount: MilliDegrees,
}

/// G command.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct G(u8);

/// M command.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct M(u8);

/// Linear axis.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum LinAxis {
    X,
    Y,
    Z,
}

/// Rotary axis.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum RotAxis {
    A,
    B,
    C,
}

/// Parse multiple GCodes, storing them in a buffer.
///
/// This function tries to parse as many [GCode]s as will fit in the `buffer`
/// before returning. It will return when either the input is empty, or when
/// the buffer is full. The function does not empty the buffer before
/// accumulating into it.
///
/// If the buffer fills up before the input has been read, the input will be
/// set to the next gcode.
///
/// If parsing fails, the buffer will still contain any GCodes that were parsed
/// until the failure.
///
/// # Parameters
///
/// - `input`: The input to parse.
/// - `buffer`: Buffer in which to accumulate values.
///
/// # Returns
///
/// - `Ok(completed)` if parsing was successful. `completed` is a boolean
///   which indicates hether the complete string was parsed without filling
///   up the buffer.
/// - `Err(_)` if the parsing failed.
pub fn parse_gcodes<'s, const N: usize>(
    input: &mut &'s str,
    buffer: &mut heapless::Vec<GCode, N>,
) -> Result<bool> {
    while !input.is_empty() {
        let prev_input = *input;
        let gcode = parse_trim_gcode.parse_next(input)?;
        if buffer.push(gcode).is_err() {
            *input = prev_input;
            break;
        }
    }
    Ok(input.is_empty())
}

/// Parse a GCode, trimming whitespace on either side.
fn parse_trim_gcode<'s>(input: &mut &'s str) -> Result<GCode> {
    skip_ws.parse_next(input)?;
    let result = parse_gcode.parse_next(input)?;
    skip_ws.parse_next(input)?;
    Ok(result)
}

/// Parse a GCode.
fn parse_gcode<'s>(input: &mut &'s str) -> Result<GCode> {
    alt((
        parse_linear.map(|linear| GCode::Linear(linear)),
        parse_rotary.map(|rotary| GCode::Rotary(rotary)),
        parse_g.map(|g| GCode::G(g)),
        parse_m.map(|m| GCode::M(m)),
    ))
    .parse_next(input)
}

/// Parse a Linear.
fn parse_linear<'s>(input: &mut &'s str) -> Result<Linear> {
    let axis = parse_linaxis.parse_next(input)?;
    skip_ws.parse_next(input)?;
    let amount = parse_mm_as_microns.parse_next(input)?;
    Ok(Linear { axis, amount })
}

/// Parse a Rotary.
fn parse_rotary<'s>(input: &mut &'s str) -> Result<Rotary> {
    let axis = parse_rotaxis.parse_next(input)?;
    skip_ws.parse_next(input)?;
    let amount = parse_degrees_as_millidegrees.parse_next(input)?;
    Ok(Rotary { axis, amount })
}

/// Parse a "G" command.
fn parse_g<'s>(input: &mut &'s str) -> Result<G> {
    let _ = literal("G").parse_next(input)?;
    skip_ws.parse_next(input)?;
    let value = parse_digits_u8.parse_next(input)?;
    Ok(G(value))
}

/// Parse an "M" command.
fn parse_m<'s>(input: &mut &'s str) -> Result<M> {
    let _ = literal("M").parse_next(input)?;
    skip_ws.parse_next(input)?;
    let value = parse_digits_u8.parse_next(input)?;
    Ok(M(value))
}

/// Skip whitespace when parsing.
fn skip_ws<'s>(input: &mut &'s str) -> Result<()> {
    take_while(0.., char::is_whitespace)
        .parse_next(input)
        .map(|_| ())
}

/// Parse a LinAxis.
fn parse_linaxis<'s>(input: &mut &'s str) -> Result<LinAxis> {
    alt((
        literal("X").map(|_| LinAxis::X),
        literal("Y").map(|_| LinAxis::Y),
        literal("Z").map(|_| LinAxis::Z),
    ))
    .parse_next(input)
}

/// Parse a RotAxis.
fn parse_rotaxis<'s>(input: &mut &'s str) -> Result<RotAxis> {
    alt((
        literal("A").map(|_| RotAxis::A),
        literal("B").map(|_| RotAxis::B),
        literal("C").map(|_| RotAxis::C),
    ))
    .parse_next(input)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_gcode_examples() {
        let mut input1 = "G 100";
        let mut input2 = "X-100";
        let mut input3 = "A 32.5";
        let mut input4 = "M3";
        let expected1 = GCode::G(G(100));
        let expected2 = GCode::Linear(Linear {
            axis: LinAxis::X,
            amount: Microns::new(-100000),
        });
        let expected3 = GCode::Rotary(Rotary {
            axis: RotAxis::A,
            amount: MilliDegrees::new(32500),
        });
        let expected4 = GCode::M(M(3));
        assert_eq!(Ok(expected1), parse_gcode.parse_next(&mut input1));
        assert_eq!(Ok(expected2), parse_gcode.parse_next(&mut input2));
        assert_eq!(Ok(expected3), parse_gcode.parse_next(&mut input3));
        assert_eq!(Ok(expected4), parse_gcode.parse_next(&mut input4));
    }

    #[test]
    fn test_parse_gcodes_complete() {
        let mut input = "G0 X-100.02 A42.8";
        let expected: heapless::Vec<GCode, 16> = heapless::Vec::from_slice(&[
            GCode::G(G(0)),
            GCode::Linear(Linear {
                axis: LinAxis::X,
                amount: Microns::new(-100020),
            }),
            GCode::Rotary(Rotary {
                axis: RotAxis::A,
                amount: MilliDegrees::new(42800),
            }),
        ])
        .unwrap();
        let mut buffer: heapless::Vec<GCode, 16> = heapless::Vec::new();

        let result = parse_gcodes(&mut input, &mut buffer);
        assert_eq!(Ok(true), result);
        assert_eq!(expected, buffer);
    }

    #[test]
    fn test_parse_gcodes_incomplete() {
        let input: String = format!("G0 X-100.02 A42.8");
        let mut input_ref: &str = &input;
        let input_ref2: &str = &input;
        let expected: heapless::Vec<GCode, 2> = heapless::Vec::from_slice(&[
            GCode::G(G(0)),
            GCode::Linear(Linear {
                axis: LinAxis::X,
                amount: Microns::new(-100020),
            }),
        ])
        .unwrap();
        let mut buffer: heapless::Vec<GCode, 2> = heapless::Vec::new();

        let result = parse_gcodes(&mut input_ref, &mut buffer);
        assert_eq!(Ok(false), result);
        assert_eq!(&input_ref2[12..], input_ref);
        assert_eq!(expected, buffer);
    }

    #[test]
    fn test_parse_gcodes_error() {
        let input: String = format!("G0 garbledgarbled");
        let mut input_ref: &str = &input;
        let input_ref2: &str = &input;
        let expected: heapless::Vec<GCode, 2> =
            heapless::Vec::from_slice(&[GCode::G(G(0))]).unwrap();
        let mut buffer: heapless::Vec<GCode, 2> = heapless::Vec::new();

        let result = parse_gcodes(&mut input_ref, &mut buffer);
        assert!(result.is_err());
        assert_eq!(&input_ref2[3..], input_ref);
        assert_eq!(expected, buffer);
    }
}
