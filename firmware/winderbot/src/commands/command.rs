use ufmt_macros::uDebug;

/// Commands.
#[derive(Copy, Clone, uDebug)]
pub enum Command {
    /// G28: Home all axes
    Home,
}
