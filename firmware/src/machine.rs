use arduino_hal::{
    delay_us,
    port::{
        mode::{Input, Output, PullUp},
        Pin, D10, D11, D12, D13, D8, D9,
    },
    Peripherals, Pins,
};
use embedded_hal::digital::{OutputPin, PinState};
use ufmt_macros::uDebug;

/// Delay when changing direction.
const DELAY_DIREC: u32 = 10; // min: 5us
/// Delay for an ordinary stepper pulse.
const DELAY_PULSE: u32 = 6; // min: 2.5us
/// mm per revolution for x-axis lead screw.
const X_MM_PER_REV: u32 = 5;
/// Steps per revolution for x-axis.
const X_STEPS_PER_REV: u32 = 6400;
/// Steps per revolution for y-axis.
const Y_STEPS_PER_REV: u32 = 6400;

pub struct Machine {
    pin_x_pulse: Pin<Output, D8>,
    pin_x_direc: Pin<Output, D9>,
    pin_a_pulse: Pin<Output, D10>,
    pin_a_direc: Pin<Output, D11>,
    pin_estop_l: Pin<Input<PullUp>, D13>,
    pin_estop_r: Pin<Input<PullUp>, D12>,

    x_limit: Option<u32>,

    x_pos: Option<u32>,
    x_dir: XDir,
    a_pos: Option<u32>,
    a_dir: ADir,
}
impl Machine {
    pub fn new() -> Machine {
        let peripherals: Peripherals =
            unsafe { arduino_hal::Peripherals::steal() };
        let pins: Pins = arduino_hal::pins!(peripherals);

        let mut machine = Machine {
            pin_x_pulse: pins.d8.into_output(),
            pin_x_direc: pins.d9.into_output(),
            pin_a_pulse: pins.d10.into_output(),
            pin_a_direc: pins.d11.into_output(),
            pin_estop_l: pins.d13.into_pull_up_input(),
            pin_estop_r: pins.d12.into_pull_up_input(),
            x_limit: None,
            x_pos: None,
            x_dir: XDir::Left,
            a_pos: None,
            a_dir: ADir::Plus,
        };
        machine.force_set_x_dir(machine.x_dir);

        machine
    }

    /// Zero the x-axis, setting the limits and positions.
    ///
    /// Returns the number of steps along the z-axis from zero to maximum.
    pub fn zero_x(&mut self) -> u32 {
        let move_step_delay: u32 = 50;

        // Move the x-axis to the left limit switch.
        while self.left_limit_instantaneous() == LimitSwitchState::Unpressed {
            self.step_x(XDir::Left);
            delay_us(move_step_delay);
        }
        self.x_pos = Some(0);

        // Move the x-axis to the right limit switch.
        while self.right_limit_instantaneous() == LimitSwitchState::Unpressed {
            self.step_x(XDir::Right);
            delay_us(move_step_delay);
        }
        self.x_limit = Some(self.x_pos.unwrap());

        // Move back to half way.
        let half = self.x_limit.unwrap() / 2;
        while self.x_pos.unwrap() > half {
            self.step_x(XDir::Left);
            delay_us(move_step_delay);
        }

        return self.x_limit.unwrap();
    }

    pub fn relative_x_um(&mut self, microns: i32) -> Result<(), Error> {
        let abs_microns = microns.abs() as u32;
        let opt_steps = abs_microns
            .checked_mul(X_STEPS_PER_REV)
            .map(|x| x / (X_MM_PER_REV * 1000));
        let steps = match opt_steps {
            None => return Err(Error::Overflow),
            Some(x) => x,
        };

        let dir = if microns > 0 { XDir::Right } else { XDir::Left };

        self.relative_x_steps(dir, steps)
    }

    fn relative_x_steps(&mut self, dir: XDir, steps: u32) -> Result<(), Error> {
        let target_steps = match dir {
            XDir::Left => self.x_pos()?.checked_sub(steps).unwrap_or(0),
            XDir::Right => {
                let pre_limit =
                    self.x_pos()?.checked_add(steps).unwrap_or(self.x_limit()?);
                u32::min(self.x_limit()?, pre_limit)
            }
        };

        self.absolute_x_steps(target_steps)
    }

    fn absolute_x_steps(&mut self, steps: u32) -> Result<(), Error> {
        let mut target = steps;
        if target > self.x_limit()? {
            target = self.x_limit()?;
        }

        let dir = if target < self.x_pos()? {
            XDir::Left
        } else {
            XDir::Right
        };

        let move_step_delay: u32 = 400;
        while target != self.x_pos()? {
            self.safe_step_x(dir)?;
            delay_us(move_step_delay);
        }
        Ok(())
    }

    fn x_pos(&self) -> Result<u32, Error> {
        match self.x_pos {
            Some(x) => Ok(x),
            None => Err(Error::NotZeroed),
        }
    }

    fn x_limit(&self) -> Result<u32, Error> {
        match self.x_limit {
            Some(x) => Ok(x),
            None => Err(Error::NotZeroed),
        }
    }

    fn safe_step_x(&mut self, dir: XDir) -> Result<(), Error> {
        let xpos = self.x_pos()?;
        let safety = 0;
        match dir {
            XDir::Left => {
                if xpos > safety {
                    self.step_x(dir)
                }
            }
            XDir::Right => {
                if xpos < self.x_limit()? - safety {
                    self.step_x(dir)
                }
            }
        }
        Ok(())
    }

    /// Force a step in X.
    ///
    /// NOTE: This does NOT use limit switches.
    fn step_x(&mut self, dir: XDir) {
        self.set_x_dir(dir);
        self.pin_x_pulse.set_high();
        delay_us(DELAY_PULSE);
        self.pin_x_pulse.set_low();
        delay_us(DELAY_PULSE);

        if let Some(xpos) = self.x_pos {
            match dir {
                XDir::Left => self.x_pos = Some(xpos - 1),
                XDir::Right => self.x_pos = Some(xpos + 1),
            }
        }
    }

    /// Set the x direction; toggling if necessary.
    fn set_x_dir(&mut self, dir: XDir) {
        if dir != self.x_dir {
            self.force_set_x_dir(dir);
        }
    }

    /// Force-set the x-direction; pauses.
    fn force_set_x_dir(&mut self, dir: XDir) {
        delay_us(DELAY_DIREC);
        self.pin_x_direc.set_state(dir.to_pin_state()).unwrap();
        delay_us(DELAY_DIREC);
        self.x_dir = dir;
    }

    /// Instantaneous reading of the left limit switch.
    fn left_limit_instantaneous(&self) -> LimitSwitchState {
        if self.pin_estop_l.is_high() {
            LimitSwitchState::Pressed
        } else {
            LimitSwitchState::Unpressed
        }
    }

    /// Instantanous reading of the right limit switch.
    fn right_limit_instantaneous(&self) -> LimitSwitchState {
        if self.pin_estop_r.is_high() {
            LimitSwitchState::Pressed
        } else {
            LimitSwitchState::Unpressed
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum XDir {
    Right,
    Left,
}
impl XDir {
    fn toggle(&self) -> XDir {
        match self {
            XDir::Right => XDir::Left,
            XDir::Left => XDir::Right,
        }
    }
    fn to_pin_state(&self) -> PinState {
        match self {
            XDir::Right => PinState::Low,
            XDir::Left => PinState::High,
        }
    }
}

enum ADir {
    Plus,
    Minus,
}

#[derive(PartialEq)]
enum LimitSwitchState {
    Pressed,
    Unpressed,
}

#[derive(Debug, uDebug)]
pub enum Error {
    Overflow,
    NotZeroed,
}
