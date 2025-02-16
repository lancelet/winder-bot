use core::ops::{Add, Sub};

use ufmt::{uDisplay, uWrite, Formatter};
use ufmt_macros::uDebug;

/// Underlying type representing the number of microns.
type MicronsRepr = i32;

/// Distance in microns.
#[derive(Debug, uDebug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub struct Microns(MicronsRepr);
impl Microns {
    /// Creates a new `Microns`.
    pub fn new(value: MicronsRepr) -> Self {
        Self(value)
    }

    /// Returns the value as an `i32`.
    pub fn get_value(&self) -> MicronsRepr {
        self.0
    }
}

impl Add for Microns {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Microns::new(self.get_value() + rhs.get_value())
    }
}

impl Sub for Microns {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Microns::new(self.get_value() - rhs.get_value())
    }
}

impl uDisplay for Microns {
    fn fmt<W>(&self, f: &mut Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: uWrite + ?Sized,
    {
        udisplay_millis(self.get_value(), f)
    }
}

pub fn udisplay_millis<W>(
    value: i32,
    f: &mut Formatter<W>,
) -> Result<(), W::Error>
where
    W: uWrite + ?Sized,
{
    // Sign character.
    if value >= 0 {
        f.write_char('+')?;
    } else {
        f.write_char('-')?;
    }

    let v = value.abs();
    let int_part = v / 1000;
    let frc_part = v % 1000;

    int_part.fmt(f)?;
    f.write_char('.')?;

    if frc_part == 0 {
        f.write_str("000")?;
    } else {
        if frc_part < 10 {
            f.write_str("00")?;
        } else if frc_part < 100 {
            f.write_char('0')?;
        }
        frc_part.fmt(f)?;
    }

    Ok(())
}

#[cfg(test)]
pub mod test {
    use super::*;
    use core::fmt::Display;
    use core::fmt::Formatter;
    use core::fmt::Result;
    use proptest::prelude::*;

    impl Display for Microns {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            f.write_str(&microns_to_mm_string(self))
        }
    }

    /// Strategy for generating [Microns].
    ///
    /// This generates `Microns` across the entire `i32` range.
    pub fn microns() -> impl Strategy<Value = Microns> {
        any::<i32>().prop_map(Microns::new)
    }

    /// Converts a [Microns] value to a decimal String in mm.
    pub fn microns_to_mm_string(microns: &Microns) -> String {
        let number = microns.get_value();
        let sign_str = if number < 0 { "-" } else { "" };
        let abs_number = number.abs();
        let int_part = abs_number / 1000;
        let frac_part = abs_number % 1000;

        let frac_str = if frac_part == 0 {
            format!("")
        } else if frac_part < 10 {
            format!(".00{}", frac_part)
        } else if frac_part < 100 {
            format!(".0{}", frac_part)
        } else {
            format!(".{}", frac_part)
        };

        format!("{}{}{}", sign_str, int_part, frac_str)
    }
}
