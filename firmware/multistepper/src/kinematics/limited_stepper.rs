use crate::Delay;
use crate::Direction;
use crate::LimitSwitch;
use crate::LimitSwitchState;
use crate::PositionedStepper;
use crate::Stepper;
use crate::Steps;

/// Stepper which tracks its position and uses limit switches.
///
/// # Type Parameters
///
/// - `S`: type of the [PositionedStepper].
/// - `L`: type of the [LimitSwitch].
pub struct LimitedStepper<S, L> {
    stepper: PositionedStepper<S>,
    limit_switches: LimitSwitches<L>,
    soft_range: Option<StepRange>,
}
impl<S: Stepper, L: LimitSwitch> LimitedStepper<S, L> {
    /// Creates a new `LimitedStepper`.
    ///
    /// Initially there is no soft range for the `LimitedStepper`. Usually,
    /// establishing the soft range requires procedural zeroing.
    ///
    /// # Parameters
    ///
    /// - `stepper`: Underlying `PositionedStepper` to use for position.
    /// - `limit_switch_pos`: Limit switch at the end of the axis when moving
    ///   in a positive direction.
    /// - `limit_switch_neg`: Limit switch at the end of the axis when moving
    ///   in a negative direction.
    pub fn new(
        stepper: PositionedStepper<S>,
        limit_switch_pos: L,
        limit_switch_neg: L,
    ) -> Self {
        let limit_switches =
            LimitSwitches::new(limit_switch_pos, limit_switch_neg);
        let soft_range = None;

        Self {
            stepper,
            limit_switches,
            soft_range,
        }
    }

    /// Takes a step.
    ///
    /// A step is taken provided the following criteria are met:
    ///
    /// 1. The limit switch in the direction of motion is not in the
    ///    [LimitSwitchState::AtLimit] state.
    /// 2. The soft range (if there is one) includes the step we want to take.
    /// 3. Moving would not overflow the step count.
    ///
    /// # Parameters
    ///
    /// - `direction`: The direction in which to take a step.
    ///
    /// # Returns
    ///
    /// - `Some(steps)`: if the step could successfully be taken. This returns
    ///   the new position of the stepper.
    /// - `None`: if no step could be taken without overflowing limits.
    pub fn step(&mut self, direction: Direction) -> Option<Steps> {
        // Bail if the limit switches do not allow us to move in the specified
        // direction.
        if !self.limit_switches.ok_to_move(direction) {
            return None;
        }

        // Bail if the soft range does not allow us to move in the specified
        // direction.
        if let Some(sr) = self.soft_range.as_ref() {
            if !sr.ok_to_move(self.stepper.get_position(), direction) {
                return None;
            }
        }

        // Both the limit switches and soft range allow the move; try to move
        // the underlying stepper.
        self.stepper.step(direction)
    }

    /// Runs a zeroing procedure.
    ///
    /// Zeroing does the following:
    ///
    /// 1. If the negative limit switch is engaged, moves the axis in the
    ///    positive direction until the negative limit switch is disengaged.
    ///    It moves `soft_safety_margin` steps beyond this. If the positive
    ///    limit switch engages at any time during this period, zeroing is
    ///    failed.
    /// 2. Moves the axis in the negative direction until the negative limit
    ///    switch engages, then moves back `soft_safety_margin` steps.
    /// 3. Sets the lower value of the soft limits to the current steps.
    /// 4. Moves the axis in the positive direction until the positive limit
    ///    switch is engaged, then moves back `soft_safety_margin` steps.
    /// 5. Sets the upper value of the soft limits to the current steps.
    /// 6. Moves the stepper to the middle of its soft range.
    ///
    /// # Parameters
    ///
    /// - `move_delay_us`: An amount to delay in microseconds between each
    ///   stepper move. This controls the speed of zeroing.
    /// - `soft_safety_margin`: A number of steps before the limit switches
    ///   engaged to set the soft limits.
    ///
    /// # Returns
    ///
    /// - `true` if the procedure completed successfully.
    /// - `false` if it failed.
    pub fn run_zeroing<D: Delay>(
        &mut self,
        move_delay_us: u32,
        soft_safety_margin: Steps,
    ) -> bool {
        // Disengage the negative limit switch if it is engaged.
        if !self.zeroing_disengage_negative::<D>(move_delay_us) {
            return false;
        }

        // Engage the negative limit switch.
        if !self.zeroing_engage_negative::<D>(move_delay_us) {
            return false;
        }

        // Zero the underling stepper.
        self.stepper.set_gauge_zero();

        // Back off the negatie limit switch by `soft_safety_margin`.
        if !self
            .zeroing_backoff_negative::<D>(move_delay_us, soft_safety_margin)
        {
            return false;
        }

        // Record the minimum soft-limit.
        let min_steps = self.stepper.get_position();

        // Engage the positive limit switch.
        if !self.zeroing_engage_positive::<D>(move_delay_us) {
            return false;
        }

        // Back off the positive limit switch by `soft_safety_margin`.
        if !self
            .zeroing_backoff_positive::<D>(move_delay_us, soft_safety_margin)
        {
            return false;
        }

        // Record the maximum soft-limit.
        let max_steps = self.stepper.get_position();

        // Store both soft limits.
        self.soft_range = Some(StepRange::new(min_steps, max_steps));

        // Center the axis.
        if !self.zeroing_center::<D>(move_delay_us) {
            return false;
        }

        true
    }

