use arduino_hal::delay_us;
use embedded_hal::digital::PinState;

use crate::gitm::GhostInTheMachine;

pub struct Machine {
    gitm: GhostInTheMachine,
    move_mode: MoveMode,
    move_delay_us: u32,
    x_pos: u32,
    a_pos: u32,
    x_limit: u32,
}
impl Machine {
    /// Number of steps to use as an "electronic addition" to the limit
    /// switches along X.
    const X_EDGE_SAFETY_STEPS: u32 = 3200;
    /// mm per revolution for x-axis lead screw.
    const X_MM_PER_REV: u32 = 5;
    /// Steps per revolution for x-axis.
    const X_STEPS_PER_REV: u32 = 6400;
    /// Steps per revolution for a-axis.
    const A_STEPS_PER_REV: u32 = 6400;

    /// Return a new machine.
    ///
    /// This zeroes the machine (on startup) so that we know where we are.
    pub fn new() -> Machine {
        let mut gitm = GhostInTheMachine::new();
        let move_mode = MoveMode::Absolute;
        let move_delay_us = 100;
        let count = gitm.zero();
        let x_pos = (count / 2) - Self::X_EDGE_SAFETY_STEPS;
        let a_pos = 0;
        let x_limit = count - 2 * Self::X_EDGE_SAFETY_STEPS;

        Machine {
            gitm,
            move_mode,
            move_delay_us,
            x_pos,
            a_pos,
            x_limit,
        }
    }

    /// Set the move mode (absolute or relative moves).
    pub fn set_move_mode(&mut self, move_mode: MoveMode) {
        self.move_mode = move_mode;
    }

    /// Perform a move.
    pub fn move_millis(&mut self, x_microns: i32, a_millidegrees: i32) {
        match self.move_mode {
            MoveMode::Relative => {
                self.move_rel_millis(x_microns, a_millidegrees)
            }
            MoveMode::Absolute => {
                self.move_abs_millis(x_microns, a_millidegrees)
            }
        }
    }

    /// Move an absolute number of microns and milli-degrees along both X and
    /// A at the same time.
    fn move_abs_millis(&mut self, x_microns: i32, a_millidegrees: i32) {
        let mut x_target = self.x_microns_to_steps(x_microns);
        let a_target = self.a_millidegrees_to_steps(a_millidegrees);

        if x_target < 0 {
            x_target = 0;
        }
        if x_target > self.x_limit as i32 {
            x_target = self.x_limit as i32;
        }

        let dx = x_target - self.x_pos as i32;
        let da = a_target - self.a_pos as i32;

        self.move_rel_steps(dx, da);
    }

    /// Move a relative number of microns and milli-degrees along both X and
    /// A at the same time.
    fn move_rel_millis(&mut self, dx_microns: i32, da_millidegrees: i32) {
        let dx_steps = self.x_microns_to_steps(dx_microns);
        let da_steps = self.a_millidegrees_to_steps(da_millidegrees);
        self.move_rel_steps(dx_steps, da_steps);
    }

    /// Move a relative number of steps along both X and A at the same time.
    fn move_rel_steps(&mut self, dx: i32, da: i32) {
        if dx == 0 {
            self.move_rel_a_only(da);
        } else {
            let x_dir = if dx >= 0 { XDir::Right } else { XDir::Left };
            let a_dir = if da >= 0 { ADir::Pos } else { ADir::Neg };

            // For Bresenham:
            // - x is x
            // - a is y
            let mut d = 2 * da - dx;
            for _ in 0..dx.abs() {
                self.step_x(x_dir);
                delay_us(self.move_delay_us);
                if d > 0 {
                    self.step_a(a_dir);
                    delay_us(self.move_delay_us);
                    d -= 2 * dx;
                }
                d += 2 * da;
            }
        }
    }

    /// Move a relative number of steps along A only.
    fn move_rel_a_only(&mut self, da: i32) {
        let a_dir = if da >= 0 { ADir::Pos } else { ADir::Neg };

        for _ in 0..da.abs() {
            self.step_a(a_dir);
            delay_us(self.move_delay_us);
        }
    }

    /// Take a step along the A axis.
    ///
    /// There are no limit switches governing A-axis motion.
    ///
    /// # Parameters
    ///
    /// - `a_dir`: Direction in which to take a step.
    fn step_a(&mut self, a_dir: ADir) {
        match a_dir {
            ADir::Pos => {
                self.gitm.step_a(PinState::High);
                self.a_pos += 1;
            }
            ADir::Neg => {
                self.gitm.step_a(PinState::Low);
                self.a_pos -= 1;
            }
        }
    }

    /// Take a step along the X axis.
    ///
    /// This motion is protected by both soft limits and limit switches.
    ///
    /// # Parameters
    ///
    /// - `x_dir`: Direction in which to take a step.
    ///
    /// # Returns
    /// `true` if the step could be taken; `false` otherwise.
    fn step_x(&mut self, x_dir: XDir) -> bool {
        match x_dir {
            XDir::Left => {
                if self.x_pos > 0 {
                    self.gitm.step_x(PinState::High);
                    self.x_pos -= 1;
                    true
                } else {
                    false
                }
            }
            XDir::Right => {
                if self.x_pos < self.x_limit - 1 {
                    self.gitm.step_x(PinState::Low);
                    self.x_pos += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn x_microns_to_steps(&self, x_microns: i32) -> i32 {
        let dx = x_microns.abs() as u32;
        let dsteps = dx * Self::X_STEPS_PER_REV / Self::X_MM_PER_REV / 1000;
        (dsteps as i32) * x_microns.signum()
    }

    fn a_millidegrees_to_steps(&self, a_millidegrees: i32) -> i32 {
        let da = a_millidegrees.abs() as u32;
        let dsteps = da * Self::A_STEPS_PER_REV / 360 / 1000;
        (dsteps as i32) * a_millidegrees.signum()
    }
}

#[derive(Copy, Clone)]
pub enum ADir {
    Pos,
    Neg,
}

#[derive(Copy, Clone)]
pub enum XDir {
    Left,
    Right,
}

#[derive(Copy, Clone)]
pub enum MoveMode {
    Absolute,
    Relative,
}
