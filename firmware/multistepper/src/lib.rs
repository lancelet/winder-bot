#![cfg_attr(not(test), no_std)]

mod kinematics;

pub use kinematics::Direction;
pub use kinematics::PositionedStepper;
pub use kinematics::Stepper;
pub use kinematics::Steps;

#[cfg(test)]
pub use kinematics::TestStepper;