    /// Start of zeroing: disengage the negative limit switch.
    ///
    /// IFF the negative limit switch is engaged, move the axis in the positive
    /// direction until the negative limit switch is disengaged. If the
    /// positive limit switch engages at any time during this procedure, fail.
    fn zeroing_disengage_negative<D: Delay>(
        &mut self,
        move_delay_us: u32,
    ) -> bool {
        while self.limit_switches.negative_end.read_limitswitch_state()
            == LimitSwitchState::AtLimit
        {
            if self.step(Direction::Positive) == None {
                return false;
            }
            D::delay_us(move_delay_us);
        }
        true
    }

    /// Engage the negative limit switch.
    fn zeroing_engage_negative<D: Delay>(
        &mut self,
        move_delay_us: u32,
    ) -> bool {
        while self.limit_switches.negative_end.read_limitswitch_state()
            == LimitSwitchState::NotAtLimit
        {
            if self.step(Direction::Negative) == None {
                return false;
            }
            D::delay_us(move_delay_us);
        }
        true
    }

    /// Back-off the negative limit switch by the specified safety margin.
    fn zeroing_backoff_negative<D: Delay>(
        &mut self,
        move_delay_us: u32,
        soft_safety_margin: Steps,
    ) -> bool {
        let mut step_count = Steps::zero();
        while step_count < soft_safety_margin {
            if self.step(Direction::Positive) == None {
                return false;
            }
            D::delay_us(move_delay_us);
            step_count = step_count.inc().unwrap();
        }
        true
    }

    /// Engage the positive limit switch.
    fn zeroing_engage_positive<D: Delay>(
        &mut self,
        move_delay_us: u32,
    ) -> bool {
        while self.limit_switches.positive_end.read_limitswitch_state()
            == LimitSwitchState::NotAtLimit
        {
            if self.step(Direction::Positive) == None {
                return false;
            }
            D::delay_us(move_delay_us);
        }
        true
    }

    /// Back-off the positive limit switch by the specified safety margin.
    fn zeroing_backoff_positive<D: Delay>(
        &mut self,
        move_delay_us: u32,
        soft_safety_margin: Steps,
    ) -> bool {
        let mut step_count = Steps::zero();
        while step_count < soft_safety_margin {
            if self.step(Direction::Negative) == None {
                return false;
            }
            D::delay_us(move_delay_us);
            step_count = step_count.inc().unwrap();
        }
        true
    }

    /// After zeroing; move to the center of the soft range.
    fn zeroing_center<D: Delay>(&mut self, move_delay_us: u32) -> bool {
        if let Some(range) = self.soft_range.as_ref() {
            let target = Steps::new(
                range.max_steps.get_value() / 2
                    + range.min_steps.get_value() / 2,
            );
            if self.stepper.get_position() > target {
                return false;
            } else {
                while target < self.stepper.get_position() {
                    if self.step(Direction::Negative) == None {
                        return false;
                    }
                    D::delay_us(move_delay_us);
                }
                return true;
            }
        } else {
            return false;
        }
    }
}

/// Represents the allowed (soft-limited) step range.
struct StepRange {
    /// Minimum allowed step value (inclusive).
    min_steps: Steps,
    /// Maximum allowed step value (inclusive).
    max_steps: Steps,
}
impl StepRange {
    /// Creates a new `StepRange`.
    fn new(min_steps: Steps, max_steps: Steps) -> Self {
        debug_assert!(min_steps <= max_steps);
        Self {
            min_steps,
            max_steps,
        }
    }

    /// Checks if we're OK to move in a given direction.
    ///
    /// We can move in a direction if the following criteria are both met:
    ///
    /// 1. The position would not go outside the range:
    ///    `min_steps <= position <= max_steps`.
    /// 2. The step counter would not overflow.
    ///
    /// # Parameters
    ///
    /// - `position`: Current position.
    /// - `direction`: Desired direction of movement.
    ///
    /// # Returns
    ///
    /// - `true`: if we are OK to move.
    /// - `false`: if moving would violate constraints
    fn ok_to_move(&self, position: Steps, direction: Direction) -> bool {
        match direction {
            Direction::Positive => self.ok_to_move_positive(position),
            Direction::Negative => self.ok_to_move_negative(position),
        }
    }

