use arduino_hal::port::{
    mode::{Input, PullUp},
    Pin, PinOps,
};

/// Limit switch.
///
/// # Type Parameters
///
/// - `P`: pin
pub struct LimitSwitch<P> {
    pin: Pin<Input<PullUp>, P>,
}
impl<P: PinOps> LimitSwitch<P> {
    /// Creates a new `LimitSwitch`.
    ///
    /// # Parameters
    ///
    /// - `pin`: Pin to use for the limit switch.
    pub fn new(pin: Pin<Input<PullUp>, P>) -> Self {
        Self { pin }
    }

    /// Reads the state of the limit switch.
    fn state(&self) -> multistepper::LimitSwitchState {
        if self.pin.is_high() {
            multistepper::LimitSwitchState::AtLimit
        } else {
            multistepper::LimitSwitchState::NotAtLimit
        }
    }
}

/// The `LimitSwitch` interface that allows the limit switch to be used with
/// the rest of `multistepper`.
impl<P: PinOps> multistepper::LimitSwitch for LimitSwitch<P> {
    fn read_limitswitch_state(&self) -> multistepper::LimitSwitchState {
        self.state()
    }
}
