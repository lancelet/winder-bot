#![no_std]
#![no_main]

mod command;
mod machine;
mod readln;

use command::Command;
use heapless::String;
use machine::Machine;

use arduino_hal::prelude::*;
use panic_halt as _;
use readln::readln;
use ufmt::uwriteln;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut machine = Machine::new();
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    let mut move_mode = MoveMode::Absolute;

    uwriteln!(&mut serial, "WINDERBOT!").unwrap_infallible();

    let mut serial_buffer = String::<512>::new();
    loop {
        match readln(&mut serial, &mut serial_buffer) {
            Ok(()) => {}
            Err(readln::Error::BufferOverflow) => {
                uwriteln!(&mut serial, "Error. Input buffer overflow.")
                    .unwrap_infallible();
            }
        }
        match Command::parse(&mut serial_buffer.as_str()) {
            Err(command::Error::InvalidGCode) => {
                uwriteln!(
                    &mut serial,
                    "Error. Invalid GCode \"{}\"",
                    serial_buffer.as_str()
                )
                .unwrap_infallible();
            }
            Ok(cmd) => {
                uwriteln!(&mut serial, "Info. Parsed: {:?}.", cmd)
                    .unwrap_infallible();
                match cmd {
                    Command::Zero => {
                        let steps = machine.zero_x();
                        uwriteln!(&mut serial, "Info. Steps: {}", steps)
                            .unwrap_infallible();
                    }
                    Command::AbsolutePositioning => {
                        move_mode = MoveMode::Absolute;
                    }
                    Command::RelativePositioning => {
                        move_mode = MoveMode::Relative;
                    }
                    Command::Move(m) => {
                        uwriteln!(&mut serial, "Info. Move: {:?}", m)
                            .unwrap_infallible();
                        match move_mode {
                            MoveMode::Relative => {
                                let microns = m.x_microns.unwrap();
                                match machine.relative_x_um(microns) {
                                    Ok(()) => {}
                                    Err(machine::Error::NotZeroed) => {
                                        uwriteln!(
                                            &mut serial,
                                            "Error. Machine not zeroed."
                                        )
                                        .unwrap_infallible();
                                    }
                                    Err(machine::Error::Overflow) => {
                                        uwriteln!(
                                            &mut serial,
                                            "Error. Overflow."
                                        )
                                        .unwrap_infallible();
                                    }
                                };
                            }
                            MoveMode::Absolute => {
                                uwriteln!(&mut serial, "TODO")
                                    .unwrap_infallible();
                            }
                        }
                    }
                }
                uwriteln!(&mut serial, "Ok.").unwrap_infallible();
            }
        }
    }
}

enum MoveMode {
    Relative,
    Absolute,
}
