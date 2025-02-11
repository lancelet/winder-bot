/// Abstraction for synchronous timing delays.
pub trait Delay {
    /// Blocks for the specified number of microseconds before returning.
    ///
    /// # Parameters
    ///
    /// - `microseconds`: Number of microseconds to delay.
    fn delay_us(microseconds: u32);
}
