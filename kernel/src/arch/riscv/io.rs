use crate::drivers::SERIAL_DRIVERS;
use core::fmt::{self, Arguments, Write};
use super::sbi;

struct FmtWritter;

impl Write for FmtWritter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            sbi::console_putchar(c as usize);
        }
        Ok(())
    }
}


pub fn putfmt(fmt: Arguments) {
    //putstr("putfmt");
    // output to serial
    /*
    let mut drivers = SERIAL_DRIVERS.write();
    if let Some(serial) = drivers.first_mut() {
        serial.write(format!("{}", fmt).as_bytes());
    }
     */
    /*
    for byte in format!("{}", fmt).as_bytes() {
        sbi::console_putchar(*byte as usize);
    }
     */
    FmtWritter.write_fmt(fmt).unwrap();
    // might miss some early messages, but it's okay
}

pub fn putstr(str: &'static str) {
    for c in str.as_bytes() {
        sbi::console_putchar(*c as usize);
    }
}

