/// Time in microseconds;
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub struct MicroSeconds(u32);
impl MicroSeconds {
    /// Creates a new `MicroSeconds`.
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the value as a `u32`.
    pub fn get_value(&self) -> u32 {
        self.0
    }
}
