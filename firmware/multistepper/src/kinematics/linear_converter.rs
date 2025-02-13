use crate::Microns;
use crate::Steps;

/// Conversions for linear motion.
///
/// This converts:
/// - Microns to steps.
/// - Steps to microns.
pub struct LinearConverter {
    steps_per_revolution: i32,
    microns_per_revolution: i32,
}

impl LinearConverter {
    /// Creates a new linear converter.
    pub fn new(steps_per_revolution: i32, microns_per_revolution: i32) -> Self {
        Self {
            steps_per_revolution,
            microns_per_revolution,
        }
    }

    /// Converts a value in [Steps] to a value in [Microns].
    pub fn to_microns(&self, steps: Steps) -> Microns {
        Microns::new(
            (steps.get_value() as i64 * self.microns_per_revolution as i64
                / self.steps_per_revolution as i64) as i32,
        )
    }

    /// Converts a value in [Microns] to a value in [Steps].
    pub fn to_steps(&self, microns: Microns) -> Steps {
        Steps::new(
            (microns.get_value() as i64 * self.steps_per_revolution as i64
                / self.microns_per_revolution as i64) as i32,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_roundtrip(microns_value in -1000000i32..1000000i32) {
            let lc = LinearConverter::new(6400, 5000);

            let microns = Microns::new(microns_value);
            let steps = lc.to_steps(microns);
            let result = lc.to_microns(steps);

            // The values may be slightly different, but no greater than 1 unit.
            let difference = (microns.get_value() - result.get_value()).abs();
            assert!(difference <= 1);
        }
    }
}
