use core::ops::{Add, Sub};

/// Underlying type representing the number of microns.
type MilliDegreesRepr = i32;

/// Angle in millidegrees.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub struct MilliDegrees(MilliDegreesRepr);
impl MilliDegrees {
    /// Creates a new `Microns`.
    pub fn new(value: MilliDegreesRepr) -> Self {
        Self(value)
    }

    /// Returns the value as an `i32`.
    pub fn get_value(&self) -> MilliDegreesRepr {
        self.0
    }
}

impl Add for MilliDegrees {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        MilliDegrees::new(self.get_value() + rhs.get_value())
    }
}

impl Sub for MilliDegrees {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        MilliDegrees::new(self.get_value() - rhs.get_value())
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use core::fmt::Display;
    use core::fmt::Formatter;
    use core::fmt::Result;
    use proptest::prelude::*;

    impl Display for MilliDegrees {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            f.write_str(&millidegrees_to_degrees_string(self))
        }
    }

    /// Strategy for generating [MilliDegrees].
    ///
    /// This generates `MilliDegrees` across the entire `i32` range.
    pub fn millidegrees() -> impl Strategy<Value = MilliDegrees> {
        any::<i32>().prop_map(MilliDegrees::new)
    }

    /// Converts a [MilliDegrees] value to a decimal String in degrees.
    pub fn millidegrees_to_degrees_string(mdg: &MilliDegrees) -> String {
        let number = mdg.get_value();
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
