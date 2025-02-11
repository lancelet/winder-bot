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

    /// Returns the current position of the stepper.
    fn get_position(&self) -> Steps {
        self.stepper.get_position()
    }

    /// Returns the state of the negative-end limit switch.
    fn negative_limit_state(&self) -> LimitSwitchState {
        self.limit_switches.negative_end.read_limitswitch_state()
    }

    /// Returns the state of the positive-end limit switch.
    fn positive_limit_state(&self) -> LimitSwitchState {
        self.limit_switches.positive_end.read_limitswitch_state()
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
    /// - `None`: if the zeroing failed.
    /// - `Some(steps)`: if the zeroing succeeded, where `steps` is the
    ///   current position.
    pub fn run_zeroing<D: Delay>(
        &mut self,
        move_delay_us: u32,
        soft_safety_margin: Steps,
    ) -> Option<Steps> {
        // We cannot have negative safety margin steps.
        debug_assert!(soft_safety_margin >= Steps::zero());

        // Run all the side-effects for zeroing.
        self.zeroing_disengage_negative::<D>(move_delay_us)?;
        self.zeroing_engage_negative::<D>(move_delay_us)?;
        self.stepper.set_gauge_zero();
        self.zeroing_backoff_negative::<D>(move_delay_us, soft_safety_margin)?;
        let min_steps = self.stepper.get_position();
        self.zeroing_engage_positive::<D>(move_delay_us)?;
        self.zeroing_backoff_positive::<D>(move_delay_us, soft_safety_margin)?;
        let max_steps = self.stepper.get_position();
        if min_steps >= max_steps {
            return None;
        }
        self.soft_range = Some(StepRange::new(min_steps, max_steps));
        self.zeroing_center::<D>(move_delay_us)?;

        // Return the position.
        Some(self.get_position())
    }

    /// Start of zeroing: disengage the negative limit switch.
    ///
    /// IFF the negative limit switch is engaged, move the axis in the positive
    /// direction until the negative limit switch is disengaged. If the
    /// positive limit switch engages at any time during this procedure, fail.
    fn zeroing_disengage_negative<D: Delay>(
        &mut self,
        move_delay_us: u32,
    ) -> Option<()> {
        use LimitSwitchState::AtLimit;
        while self.negative_limit_state() == AtLimit {
            self.step(Direction::Positive)?;
            D::delay_us(move_delay_us);
        }
        Some(())
    }

    /// Engage the negative limit switch.
    fn zeroing_engage_negative<D: Delay>(
        &mut self,
        move_delay_us: u32,
    ) -> Option<()> {
        use LimitSwitchState::NotAtLimit;
        while self.negative_limit_state() == NotAtLimit {
            self.step(Direction::Negative)?;
            D::delay_us(move_delay_us);
        }
        self.step(Direction::Positive)?;
        Some(())
    }

    /// Back-off the negative limit switch by the specified safety margin.
    fn zeroing_backoff_negative<D: Delay>(
        &mut self,
        move_delay_us: u32,
        soft_safety_margin: Steps,
    ) -> Option<()> {
        let mut step_count = Steps::zero();
        while step_count < soft_safety_margin {
            self.step(Direction::Positive)?;
            D::delay_us(move_delay_us);
            step_count = step_count.inc().unwrap();
        }
        Some(())
    }

    /// Engage the positive limit switch.
    fn zeroing_engage_positive<D: Delay>(
        &mut self,
        move_delay_us: u32,
    ) -> Option<()> {
        use LimitSwitchState::NotAtLimit;
        while self.positive_limit_state() == NotAtLimit {
            self.step(Direction::Positive)?;
            D::delay_us(move_delay_us);
        }
        self.step(Direction::Negative)?;
        Some(())
    }

    /// Back-off the positive limit switch by the specified safety margin.
    fn zeroing_backoff_positive<D: Delay>(
        &mut self,
        move_delay_us: u32,
        soft_safety_margin: Steps,
    ) -> Option<()> {
        let mut step_count = Steps::zero();
        while step_count < soft_safety_margin {
            self.step(Direction::Negative)?;
            D::delay_us(move_delay_us);
            step_count = step_count.inc().unwrap();
        }
        Some(())
    }

    /// After zeroing; move to the center of the soft range.
    fn zeroing_center<D: Delay>(&mut self, move_delay_us: u32) -> Option<()> {
        let range = self.soft_range?;
        let target = range.half();
        while target < self.get_position() {
            self.step(Direction::Negative)?;
            D::delay_us(move_delay_us);
        }
        Some(())
    }
}

