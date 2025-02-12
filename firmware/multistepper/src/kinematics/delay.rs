use crate::MicroSeconds;

/// Abstraction for synchronous timing delays.
pub trait Delay {
    /// Blocks for the specified number of microseconds before returning.
    ///
    /// # Parameters
    ///
    /// - `microseconds`: Number of microseconds to delay.
    fn delay_us(microseconds: MicroSeconds);
}

#[cfg(test)]
pub mod test {
    use super::*;

    /// A no-delay type for testing.
    pub struct NoDelay;
    impl Delay for NoDelay {
        fn delay_us(_: MicroSeconds) {}
    }
}
