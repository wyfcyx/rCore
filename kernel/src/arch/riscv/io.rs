use crate::drivers::SERIAL_DRIVERS;
use core::fmt::{Arguments, Write};
use super::sbi;
pub fn putfmt(fmt: Arguments) {
    // output to serial
    /*
    let mut drivers = SERIAL_DRIVERS.write();
    if let Some(serial) = drivers.first_mut() {
        serial.write(format!("{}", fmt).as_bytes());
    }
     */
    for byte in format!("{}", fmt).as_bytes() {
        sbi::console_putchar(*byte as usize);
    }
    // might miss some early messages, but it's okay
}