    /// Checks if we're OK to move in a positive direction.
    ///
    /// We can move in a positive direction if the following criteria are
    /// both met:
    ///
    /// 1. The position would not exceed the `max_steps` value, and
    /// 2. The step counter would not overflow.
    ///
    /// # Parameters
    ///
    /// - `position`: Current position.
    ///
    /// # Returns
    ///
    /// - `true`: if we are OK to move.
    /// - `false`: if moving would violate constraints
    fn ok_to_move_positive(&self, position: Steps) -> bool {
        match position.inc() {
            None => false,
            Some(next_position) => next_position <= self.max_steps,
        }
    }

    /// Checks if we're OK to move in a negative direction.
    ///
    /// We can move in a negative direction if the following criteria are
    /// both met:
    ///
    /// 1. The position would not be less than the `min_steps` value, and
    /// 2. The step counter would not overflow.
    ///
    /// # Parameters
    ///
    /// - `position`: Current position.
    ///
    /// # Returns
    ///
    /// - `true`: if we are OK to move.
    /// - `false`: if moving would violate constraints
    fn ok_to_move_negative(&self, position: Steps) -> bool {
        match position.dec() {
            None => false,
            Some(next_position) => next_position >= self.min_steps,
        }
    }
}

/// Container for limit switches.
struct LimitSwitches<L> {
    positive_end: L,
    negative_end: L,
}
impl<L: LimitSwitch> LimitSwitches<L> {
    /// Creates a new pair of limit switches.
    ///
    /// # Parameters
    ///
    /// - `positive_end`: Limit switch which should be engaged at the end of
    ///   the axis if we keep moving in the positive direction.
    /// - `negative_end`: Limit switch which should be engaged at the end of
    ///   the axis if we keep moving in the negative direction.
    fn new(positive_end: L, negative_end: L) -> Self {
        Self {
            positive_end,
            negative_end,
        }
    }

    /// Check if the limit switches indicate we can move in the desired
    /// direction.
    ///
    /// # Parameters
    ///
    /// - `direction`: The desired direction of movement.
    fn ok_to_move(&self, direction: Direction) -> bool {
        match direction {
            Direction::Negative => self.ok_to_move_negative(),
            Direction::Positive => self.ok_to_move_positive(),
        }
    }

    /// Check if the limit switches indicate we can move in the positive
    /// direction.
    ///
    /// We can move in the positive direction if the positive direction
    /// limit switch is not at the limit.
    fn ok_to_move_positive(&self) -> bool {
        let s = self.positive_end.read_limitswitch_state();
        s == LimitSwitchState::NotAtLimit
    }

    /// Check if the limit switches indicate we can move in the negative
    /// direction.
    ///
    /// We can move in the negative direction if the negative direction
    /// limit switch is not at the limit.
    fn ok_to_move_negative(&self) -> bool {
        let s = self.negative_end.read_limitswitch_state();
        s == LimitSwitchState::NotAtLimit
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::TestLimitSwitch;

    #[test]
    fn test_steprange_ok_to_move() {
        let sr = StepRange::new(Steps::new(0), Steps::new(10));

        assert!(sr.ok_to_move(Steps::new(5), Direction::Negative));
        assert!(sr.ok_to_move(Steps::new(5), Direction::Positive));
        assert!(!sr.ok_to_move(Steps::new(0), Direction::Negative));
        assert!(sr.ok_to_move(Steps::new(0), Direction::Positive));
        assert!(sr.ok_to_move(Steps::new(10), Direction::Negative));
        assert!(!sr.ok_to_move(Steps::new(10), Direction::Positive));
    }

    #[test]
    fn test_limitswitches_ok_to_move() {
        let ps = TestLimitSwitch::new(LimitSwitchState::NotAtLimit);
        let ns = TestLimitSwitch::new(LimitSwitchState::NotAtLimit);
        let mut ss = LimitSwitches::new(ps, ns);

        assert!(ss.ok_to_move(Direction::Positive));
        assert!(ss.ok_to_move(Direction::Negative));

        ss.positive_end
            .set_limitswitch_state(LimitSwitchState::AtLimit);
        assert!(!ss.ok_to_move(Direction::Positive));
        assert!(ss.ok_to_move(Direction::Negative));

        ss.positive_end
            .set_limitswitch_state(LimitSwitchState::NotAtLimit);
        ss.negative_end
            .set_limitswitch_state(LimitSwitchState::AtLimit);
        assert!(ss.ok_to_move(Direction::Positive));
        assert!(!ss.ok_to_move(Direction::Negative));

        ss.positive_end
            .set_limitswitch_state(LimitSwitchState::AtLimit);
        ss.negative_end
            .set_limitswitch_state(LimitSwitchState::AtLimit);
        assert!(!ss.ok_to_move(Direction::Positive));
        assert!(!ss.ok_to_move(Direction::Negative));
    }
}
