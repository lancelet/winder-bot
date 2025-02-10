use crate::Direction;

/// Stepper represents a stepper motor, which can take steps.
pub trait Stepper {
    /// Takes a single step in the supplied direction.
    ///
    /// # Parameters
    /// - `direction`: Direction in which to take the step.
    fn step(&mut self, direction: Direction);
}

/// Stepper to use for testing purposes.
#[cfg(test)]
pub struct TestStepper {
    position: i128,
}
#[cfg(test)]
impl TestStepper {
    /// Creates a new test stepper.
    pub fn new(position: i128) -> Self {
        Self { position }
    }

    /// Returns the position of a test stepper.
    pub fn get_position(&self) -> i128 {
        self.position
    }

    /// Sets the position of a test stepper.
    pub fn set_position(&mut self, position: i128) {
        self.position = position
    }

    /// Executes a step for the test stepper.
    fn do_step(&mut self, direction: Direction) {
        self.position = match direction {
            Direction::Negative => self
                .position
                .checked_sub_unsigned(1)
                .expect("TestStepper overflowed (+) its position!"),
            Direction::Positive => self
                .position
                .checked_add_unsigned(1)
                .expect("TestStepper overflowed (-) its position!"),
        }
    }
}
#[cfg(test)]
impl Stepper for TestStepper {
    fn step(&mut self, direction: Direction) {
        self.do_step(direction);
    }
}
