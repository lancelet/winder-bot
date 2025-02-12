/// Direction for an axis movement.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Direction {
    /// Positive direction is associated with a "high" direction signal.
    Positive,
    /// Negative direction is associated with a "low" direction signal.
    Negative,
}

#[cfg(test)]
pub mod test {
    use super::*;
    use proptest::prelude::*;

    pub fn direction() -> impl Strategy<Value = Direction> {
        prop_oneof![Just(Direction::Positive), Just(Direction::Negative)]
    }
}
