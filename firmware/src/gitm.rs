use arduino_hal::{
    delay_us,
    port::{
        mode::{Input, Output, PullUp},
        Pin, D10, D11, D12, D13, D8, D9,
    },
    Peripherals, Pins,
};
use embedded_hal::digital::{OutputPin, PinState};

/// `GhostInTheMachine`: Low-level (unsafe!) machine interface.
pub struct GhostInTheMachine {
    pin_x_pulse: Pin<Output, D8>,
    pin_x_direc: Pin<Output, D9>,
    pin_a_pulse: Pin<Output, D10>,
    pin_a_direc: Pin<Output, D11>,
    pin_limitswitch_l: Pin<Input<PullUp>, D13>,
    pin_limitswitch_r: Pin<Input<PullUp>, D12>,
    x_dir: PinState,
    a_dir: PinState,
}

impl GhostInTheMachine {
    const DELAY_DIREC_US: u32 = 10;
    const DELAY_PULSE_US: u32 = 5;
    const DELAY_MOVE_US: u32 = 40;
    const X_EDGE_SAFETY_STEPS: u32 = 3200;

    pub fn new() -> Self {
        let peripherals: Peripherals =
            unsafe { arduino_hal::Peripherals::steal() };
        let pins: Pins = arduino_hal::pins!(peripherals);

        let mut gitm = GhostInTheMachine {
            pin_x_pulse: pins.d8.into_output(),
            pin_x_direc: pins.d9.into_output(),
            pin_a_pulse: pins.d10.into_output(),
            pin_a_direc: pins.d11.into_output(),
            pin_limitswitch_l: pins.d13.into_pull_up_input(),
            pin_limitswitch_r: pins.d12.into_pull_up_input(),
            x_dir: PinState::Low,
            a_dir: PinState::Low,
        };
        gitm.force_set_x_dir(PinState::Low);
        gitm.force_set_a_dir(PinState::Low);

        gitm
    }

    /// Zero the machine.
    ///
    /// This does the following:
    /// 1. Moves the machine to the left limit switch.
    /// 2. Moves to the right limit switch (counting the steps).
    /// 3. Moves to the middle (at half the number of steps).
    ///
    /// # Returns
    /// The number of steps from the left limit switch to the right.
    pub fn zero(&mut self) -> u32 {
        let _ = self.move_to_left_limit_switch();
        let count = self.move_to_right_limit_switch();
        for _ in 0..(count / 2) {
            self.step_x(PinState::High);
            delay_us(Self::DELAY_MOVE_US);
        }
        count
    }

    /// Move the carriage until the left limit switch is engaged.
    ///
    /// NOTE: This assumes that a move with a HIGH direction pin moves toward
    ///       the left limit switch.
    ///
    /// # Returns
    /// The number of steps.
    pub fn move_to_left_limit_switch(&mut self) -> u32 {
        let mut count: u32 = 0;
        // Move on to the limit switch.
        while !self.left_limit_switch_is_down()
            && !self.right_limit_switch_is_down()
        {
            self.step_x_unsafe(PinState::High);
            count += 1;
            delay_us(Self::DELAY_MOVE_US);
        }
        // In the unlikely case that the right limit switch is down; just do
        // nothing at this point.
        if self.right_limit_switch_is_down() {
            return 0;
        }
        // Move off the left limit switch.
        while self.left_limit_switch_is_down() {
            self.step_x_unsafe(PinState::Low);
            count -= 1;
            delay_us(Self::DELAY_MOVE_US);
        }
        // Take some extra steps to make sure we're really off it.
        let mut extra_steps = Self::X_EDGE_SAFETY_STEPS;
        while !self.right_limit_switch_is_down() && extra_steps > 0 {
            self.step_x_unsafe(PinState::Low);
            extra_steps -= 1;
            delay_us(Self::DELAY_MOVE_US);
        }
        count
    }

    /// Move the carriage until the right limit switch is engaged.
    ///
    /// NOTE: This assumes that a move with a LOW direction pin moves toward
    ///       the right limit switch.
    ///
    /// # Returns
    /// The number of steps.
    pub fn move_to_right_limit_switch(&mut self) -> u32 {
        let mut count: u32 = 0;
        // Move on to the limit switch.
        while !self.left_limit_switch_is_down()
            && !self.right_limit_switch_is_down()
        {
            self.step_x_unsafe(PinState::Low);
            count += 1;
            delay_us(Self::DELAY_MOVE_US);
        }
        // In the unlikely case that the right limit switch is down; just do
        // nothing at this point.
        if self.left_limit_switch_is_down() {
            return 0;
        }
        // Move off the left limit switch.
        while self.right_limit_switch_is_down() {
            self.step_x_unsafe(PinState::High);
            count -= 1;
            delay_us(Self::DELAY_MOVE_US);
        }
        // Take some extra steps to make sure we're really off it.
        let mut extra_steps = Self::X_EDGE_SAFETY_STEPS;
        while !self.left_limit_switch_is_down() && extra_steps > 0 {
            self.step_x_unsafe(PinState::High);
            extra_steps -= 1;
            delay_us(Self::DELAY_MOVE_US);
        }
        count
    }

    /// Take a step along a.
    pub fn step_a(&mut self, dir: PinState) {
        self.set_a_dir(dir);
        self.pin_a_pulse.set_high();
        delay_us(Self::DELAY_PULSE_US);
        self.pin_a_pulse.set_low();
        delay_us(Self::DELAY_PULSE_US);
    }

    /// Take a step along x, provided that neither limit switch is triggered.
    ///
    /// # Returns
    /// `true` if the step could be taken, `false` if a limit switch was
    /// engage.
    pub fn step_x(&mut self, dir: PinState) -> bool {
        if !self.left_limit_switch_is_down()
            && !self.right_limit_switch_is_down()
        {
            self.step_x_unsafe(dir);
            true
        } else {
            false
        }
    }

    /// Take a step along x, ignoring limit switches.
    pub fn step_x_unsafe(&mut self, dir: PinState) {
        self.set_x_dir(dir);
        self.pin_x_pulse.set_high();
        delay_us(Self::DELAY_PULSE_US);
        self.pin_x_pulse.set_low();
        delay_us(Self::DELAY_PULSE_US);
    }

    /// Read the value of the left limit switch.
    pub fn left_limit_switch_is_down(&self) -> bool {
        self.pin_limitswitch_l.is_high()
    }

    /// Read the value of the right limit switch.
    pub fn right_limit_switch_is_down(&self) -> bool {
        self.pin_limitswitch_r.is_high()
    }

    /// Set the x direction flag if necessary.
    fn set_x_dir(&mut self, dir: PinState) {
        if dir != self.x_dir {
            self.force_set_x_dir(dir);
        }
    }

    /// Set the a direction flag if necessary.
    fn set_a_dir(&mut self, dir: PinState) {
        if dir != self.a_dir {
            self.force_set_a_dir(dir);
        }
    }

    /// Force set the x direction flag and pin to the given value (high or low),
    /// regardless of the current direction flag.
    fn force_set_x_dir(&mut self, state: PinState) {
        delay_us(Self::DELAY_DIREC_US);
        self.pin_x_direc.set_state(state).unwrap();
        self.x_dir = state;
        delay_us(Self::DELAY_DIREC_US);
    }

    /// Force set the a direction flag and pin to the given value (high or low),
    /// regardless of the current direction flag.
    fn force_set_a_dir(&mut self, state: PinState) {
        delay_us(Self::DELAY_DIREC_US);
        self.pin_a_direc.set_state(state).unwrap();
        self.a_dir = state;
        delay_us(Self::DELAY_DIREC_US);
    }
}
