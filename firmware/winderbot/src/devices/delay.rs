use multistepper::MicroSeconds;

/// Real delay on the Arduino Uno microcontroller.
pub struct Delay;

impl multistepper::Delay for Delay {
    fn delay_us(microseconds: MicroSeconds) {
        arduino_hal::delay_us(microseconds.get_value());
    }
}
