use arduino_hal::prelude::*;
use arduino_hal::{hal::Atmega, usart::UsartOps, Usart};
use heapless::String;

/// Read an ASCII line from the serial UART.
pub fn readln<USART, RX, TX, const N: usize>(
    serial: &mut Usart<USART, RX, TX>,
    buffer: &mut String<N>,
) -> Result<(), Error>
where
    USART: UsartOps<Atmega, RX, TX>,
{
    buffer.clear();
    loop {
        let c = read_u8_blocking(serial);
        if c == b'\n' {
            break;
        }
        match buffer.push(c as char) {
            Ok(()) => {}
            Err(()) => return Err(Error::BufferOverflow),
        }
    }

    Ok(())
}

/// Block and wait for a character from a serial input.
fn read_u8_blocking<USART, RX, TX>(serial: &mut Usart<USART, RX, TX>) -> u8
where
    USART: UsartOps<Atmega, RX, TX>,
{
    nb::block!(serial.read()).unwrap_infallible()
}

/// Errors that might occur when reading.
#[derive(Debug)]
pub enum Error {
    /// A buffer overflow error.
    BufferOverflow,
}
