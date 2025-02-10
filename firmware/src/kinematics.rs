use arduino_hal::{
    delay_us,
    port::{mode::Output, Pin, PinOps},
    prelude::_unwrap_infallible_UnwrapInfallible,
};
use embedded_hal::digital::{OutputPin, PinState};

/// Describes the direction for an axis movement.
#[derive(PartialEq, Clone, Copy)]
enum Direction {
    /// Positive direction is associated with a "high" direction signal.
    Positive,
    /// Negative direction is associated with a "low" direction signal.
    Negative,
}
impl Direction {
    /// Convert a `Direction` to a `PinState`.
    fn to_pin_state(&self) -> PinState {
        match self {
            Direction::Positive => PinState::High,
            Direction::Negative => PinState::Low,
        }
    }
}

/// Type that represents a number of steps.
///
/// The key feature of `Steps` is that it's careful to prevent overflows,
/// so that axes will not get themselves into bad states.
#[derive(PartialEq, Clone, Copy)]
struct Steps(i32);
impl Steps {
    /// Create a new number of steps.
    fn new(steps: i32) -> Self {
        Self(steps)
    }

    /// Zero steps.
    fn zero() -> Self {
        Steps(0)
    }

    /// Increment the value if it's safe to do so without an overflow.
    fn inc(&self) -> Option<Self> {
        self.0.checked_add_unsigned(1).map(Steps)
    }

    /// Decrement the value if it's safe to do so without an overflow.
    fn dec(&mut self) -> Option<Self> {
        self.0.checked_sub_unsigned(1).map(Steps)
    }
}

/// Describes the delays required after sending pulses to a stepper motor.
///
/// In order for the stepper controller to run correctly, some pulse timing
/// constraints are required. These delays are used for synchronous control.
#[derive(Clone, Copy)]
struct PulseDelays {
    delay_pulse_us: u32,
    delay_direction_us: u32,
}
impl PulseDelays {
    /// Returns the default pulse delays for this project.
    fn default() -> Self {
        Self {
            delay_pulse_us: 10,
            delay_direction_us: 5,
        }
    }

    /// Blocks waiting for the amount required after a pulse signal is set
    /// high or low.
    fn pulse_wait(&self) {
        delay_us(self.delay_pulse_us);
    }

    /// Blocks waiting for the amount require before or after a direction
    /// signal is set high or low.
    fn direction_wait(&self) {
        delay_us(self.delay_direction_us);
    }
}

/// `Steppable` is an abstraction point to allow fake axes to be used in tests.
///
/// It implements the most essential function of `BasicAxis`, which is to take
/// a step in one direction or another.
trait Steppable {
    fn step(&mut self, direction: Direction);
}

/// Basic axis.
///
/// A basic axis is associated with a single stepper motor. It has the
/// ability to move the stepper, but no notion of its own position, of zero
/// points, limit switches, or any other safety mechanisms.
///
/// There are two pins for the axis:
/// - `pin_pulse`: Used to send a single pulse that advances the stepper.
/// - `pin_direction`: Used to set the stepping direction.
///
/// The axis also has some delays associated with it, to ensure proper timing
/// of synchronous actions, and it remembers which direction it is pointing.
///
/// # Type Parameters
///
/// - `P`: Pin type to use for pulse.
/// - `D`: Pin type to use for direction.
struct BasicAxis<P, D> {
    delays: PulseDelays,
    pin_pulse: Pin<Output, P>,
    pin_direction: Pin<Output, D>,
    direction: Direction,
}
impl<P, D> BasicAxis<P, D>
where
    P: PinOps,
    D: PinOps,
{
    /// Creates a new unbounded axis.
    fn new(
        delays: PulseDelays,
        pin_pulse: Pin<Output, P>,
        pin_direction: Pin<Output, D>,
    ) -> Self {
        let direction = Direction::Negative;
        let mut ua = Self {
            delays,
            pin_pulse,
            pin_direction,
            direction,
        };

        // Ensure that the pulse pin is low.
        ua.delays.pulse_wait();
        ua.pin_pulse.set_low();
        ua.delays.pulse_wait();

        // Ensure that the direction pin is set correctly.
        ua.force_set_direction(direction);

        ua
    }

    /// Takes a single step in either the positive or negative direction for
    /// the axis.
    ///
    /// If the direction changes, an extra delay is required to set the new
    /// direction. Otherwise, only a pulse is executed.
    ///
    /// # Parameters
    ///
    /// - `direction`: The direction for the step.
    fn do_step(&mut self, direction: Direction) {
        self.set_direction_if_required(direction);
        self.pin_pulse.set_high();
        self.delays.pulse_wait();
        self.pin_pulse.set_low();
        self.delays.pulse_wait();
    }

    /// Sets the direction pin, but only if it has changed.
    ///
    /// Direction settings are only necessary when the direction *changes*.
    /// This method sets the direction if it differs from the current
    /// direction.
    ///
    /// # Parameters
    ///
    /// - `direction`: The direction required after this call.
    fn set_direction_if_required(&mut self, direction: Direction) {
        if direction != self.direction {
            self.force_set_direction(direction);
        }
    }

    /// Force-sets the direction pin.
    ///
    /// Direction settings are only necessary when the direction *changes*.
    /// This method forces the setting, with its associated pause.
    ///
    /// # Parameters
    ///
    /// - `direction`: The direction required after this call.
    fn force_set_direction(&mut self, direction: Direction) {
        self.delays.direction_wait();
        self.pin_direction
            .set_state(direction.to_pin_state())
            .unwrap_infallible();
        self.delays.direction_wait();
        self.direction = direction;
    }
}
impl<P, D> Steppable for BasicAxis<P, D>
where
    P: PinOps,
    D: PinOps,
{
    fn step(&mut self, direction: Direction) {
        self.do_step(direction);
    }
}

