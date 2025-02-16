use crate::MilliDegrees;
use crate::Steps;

/// Conversions for rotary motion.
///
/// This converts:
/// - Millidegrees to steps.
/// - Steps to millidegrees.
///
/// It also performs delta calculations for movement.
pub struct RotaryConverter {
    steps_per_revolution: i32,
}

impl RotaryConverter {
    pub fn new(steps_per_revolution: i32) -> Self {
        Self {
            steps_per_revolution,
        }
    }

    /// Converts a value in [Steps] to a value in [MilliDegrees].
    pub fn to_millidegrees(&self, steps: Steps) -> Option<MilliDegrees> {
        (steps.get_value() as i64)
            .checked_mul(360000)
            .and_then(|q| q.checked_div(self.steps_per_revolution as i64))
            .and_then(|q| q.try_into().ok())
            .map(MilliDegrees::new)
    }

    /// Converts a value in [MilliDegrees] to a value in [Steps].
    pub fn to_steps(&self, millidegrees: MilliDegrees) -> Option<Steps> {
        (millidegrees.get_value() as i64)
            .checked_mul(self.steps_per_revolution as i64)
            .and_then(|q| q.checked_div(360000))
            .and_then(|q| q.try_into().ok())
            .map(Steps::new)
    }

    /// Computes the minimum number of steps to move the axis from the current
    /// position (in steps) to a target position (in millidegrees).
    ///
    /// This is an absolute move, and always moves the axis the least angular
    /// amount. This means that full rotations must be broken up into multiple
    /// parts (ideally a 360 degree rotation should be split into 4x90 degree
    /// rotations for maximum clarity).
    ///
    /// # Parameters
    ///
    /// - `current`: Current position, in steps.
    /// - `target`: Target position, in millidegrees.
    ///
    /// # Returns
    ///
    /// - The number of steps to move (signed).
    pub fn steps_to_abs(
        &self,
        current: Steps,
        target: MilliDegrees,
    ) -> Option<Steps> {
        self.to_millidegrees(current)
            .map(|mdg| mdg.shortest_angle_to(target))
            .and_then(|q| self.to_steps(q))
    }

    /// Computes the minimum number of steps to move the axis from the current
    /// position (in steps) to a target position (in millidegrees).
    ///
    /// This is a relative move, and always moves the axis the specified
    /// angular amount.
    ///
    /// # Parameters
    ///
    /// - `current`: Current position, in steps.
    /// - `offset`: Offset position, in millidegrees.
    ///
    /// # Returns
    ///
    /// - The number of steps to move (signed).
    pub fn steps_to_rel(
        &self,
        current: Steps,
        offset: MilliDegrees,
    ) -> Option<Steps> {
        self.to_steps(offset)
            .and_then(|q| current.get_value().checked_add(q.get_value()))
            .map(Steps::new)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_roundtrip(mdg_value in -1000000i32..1000000i32) {
            let rc = RotaryConverter::new(6400);

            let mdg = MilliDegrees::new(mdg_value);
            let steps = rc.to_steps(mdg).unwrap();
            let result = rc.to_millidegrees(steps).unwrap();

            // The values may be slightly different due to rounding and step
            // precision.
            let difference = (mdg.get_value() - result.get_value()).abs();
            assert!(difference <= 56);
        }
    }
}
