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

/*
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Get the default UART0 serial port
    let dp = unsafe { arduino_hal::Peripherals::steal() };
    let pins = arduino_hal::pins!(dp);
    let mut serial = default_serial!(dp, pins, 57600); // Set baud rate

    // Write panic message
    uwriteln!(&mut serial, "Panic!\r").unwrap_infallible();

    /*
    if let Some(loc) = info.location() {
        uwriteln!(&mut serial, "File: {}", loc.file()).unwrap_infallible();
        uwriteln!(&mut serial, "Line: {}", loc.line()).unwrap_infallible();
        uwriteln!(&mut serial, "Col : {}", loc.column()).unwrap_infallible();
    } else {
        uwriteln!(&mut serial, "(Location unknown)").unwrap_infallible();
    }
    */

    // Infinite loop to prevent further execution
    loop {}
}
*/
