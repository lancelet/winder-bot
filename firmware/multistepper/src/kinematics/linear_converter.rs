use crate::Microns;
use crate::Steps;

/// Conversions for linear motion.
///
/// This converts:
/// - Microns to steps.
/// - Steps to microns.
///
/// It also performs delta calculations for movement.
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
    pub fn to_microns(&self, steps: Steps) -> Option<Microns> {
        (steps.get_value() as i64)
            .checked_mul(self.microns_per_revolution as i64)
            .and_then(|q| q.checked_div(self.steps_per_revolution as i64))
            .and_then(|q| q.try_into().ok())
            .map(Microns::new)
    }

    /// Converts a value in [Microns] to a value in [Steps].
    pub fn to_steps(&self, microns: Microns) -> Option<Steps> {
        (microns.get_value() as i64)
            .checked_mul(self.steps_per_revolution as i64)
            .and_then(|q| q.checked_div(self.microns_per_revolution as i64))
            .and_then(|q| q.try_into().ok())
            .map(Steps::new)
    }

    /// Computes the number of steps to move the axis to get from the current
    /// position (in steps) to a target position (in microns).
    ///
    /// # Parameters
    ///
    /// - `current`: Current position, in steps.
    /// - `target`: Target position, in microns.
    ///
    /// # Returns
    ///
    /// - The number of steps to move (signed).
    pub fn steps_to(&self, current: Steps, target: Microns) -> Option<Steps> {
        target
            .get_value()
            .checked_sub(self.to_microns(current)?.get_value())
            .and_then(|q| self.to_steps(Microns::new(q)))
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
            let steps = lc.to_steps(microns).unwrap();
            let result = lc.to_microns(steps).unwrap();

            // The values may be slightly different due to rounding, but no
            // greater than 1 unit.
            let difference = (microns.get_value() - result.get_value()).abs();
            assert!(difference <= 1);
        }
    }
}
