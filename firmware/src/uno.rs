use arduino_hal::{
    hal::port::{PD0, PD1},
    pac::USART0,
    port::{
        mode::{Input, Output},
        Pin,
    },
    Usart,
};

pub type UnoSerial = Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>>;
