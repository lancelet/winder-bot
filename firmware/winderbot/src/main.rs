#![no_std]
#![no_main]

mod devices;

use arduino_hal::prelude::*;
use multistepper::{MicroSeconds, Steps};
use panic_halt as _;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    ufmt::uwriteln!(&mut serial, "Hello from Arduino!\r").unwrap_infallible();

    let pin_x_pulse = pins.d8.into_output();
    let pin_x_direction = pins.d9.into_output();
    let pin_limitswitch_neg = pins.d12.into_pull_up_input(); // left
    let pin_limitswitch_pos = pins.d13.into_pull_up_input(); // right

    let x_raw_stepper = devices::Stepper::new(
        pin_x_pulse,
        pin_x_direction,
        MicroSeconds::new(5),
        MicroSeconds::new(10),
    );
    let limit_switch_pos = devices::LimitSwitch::new(pin_limitswitch_pos);
    let limit_switch_neg = devices::LimitSwitch::new(pin_limitswitch_neg);
    let pos_stepper = multistepper::PositionedStepper::new(x_raw_stepper);
    let mut x_stepper = multistepper::LimitedStepper::new(
        pos_stepper,
        limit_switch_pos,
        limit_switch_neg,
    );

    let move_delay = MicroSeconds::new(100);
    x_stepper.run_zeroing::<devices::Delay>(move_delay, Steps::new(1024));

    loop {}
}
