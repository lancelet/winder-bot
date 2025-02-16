use multistepper::{Microns, MilliDegrees};
use ufmt_macros::uDebug;

/// Commands.
#[derive(Copy, Clone, uDebug)]
pub enum Command {
    /// G0: Move.
    Move(Move),
    /// G28: Home all axes.
    Home,
    /// G90: Absolute positioning.
    AbsolutePositioning,
    /// G91: Relative positioning.
    RelativePositioning,
}

///
#[derive(Copy, Clone, uDebug)]
pub struct Move {
    pub x_amount: Microns,
    pub a_amount: MilliDegrees,
}
