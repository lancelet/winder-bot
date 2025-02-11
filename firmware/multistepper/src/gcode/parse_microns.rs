use crate::Microns;
use winnow::ascii::digit1;
use winnow::combinator::{alt, opt};
use winnow::token::literal;
use winnow::{Parser, Result};

/// Parses a decimal value in mm as a value in microns.
///
/// This permits only decimal notation, NOT scientific notation. A maximum of
/// 3 decimal places is allowed following the decimal point.
///
/// Examples of valid input:
///
/// - `"123.456"`
/// - `"+123.45"`
/// - `"-123.0"`
pub fn parse_mm_as_microns<'s>(input: &mut &'s str) -> Result<Microns> {
    let sign = {
        match opt(parse_sign).parse_next(input)? {
            None => 1,
            Some(sign) => sign.to_i32(),
        }
    };
    let int_part = parse_digits_i32.parse_next(input)?;
    let frac_part = {
        match opt(parse_period).parse_next(input)? {
            None => 0i32,
            Some(()) => parse_fractional_digits::<3>.parse_next(input)?,
        }
    };

    let value: i32 = sign * (int_part * 1000 + frac_part);
    Ok(Microns::new(value))
}

/// Represents a sign when parsing numbers.
#[derive(Debug, PartialEq, Copy, Clone)]
enum Sign {
    Plus,
    Minus,
}
impl Sign {
    fn to_i32(&self) -> i32 {
        use Sign::*;
        match self {
            Plus => 1,
            Minus => -1,
        }
    }
}

/// Parse a sign indicator ("+" or "-").
fn parse_sign<'s>(input: &mut &'s str) -> Result<Sign> {
    alt((
        literal("+").map(|_| Sign::Plus),
        literal("-").map(|_| Sign::Minus),
    ))
    .parse_next(input)
}

/// Parse digits (0-9) as an i32.
fn parse_digits_i32<'s>(input: &mut &'s str) -> Result<i32> {
    digit1.try_map(str::parse).parse_next(input)
}

/// Parse digits (0-9) as "fractional digits".
///
/// This means that the digits will be padded with zeros up to `N` width.
fn parse_fractional_digits<'s, const N: usize>(
    input: &mut &'s str,
) -> Result<i32> {
    digit1
        .try_map(|s: &str| {
            let n_digits = s.len();
            if n_digits > N {
                Err("too many fractional digits")
            } else {
                s.parse::<i32>()
                    .map(|number| number * 10i32.pow((N - n_digits) as u32))
                    .map_err(|_| "could not parse digits as i32")
            }
        })
        .parse_next(input)
}

/// Parse and discard a period (`.`)
fn parse_period<'s>(input: &mut &'s str) -> Result<()> {
    literal(".").map(|_| ()).parse_next(input)
}

#[cfg(test)]
mod test {
    use crate::microns::test::microns_to_mm_string;

    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_parse_sign() {
        let mut input_plus = "+";
        let mut input_minus = "-";
        let mut input_other = "foo";
        assert_eq!(Ok(Sign::Plus), parse_sign(&mut input_plus));
        assert_eq!(Ok(Sign::Minus), parse_sign(&mut input_minus));
        assert!(parse_sign(&mut input_other).is_err());
    }

    #[test]
    fn test_parse_digits_i32() {
        let mut input1 = "456";
        let mut input2 = "-456";
        let mut input3 = "foo";
        assert_eq!(Ok(456i32), parse_digits_i32(&mut input1));
        assert!(parse_digits_i32(&mut input2).is_err());
        assert!(parse_digits_i32(&mut input3).is_err());
    }

    #[test]
    fn test_parse_period() {
        let mut input1 = ".";
        let mut input2 = ",";
        assert_eq!(Ok(()), parse_period(&mut input1));
        assert!(parse_period(&mut input2).is_err());
    }

    #[test]
    fn test_parse_fractional_digits() {
        let mut input1 = "123";
        let mut input2 = "12";
        let mut input3 = "1";
        let mut input4 = "foo";
        let mut input5 = "1234";
        assert_eq!(Ok(123), parse_fractional_digits::<3>(&mut input1));
        assert_eq!(Ok(120), parse_fractional_digits::<3>(&mut input2));
        assert_eq!(Ok(100), parse_fractional_digits::<3>(&mut input3));
        assert!(parse_fractional_digits::<3>(&mut input4).is_err());
        assert!(parse_fractional_digits::<3>(&mut input5).is_err());
    }

    #[test]
    fn test_parse_mm_as_microns_example() {
        let mut input1 = "123";
        let mut input2 = "123.456";
        let mut input3 = "123.45";
        let mut input4 = "+123.456";
        let mut input5 = "-123.456";
        let mut input6 = "-123.0";
        let mut input7 = "foo";
        assert_eq!(Ok(Microns::new(123000)), parse_mm_as_microns(&mut input1));
        assert_eq!(Ok(Microns::new(123456)), parse_mm_as_microns(&mut input2));
        assert_eq!(Ok(Microns::new(123450)), parse_mm_as_microns(&mut input3));
        assert_eq!(Ok(Microns::new(123456)), parse_mm_as_microns(&mut input4));
        assert_eq!(Ok(Microns::new(-123456)), parse_mm_as_microns(&mut input5));
        assert_eq!(Ok(Microns::new(-123000)), parse_mm_as_microns(&mut input6));
        assert!(parse_mm_as_microns(&mut input7).is_err());
    }

    proptest! {
        #[test]
        fn test_parse_mm_as_microns(
            sign in prop_oneof![Just(Sign::Plus), Just(Sign::Minus)].boxed(),
            include_sign: bool,
            int_part in (0..999),
            frac_part in (0..999)
        ) {
            let number = sign.to_i32() * (int_part * 1000 + frac_part);
            let expected = Microns::new(number);

            let frac_str =
                if frac_part == 0 {
                    format!("")
                } else if frac_part < 10 {
                    format!(".00{}", frac_part)
                } else if frac_part < 100 {
                    format!(".0{}", frac_part)
                } else {
                    format!(".{}", frac_part)
                };
            let sign_str =
                match sign {
                    Sign::Plus => if include_sign { "+" } else { "" }
                    Sign::Minus => "-"
                };
            let input = format!("{}{}{}", sign_str, int_part, frac_str);
            let mut input_ref: &str = &input;

            assert_eq!(Ok(expected), parse_mm_as_microns(&mut input_ref));
        }
    }

    proptest! {
        #[test]
        fn microns_round_trip(
            microns_number: i32
        ) {
            let microns = Microns::new(microns_number);
            let microns_str = microns_to_mm_string(&microns);
            let mut input_ref: &str = &microns_str;
            assert_eq!(Ok(microns), parse_mm_as_microns(&mut input_ref));
        }
    }
}
