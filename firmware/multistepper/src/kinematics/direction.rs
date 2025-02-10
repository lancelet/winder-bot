/// Describes the direction for an axis movement.
#[derive(PartialEq, Clone, Copy)]
pub enum Direction {
    /// Positive direction is associated with a "high" direction signal.
    Positive,
    /// Negative direction is associated with a "low" direction signal.
    Negative,
}