/// Axis which knows its own position.
///
/// A `TrackedAxis` keeps track of its position in steps, so that it knows
/// where it is. It tracks position so that it doesn't get into a bad state.
///
/// # Type Parameters
///
/// - `S`: Type of the steppable thing; usually a `BasicAxis`.
struct TrackedAxis<S> {
    steppable: S,
    position: Steps,
}
impl<S> TrackedAxis<S>
where
    S: Steppable,
{
    /// Creates a new `TrackedAxis`.
    fn new(steppable: S) -> Self {
        Self {
            steppable,
            position: Steps::zero(),
        }
    }

    /// Sets the current position of the axis as zero.
    fn set_current_position_zero(&mut self) {
        self.position = Steps::zero();
    }

    /// Returns the current position of the axis, in steps.
    fn get_position(&self) -> Steps {
        return self.position;
    }

    /// Takes a single step in either the positive or negative direction for
    /// the axis.
    ///
    /// The step is only taken if it would NOT overflow the step count in
    /// either direction.
    ///
    /// If the direction changes, an extra delay is required to set the new
    /// direction. Otherwise, only a pulse is executed.
    ///
    /// # Parameters
    ///
    /// - `direction`: The direction for the step.
    ///
    /// # Returns
    ///
    /// - `Some(steps)`: If the step was taken. This returns the current
    ///   position after the step.
    /// - `None`: Otherwise.
    fn step(&mut self, direction: Direction) -> Option<Steps> {
        // Check if the move is possible without overflow.
        let next_position_option = match direction {
            Direction::Positive => self.position.inc(),
            Direction::Negative => self.position.dec(),
        };

        // Take the step if we're allowed.
        if let Some(next_position) = next_position_option {
            self.steppable.step(direction);
            self.position = next_position;
        }

        // Return the result.
        next_position_option
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_steps_max_inc() {
        let steps_max = Steps::new(i32::MAX);
        assert_eq!(None, steps_max.inc());
    }

    #[test]
    fn test_steps_min_dec() {
        let steps_min = Steps::new(i32::MIN);
        assert_eq!(None, steps_min.dec());
    }

    /// Create a simulated basic axis for testing.
    struct SimulatedBasicAxis {
        position: i64,
    }
    impl SimulatedBasicAxis {
        fn new() -> Self {
            Self { position: 0 }
        }
        fn do_step(&mut self, direction: Direction) {
            self.position = match direction {
                Direction::Positive => self
                    .position
                    .checked_add_unsigned(1)
                    .expect("SimulatedBasicAxis should not overflow!"),
                Direction::Negative => self
                    .position
                    .checked_sub_unsigned(1)
                    .expect("SimulatedBasicAxis should not underflow!"),
            };
        }
    }
    impl Steppable for SimulatedBasicAxis {
        fn step(&mut self, direction: Direction) {
            self.do_step(direction);
        }
    }
}
