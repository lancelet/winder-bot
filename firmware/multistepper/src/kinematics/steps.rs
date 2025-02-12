/// Underlying type representing the number of steps.
type StepRepr = i32;

/// Number of steps.
///
/// `Steps` is careful to prevent overflows, so that it can be used safely to
/// track axis positions.
#[derive(Debug, PartialEq, Clone, Copy, PartialOrd)]
pub struct Steps(StepRepr);
impl Steps {
    /// Create a new number of steps.
    pub fn new(steps: StepRepr) -> Self {
        Self(steps)
    }

    /// Zero steps.
    pub fn zero() -> Self {
        Steps(0)
    }

    /// Returns the value represented by `Steps`.
    pub fn get_value(&self) -> StepRepr {
        self.0
    }

    /// Increment the value if it's safe to do so without an overflow.
    pub fn inc(&self) -> Option<Self> {
        self.0.checked_add_unsigned(1).map(Steps)
    }

    /// Decrement the value if it's safe to do so without an overflow.
    pub fn dec(&self) -> Option<Self> {
        self.0.checked_sub_unsigned(1).map(Steps)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_zero() {
        assert_eq!(0, Steps::zero().get_value());
        assert_eq!(Steps::new(0), Steps::zero());
    }

    #[test]
    fn test_steps_max_inc() {
        assert_eq!(None, Steps::new(StepRepr::MAX).inc());
    }

    #[test]
    fn test_steps_min_dec() {
        assert_eq!(None, Steps::new(StepRepr::MIN).dec());
    }

    proptest! {
        #[test]
        fn test_new_get(value: StepRepr) {
            assert_eq!(value, Steps::new(value).get_value());
        }
    }

    proptest! {
        #[test]
        fn test_inc_dec(value in ((StepRepr::MIN + 1)..(StepRepr::MAX - 1))) {
            let s = Steps::new(value);
            assert_eq!(value + 1, s.inc().unwrap().get_value());
            assert_eq!(value - 1, s.dec().unwrap().get_value());
            assert_eq!(s, s.inc().unwrap().dec().unwrap());
        }
    }
}
