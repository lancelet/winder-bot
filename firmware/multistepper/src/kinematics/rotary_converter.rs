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
    pub fn to_millidegrees(&self, steps: Steps) -> MilliDegrees {
        MilliDegrees::new(
            (steps.get_value() as i64 * 360000
                / self.steps_per_revolution as i64) as i32,
        )
    }

    /// Converts a value in [MilliDegrees] to a value in [Steps].
    pub fn to_steps(&self, millidegrees: MilliDegrees) -> Steps {
        Steps::new(
            (millidegrees.get_value() as i64 * self.steps_per_revolution as i64
                / 360000) as i32,
        )
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
    pub fn steps_to_abs(&self, current: Steps, target: MilliDegrees) -> Steps {
        let mdg = self.to_millidegrees(current);
        let delta = mdg.shortest_angle_to(target);
        self.to_steps(delta)
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
    pub fn steps_to_rel(&self, current: Steps, offset: MilliDegrees) -> Steps {
        let offset_steps = self.to_steps(offset);
        Steps::new(current.get_value() + offset_steps.get_value())
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
            let steps = rc.to_steps(mdg);
            let result = rc.to_millidegrees(steps);

            // The values may be slightly different due to rounding, but no
            // greater than 1 unit.
            println!("mdg    = {}", mdg);
            println!("steps  = {}", steps.get_value());
            println!("result = {}", result);

            // The values may be slightly different due to rounding and step
            // precision.
            let difference = (mdg.get_value() - result.get_value()).abs();
            assert!(difference <= 56);
        }
    }
}
