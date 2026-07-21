//! COM1 serial output (0x3F8) — primary console for the test harness.

use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::{backend::PioBackend, Config, Uart16550Tty};

lazy_static! {
    pub static ref SERIAL1: Mutex<Uart16550Tty<PioBackend>> = Mutex::new({
        // SAFETY: 0x3F8 is the standard COM1 base port (REFERENCES.md); nothing
        // else in the kernel touches these ports, and the Mutex serializes use.
        unsafe { Uart16550Tty::new_port(0x3F8, Config::default()) }
            .expect("failed to initialize COM1")
    });
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1
        .lock()
        .write_fmt(args)
        .expect("Printing to serial failed");
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}
