use core::ops::{Add, Sub};

/// Underlying type representing the number of microns.
type MicronsRepr = i32;

/// Represents a distance in microns.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct Microns(MicronsRepr);
impl Microns {
    /// Creates a new `Microns`.
    pub fn new(value: MicronsRepr) -> Self {
        Self(value)
    }

    /// Returns the value as a [MicronsRepr].
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

#[cfg(test)]
pub mod test {
    use super::*;

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
