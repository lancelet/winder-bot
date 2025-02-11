#![cfg_attr(not(test), no_std)]

mod kinematics;

pub use kinematics::Delay;
pub use kinematics::Direction;
pub use kinematics::LimitSwitch;
pub use kinematics::LimitSwitchState;
pub use kinematics::LimitedStepper;
pub use kinematics::PositionedStepper;
pub use kinematics::Stepper;
pub use kinematics::Steps;

#[cfg(test)]
pub use kinematics::TestLimitSwitch;
#[cfg(test)]
pub use kinematics::TestStepper;
