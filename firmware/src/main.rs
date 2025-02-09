#![no_std]
#![no_main]

mod command;
mod controller;
mod gitm;
mod machine;
mod readln;
mod uno;

use controller::Controller;
use panic_halt as _;

#[arduino_hal::entry]
fn main() -> ! {
    let mut controller = Controller::new();

    loop {
        controller.command_step();
    }
}
