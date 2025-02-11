/// Abstraction for a limit switch.
///
/// A limit switch just has a state; either at the limit, or not at the
/// limit.
pub trait LimitSwitch {
    fn read_limitswitch_state(&self) -> LimitSwitchState;
}

/// State of a limit switch.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LimitSwitchState {
    /// Limit switch is engaged at the limit.
    ///
    /// This means that the device has reached the limit and should not
    /// proceed any further in whichever direction triggered the limit
    /// to be reached.
    AtLimit,
    /// Limit switch is not at the limit.
    ///
    /// This means that the device can still safely proceed in the
    /// direction of this limit switch.
    NotAtLimit,
}

#[cfg(test)]
pub struct TestLimitSwitch {
    state: LimitSwitchState,
}
#[cfg(test)]
impl TestLimitSwitch {
    /// Creates a new test limit switch.
    pub fn new(state: LimitSwitchState) -> Self {
        Self { state }
    }

    /// Sets the state of the test limit switch.
    pub fn set_limitswitch_state(&mut self, state: LimitSwitchState) {
        self.state = state;
    }
}
#[cfg(test)]
impl LimitSwitch for TestLimitSwitch {
    fn read_limitswitch_state(&self) -> LimitSwitchState {
        self.state
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_test_limit_switch() {
        let mut tls = TestLimitSwitch::new(LimitSwitchState::NotAtLimit);
        assert_eq!(LimitSwitchState::NotAtLimit, tls.read_limitswitch_state());
        tls.set_limitswitch_state(LimitSwitchState::AtLimit);
        assert_eq!(LimitSwitchState::AtLimit, tls.read_limitswitch_state());
        tls.set_limitswitch_state(LimitSwitchState::NotAtLimit);
        assert_eq!(LimitSwitchState::NotAtLimit, tls.read_limitswitch_state());
    }
}
