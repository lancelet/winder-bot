#![no_std]
#![no_main]

mod commands;
mod devices;
mod machine;

use machine::Machine;
use panic_halt as _;

#[arduino_hal::entry]
fn main() -> ! {
    let mut machine = Machine::new();
    loop {
        machine.next_command();
    }
}
