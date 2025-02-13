use arduino_hal::{
    default_serial,
    hal::{
        port::{PD0, PD1},
        Atmega,
    },
    pac::USART0,
    pins,
    port::{
        mode::{Input, Output},
        Pin,
    },
    prelude::*,
    usart::{Usart, UsartOps},
    Peripherals, Pins,
};
use heapless::String;

/// Read from a UART.
///
/// This owns its own buffer.
///
/// # Type Parameters
///
/// - `N_CHARS`: Number of characters in the buffer.
pub struct ReadUart<const N_CHARS: usize> {
    serial: Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>>,
    buffer: heapless::String<N_CHARS>,
}
impl<const N_CHARS: usize> ReadUart<N_CHARS> {
    const BAUD_RATE: u32 = 57600;

    /// Creates a new ReadUart.
    pub fn new() -> Self {
        let peripherals: Peripherals = unsafe { Peripherals::steal() };
        let pins: Pins = pins!(peripherals);

        let serial = default_serial!(peripherals, pins, Self::BAUD_RATE);
        let buffer = heapless::String::new();

        Self { serial, buffer }
    }

    /// Reads a line from the Uart into the internal buffer.
    ///
    /// This blocks until a line is read.
    ///
    /// # Returns
    ///
    /// - `Ok(())`: if the read succeeded.
    /// - `Err(error)`: if the read failed.
    pub fn readln(&mut self) -> Result<(), Error> {
        readln_(&mut self.serial, &mut self.buffer)
    }
}

impl<const N_CHARS: usize> AsRef<str> for ReadUart<N_CHARS> {
    fn as_ref(&self) -> &str {
        self.buffer.as_ref()
    }
}

/// Read an ASCII line from the serial UART.
fn readln_<USART, RX, TX, const N: usize>(
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
