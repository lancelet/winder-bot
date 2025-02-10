use crate::Direction;
use crate::Stepper;
use crate::Steps;

/// Stepper which tracks its own position.
///
/// A `PositionedStepper` executes its stepping commands by wrapping some
/// underlying [Stepper].
pub struct PositionedStepper<S> {
    stepper: S,
    position: Steps,
}
impl<S: Stepper> PositionedStepper<S> {
    /// Creates a new positioned stepper.
    ///
    /// The new stepper has an initial position of zero.
    ///
    /// # Parameters
    ///
    /// - `stepper`: The underlying stepper to use.
    pub fn new(stepper: S) -> Self {
        Self {
            stepper,
            position: Steps::zero(),
        }
    }

    /// Sets the current position of the stepper to zero.
    ///
    /// This DOES NOT move the stepper. It simply sets the position to zero
    /// whereever the stepper currently is.
    pub fn set_gauge_zero(&mut self) {
        self.set_gauge_position(Steps::zero());
    }

    /// Sets the current position of the stepper to a given value.
    ///
    /// This DOES NOT move the stepper. It simply sets the position to the
    /// supplied value, wherever the stepper currently is.
    ///
    /// # Parameters
    ///
    /// - `position`: the new position for the stepper.
    pub fn set_gauge_position(&mut self, position: Steps) {
        self.position = position;
    }

    /// Returns the current position of the stepper.
    pub fn get_position(&self) -> Steps {
        self.position
    }

    /// Take a step.
    ///
    /// This takes a step with the underlying stepper provided that doing so
    /// would not overflow the step count.
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
        // Compute the new position if possible.
        let next_position_option = match direction {
            Direction::Negative => self.position.dec(),
            Direction::Positive => self.position.inc(),
        };

        // Take the step if it's possible.
        if let Some(next_position) = next_position_option {
            self.stepper.step(direction);
            self.position = next_position;
        }

        next_position_option
    }
}

#[cfg(test)]
mod test {
    use super::super::direction::test::direction;
    use super::*;
    use crate::TestStepper;
    use proptest::collection;
    use proptest::prelude::*;

    #[test]
    fn test_new() {
        let stepper = TestStepper::new(0);
        let pstepper = PositionedStepper::new(stepper);
        assert_eq!(Steps::zero(), pstepper.get_position());
    }

    #[test]
    fn test_set_position() {
        let stepper = TestStepper::new(0);
        let mut pstepper = PositionedStepper::new(stepper);
        pstepper.set_gauge_position(Steps::new(42));

        assert_eq!(42, pstepper.get_position().get_value());
        assert_eq!(0, pstepper.stepper.get_position());
    }

    #[test]
    fn test_set_zero() {
        let stepper = TestStepper::new(0);
        let mut pstepper = PositionedStepper::new(stepper);

        // Take some steps
        for _ in 0..10 {
            pstepper.step(Direction::Positive);
        }

        // Check current status.
        assert_eq!(10, pstepper.get_position().get_value());
        assert_eq!(10, pstepper.stepper.get_position());

        // Zero the position.
        pstepper.set_gauge_zero();

        // Check the zeroed status.
        assert_eq!(0, pstepper.get_position().get_value());
        assert_eq!(10, pstepper.stepper.get_position());
    }

    /// Sanity-test taking a step in both directions.
    #[test]
    fn test_step() {
        let stepper = TestStepper::new(0);
        let mut pstepper = PositionedStepper::new(stepper);
        assert_eq!(0, pstepper.get_position().get_value());

        // Take a plus step.
        let steps_plus = pstepper.step(Direction::Positive);
        assert_eq!(Some(Steps::new(1)), steps_plus);
        assert_eq!(1, pstepper.get_position().get_value());
        assert_eq!(1, pstepper.stepper.get_position());

        // Take a minus step.
        let steps_minus = pstepper.step(Direction::Negative);
        assert_eq!(Some(Steps::new(0)), steps_minus);
        assert_eq!(0, pstepper.get_position().get_value());
        assert_eq!(0, pstepper.stepper.get_position());
    }

    proptest! {
        #[test]
        fn test_multi_steps(
            single_steps in collection::vec(direction(), 1..64)
        ) {
            // Run the sequence of steps.
            let mut pos: i32 = 0;
            let stepper = TestStepper::new(0);
            let mut pstepper = PositionedStepper::new(stepper);
            for dir in single_steps {
                // Manually track the position
                match dir {
                    Direction::Positive => { pos += 1; }
                    Direction::Negative => { pos -= 1; }
                }

                // Take the step
                let step_result = pstepper.step(dir);

                // Check that we end up at the correct position
                assert_eq!(Some(Steps::new(pos)), step_result);
                assert_eq!(pos, pstepper.get_position().get_value());
                assert_eq!(pos as i128, pstepper.stepper.get_position());
            }
        }
    }

    /// Test actions we can take on a positioned stepper.
    #[derive(Debug, Clone)]
    enum Action {
        StepPlus,
        StepMinus,
        Zero,
        SetPosition(Steps),
    }

    /// Generation strategy for actions.
    fn action() -> impl Strategy<Value = Action> {
        use Action::*;
        prop_oneof![
            Just(StepPlus),
            Just(StepMinus),
            Just(Zero),
            any::<i32>().prop_map(|x| SetPosition(Steps::new(x)))
        ]
    }

    proptest! {
        #[test]
        fn test_actions(actions in collection::vec(action(), 1..64)) {
            let stepper = TestStepper::new(0);
            let mut pstepper = PositionedStepper::new(stepper);

            let mut ppos: i32 = 0;
            let mut upos: i128 = 0;
            for a in actions {

                // Perform the action to update the test state.
                use Action::*;
                match a {
                    StepPlus => {
                        ppos += 1;
                        upos += 1;
                    },
                    StepMinus => {
                        ppos -= 1;
                        upos -= 1;
                    },
                    Zero => {
                        ppos = 0;
                    },
                    SetPosition(p) => {
                        ppos = p.get_value()
                    }
                }

                // Perform the action on the positioned stepper.
                match a {
                    StepPlus => { pstepper.step(Direction::Positive); },
                    StepMinus => { pstepper.step(Direction::Negative); },
                    Zero => { pstepper.set_gauge_zero(); },
                    SetPosition(p) => { pstepper.set_gauge_position(p); },
                }

                // Check that the stepper and test state currently match.
                assert_eq!(ppos, pstepper.get_position().get_value());
                assert_eq!(upos, pstepper.stepper.get_position());
            }
        }
    }
}
