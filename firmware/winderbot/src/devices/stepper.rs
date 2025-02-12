use crate::devices;
use arduino_hal::port::{mode::Output, Pin, PinOps};
use multistepper::{Delay, Direction, MicroSeconds};

/// Stepper motor.
///
/// # Type Parameters
///
/// - `P`: pulse pin
/// - `D`: direction pin
pub struct Stepper<P, D> {
    /// Pin to use for pulses.
    pin_pulse: Pin<Output, P>,
    /// Pin to use for direction indication.
    pin_direction: Pin<Output, D>,
    /// Stores the current direction.
    direction: Direction,
    /// Delay between pulses, in microseconds.
    delay_pulse: MicroSeconds,
    /// Delay between direction changes, in microseconds.
    delay_direction: MicroSeconds,
}

impl<P: PinOps, D: PinOps> Stepper<P, D> {
    /// Creates a new `Stepper`.
    ///
    /// # Parameters
    ///
    /// - `pin_pulse`: Pin to use for pulse signals.
    /// - `pin_direction`: Pin to use for direction signals.
    /// - `delay_pulse`: Delay between pulses, in microseconds.
    /// - `delay_direction`: Delay between direction changes, in
    ///   microseconds.
    pub fn new(
        pin_pulse: Pin<Output, P>,
        pin_direction: Pin<Output, D>,
        delay_pulse: MicroSeconds,
        delay_direction: MicroSeconds,
    ) -> Self {
        let direction = Direction::Negative;
        let mut stepper = Self {
            pin_pulse,
            pin_direction,
            direction,
            delay_pulse,
            delay_direction,
        };

        // Ensure that the direction we think we have is really what's set on
        // the pin.
        stepper.force_set_direction(direction);

        stepper
    }

    /// Execute a step.
    ///
    /// # Parameters
    ///
    /// - `direction`: Desired direction of the step.
    fn do_step(&mut self, direction: Direction) {
        self.set_direction(direction);
        self.pin_pulse.set_high();
        devices::Delay::delay_us(self.delay_pulse);
        self.pin_pulse.set_low();
        devices::Delay::delay_us(self.delay_pulse);
    }

    /// Set the direction, but only if it needs changing.
    ///
    /// # Parameters
    ///
    /// - `direction`: Desired direction of motion.
    fn set_direction(&mut self, direction: Direction) {
        if direction != self.direction {
            self.force_set_direction(direction);
        }
    }

    /// Force set the direction.
    ///
    /// This sets the direction pin even if the direction already matches what
    /// is specified. This is useful on initialization.
    ///
    /// # Parameters
    ///
    /// - `direction`: Desired direction of motion.
    fn force_set_direction(&mut self, direction: Direction) {
        devices::Delay::delay_us(self.delay_direction);
        match direction {
            Direction::Negative => self.pin_direction.set_low(),
            Direction::Positive => self.pin_direction.set_high(),
        }
        self.direction = direction;
        devices::Delay::delay_us(self.delay_direction);
    }
}

/// The `Stepper` interface that allows the stepper to be used with the rest
/// of `multistepper`.
impl<P: PinOps, D: PinOps> multistepper::Stepper for Stepper<P, D> {
    fn step(&mut self, direction: Direction) {
        self.do_step(direction);
    }
}
