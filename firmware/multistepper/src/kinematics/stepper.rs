use crate::Direction;
use crate::Steps;

/// Stepper motor.
///
/// This kind of stepper never fails to take a step (at least in principle).
/// For a stepper that can fail to take a step when it hits overflow limits,
/// see [crate::PositionedStepper].
pub trait Stepper {
    /// Takes a single step in the supplied direction.
    ///
    /// # Parameters
    ///
    /// - `direction`: Direction in which to take the step.
    fn step(&mut self, direction: Direction);
}

/// Checked stepper motor.
///
/// A checked stepper motor has a range, or limit switches.
pub trait CheckedStepper {
    /// Try to take a step.
    ///
    /// # Parameters
    ///
    /// - `direction`: Direction in which to take the step.
    fn try_step(&mut self, direction: Direction) -> Option<Steps>;
}

#[cfg(test)]
pub mod tests {
    use super::super::direction::test::direction;
    use super::*;
    use proptest::prelude::*;
    use std::sync::{Arc, Mutex};

    /// Stepper to use for testing purposes.
    ///
    /// This is just a position counter. It uses `i128`, since that is likely
    /// to be a very much larger range than the step range of any real-world
    /// stepper.
    ///
    /// If the `TestStepper` is cloned then the underlying step count is
    /// shared. This can be useful for coupling simulated devices (eg. limit
    /// switches) to the stepper.
    #[derive(Clone)]
    pub struct TestStepper {
        position: Arc<Mutex<i128>>,
    }
    impl TestStepper {
        /// Creates a new test stepper.
        pub fn new(position: i128) -> Self {
            Self {
                position: Arc::new(Mutex::new(position)),
            }
        }

        /// Returns the position of a test stepper.
        pub fn get_position(&self) -> i128 {
            *self.position.lock().unwrap()
        }

        /// Sets the position of a test stepper.
        pub fn set_position(&mut self, position: i128) {
            *self.position.lock().unwrap() = position;
        }

        /// Executes a step for the test stepper.
        fn do_step(&mut self, direction: Direction) {
            let mut pos = self.position.lock().unwrap();

            *pos = match direction {
                Direction::Negative => pos
                    .checked_sub_unsigned(1)
                    .expect("TestStepper overflowed (+) its position!"),
                Direction::Positive => pos
                    .checked_add_unsigned(1)
                    .expect("TestStepper overflowed (-) its position!"),
            };
        }
    }
    impl Stepper for TestStepper {
        fn step(&mut self, direction: Direction) {
            self.do_step(direction);
        }
    }

    proptest! {
        #[test]
        fn test_step(pos in i32::MIN..i32::MAX, dir in direction()) {
            let mut stepper = TestStepper::new(pos as i128);
            let expected = match dir {
                Direction::Positive => pos as i128 + 1,
                Direction::Negative => pos as i128 - 1
            };

            assert_eq!(pos as i128, stepper.get_position());
            stepper.do_step(dir);
            assert_eq!(expected, stepper.get_position());
        }
    }

    proptest! {
        #[test]
        fn test_set_position(pos in i32::MIN..i32::MAX) {
            let mut stepper = TestStepper::new(0);
            assert_eq!(0, stepper.get_position());
            stepper.set_position(pos as i128);
            assert_eq!(pos as i128, stepper.get_position());
        }
    }
}