/// Represents the allowed (soft-limited) step range.
#[derive(Copy, Clone)]
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

    /// Returns the point half-way along the range from `min_steps` to
    /// `max_steps`.
    fn half(&self) -> Steps {
        let s_min = self.min_steps.get_value();
        let s_max = self.max_steps.get_value();
        let s_half = s_min + (s_max - s_min) / 2;
        Steps::new(s_half)
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
    use crate::kinematics::{
        delay::test::NoDelay, limit_switch::test::TestLimitSwitch,
        stepper::tests::TestStepper,
    };
    use proptest::prelude::*;

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
        let mut ps = TestLimitSwitch::new(LimitSwitchState::NotAtLimit);
        let mut ns = TestLimitSwitch::new(LimitSwitchState::NotAtLimit);
        let ss = LimitSwitches::new(ps.clone(), ns.clone());

        assert!(ss.ok_to_move(Direction::Positive));
        assert!(ss.ok_to_move(Direction::Negative));

        ps.set_limitswitch_state(LimitSwitchState::AtLimit);
        assert!(!ss.ok_to_move(Direction::Positive));
        assert!(ss.ok_to_move(Direction::Negative));

        ps.set_limitswitch_state(LimitSwitchState::NotAtLimit);
        ns.set_limitswitch_state(LimitSwitchState::AtLimit);
        assert!(ss.ok_to_move(Direction::Positive));
        assert!(!ss.ok_to_move(Direction::Negative));

        ps.set_limitswitch_state(LimitSwitchState::AtLimit);
        ns.set_limitswitch_state(LimitSwitchState::AtLimit);
        assert!(!ss.ok_to_move(Direction::Positive));
        assert!(!ss.ok_to_move(Direction::Negative));
    }

    /// Provides a test stepper with coupled simulated limit switches.
    pub struct TestStepperWithLimitSwitches {
        test_stepper: TestStepper,
        negative_limit: Steps,
        positive_limit: Steps,
    }
    impl TestStepperWithLimitSwitches {
        /// Creates a new test stepper.
        ///
        /// # Parameters
        ///
        /// - `position`: Starting position.
        /// - `negative_limit`: Limit of the negative limit switch.
        /// - `positive_limit`: Limit of the positive limit switch.
        pub fn new(
            position: Steps,
            negative_limit: Steps,
            positive_limit: Steps,
        ) -> Self {
            assert!(negative_limit <= positive_limit);
            let test_stepper = TestStepper::new(position.get_value() as i128);
            Self {
                test_stepper,
                negative_limit,
                positive_limit,
            }
        }

        /// Returns a new [LimitedStepper] from the test components.
        pub fn limited_stepper(
            &self,
        ) -> LimitedStepper<TestStepper, TestStepperLimitSwitch> {
            LimitedStepper::new(
                PositionedStepper::new(self.stepper()),
                self.positive_limit_switch(),
                self.negative_limit_switch(),
            )
        }

        /// Returns the test stepper.
        ///
        /// This clones the test stepper, so that the stepper returned here
        /// shares its steps with the stepper we still own.
        pub fn stepper(&self) -> TestStepper {
            self.test_stepper.clone()
        }

        /// Returns the negative limit switch.
        ///
        /// The state of this limit switch is coupled to the test stepper.
        pub fn negative_limit_switch(&self) -> TestStepperLimitSwitch {
            TestStepperLimitSwitch::new(
                Direction::Negative,
                self.negative_limit,
                self.test_stepper.clone(),
            )
        }

        /// Returns the positive limit switch.
        ///
        /// The state of this limit switch is coupled to the test stepper.
        pub fn positive_limit_switch(&self) -> TestStepperLimitSwitch {
            TestStepperLimitSwitch::new(
                Direction::Positive,
                self.positive_limit,
                self.test_stepper.clone(),
            )
        }
    }

    /// Limit switch for the test stepper.
    pub struct TestStepperLimitSwitch {
        direction: Direction,
        limit: Steps,
        stepper: TestStepper,
    }
    impl TestStepperLimitSwitch {
        fn new(
            direction: Direction,
            limit: Steps,
            stepper: TestStepper,
        ) -> Self {
            Self {
                direction,
                limit,
                stepper,
            }
        }
    }
    impl LimitSwitch for TestStepperLimitSwitch {
        fn read_limitswitch_state(&self) -> LimitSwitchState {
            use LimitSwitchState::*;
            let pos = Steps::new(self.stepper.get_position() as i32);
            match self.direction {
                Direction::Negative => {
                    if pos < self.limit {
                        AtLimit
                    } else {
                        NotAtLimit
                    }
                }
                Direction::Positive => {
                    if pos > self.limit {
                        AtLimit
                    } else {
                        NotAtLimit
                    }
                }
            }
        }
    }

    #[test]
    fn test_run_zeroing_happy_path_example() {
        // Set up the limited stepper.
        let ts = TestStepperWithLimitSwitches::new(
            Steps::new(30),
            Steps::new(0),
            Steps::new(100),
        );
        let mut lstepper = ts.limited_stepper();

        // Run zeroing.
        let result = lstepper.run_zeroing::<NoDelay>(50, Steps::new(10));

        // Check outcome.
        assert_eq!(10, lstepper.soft_range.unwrap().min_steps.get_value());
        assert_eq!(90, lstepper.soft_range.unwrap().max_steps.get_value());
        assert_eq!(50, lstepper.get_position().get_value());
        assert_eq!(50, ts.stepper().get_position());
        assert_eq!(Some(Steps::new(50)), result);
    }

    proptest! {
       #[test]
        fn test_zeroing(
            negative_limit in -32..32i32,
            (limit_range, position) in (1..64i32)
              .prop_flat_map(|range| (Just(range), 0..range)),
            soft_safety_margin in 0..32i32,
        ) {
            // Establish whether zeroing should succeed.
            let should_succeed = 2 * soft_safety_margin < limit_range;

            // Create the test stepper.
            let position = Steps::new(position);
            let positive_limit = Steps::new(negative_limit + limit_range);
            let negative_limit = Steps::new(negative_limit);
            let ts = TestStepperWithLimitSwitches::new(
                position, negative_limit, positive_limit
            );
            let mut lstepper = ts.limited_stepper();

            let soft_safety_margin = Steps::new(soft_safety_margin);

            // Try zeroing.
            let result = lstepper.run_zeroing::<NoDelay>(50, soft_safety_margin);

            // Check outcome.
            if should_succeed {
                assert!(result.is_some());
                let soft_range = lstepper.soft_range.unwrap();
                let expected_soft_min = soft_safety_margin.get_value();
                let expected_soft_max =
                    limit_range - soft_safety_margin.get_value();
                let expected_steps = limit_range / 2;
                assert_eq!(expected_soft_min, soft_range.min_steps.get_value());
                assert_eq!(expected_soft_max, soft_range.max_steps.get_value());
                assert_eq!(
                    limit_range - 2 * soft_safety_margin.get_value(),
                    soft_range.max_steps.get_value() -
                    soft_range.min_steps.get_value()
                );
                assert_eq!(expected_steps, result.unwrap().get_value());
            } else {
                assert!(result.is_none());
            }
        }
    }
}
