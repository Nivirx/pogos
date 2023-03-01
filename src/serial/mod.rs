//! Minimal support for uart_16550 serial output.
//!
//! # Usage
//!
//! ```no_run
//! use uart_16550::SerialPort;
//!
//! const SERIAL_IO_PORT: u16 = 0x3F8;
//!
//! let mut serial_port = unsafe { SerialPort::new(SERIAL_IO_PORT) };
//! serial_port.init();
//!
//! // Now the serial port is ready to be used. To send a byte:
//! serial_port.send(42);
//! ```


use bitflags::bitflags;
use core::fmt;
use crate::x86_64::instructions::port::Port;
use spin::Mutex;

#[macro_use]
use lazy_static::lazy_static;

const SERIAL_IO_PORT: u16 = 0x3F8;

bitflags! {
    /// Interrupt enable flags
    struct IntEnFlags: u8 {
        const RECEIVED = 1;
        const SENT = 1 << 1;
        const ERRORED = 1 << 2;
        const STATUS_CHANGE = 1 << 3;
        // 4 to 7 are unused
    }
}

bitflags! {
    /// Line status flags
    struct LineStsFlags: u8 {
        const INPUT_FULL = 1;
        // 1 to 4 unknown
        const OUTPUT_EMPTY = 1 << 5;
        // 6 and 7 unknown
    }
}

/// An interface to a serial port that allows sending out individual bytes.
pub struct SerialPort {
    data: Port<u8>,
    int_en: Port<u8>,
    fifo_ctrl: Port<u8>,
    line_ctrl: Port<u8>,
    modem_ctrl: Port<u8>,
    line_sts: Port<u8>,
}

impl SerialPort {
    /// Creates a new serial port interface on the given I/O port.
    ///
    /// This function is unsafe because the caller must ensure that the given base address
    /// really points to a serial port device.
    pub const unsafe fn new(base: u16) -> SerialPort {
        SerialPort {
            data: Port::new(base),
            int_en: Port::new(base + 1),
            fifo_ctrl: Port::new(base + 2),
            line_ctrl: Port::new(base + 3),
            modem_ctrl: Port::new(base + 4),
            line_sts: Port::new(base + 5),
        }
    }

    /// Initializes the serial port.
    pub fn init(&mut self) {
        unsafe {
            self.int_en.write(0x00);
            self.line_ctrl.write(0x80);
            self.data.write(0x03);
            self.int_en.write(0x00);
            self.line_ctrl.write(0x03);
            self.fifo_ctrl.write(0xC7);
            self.modem_ctrl.write(0x0B);
            //self.int_en.write(0x01);
        }
    }

    fn line_sts(&self) -> LineStsFlags {
        unsafe { LineStsFlags::from_bits_truncate(self.line_sts.read()) }
    }

    /// Sends a byte on the serial port.
    pub fn send(&mut self, data: u8) {
        unsafe {
            match data {
                8 | 0x7F => {
                    while !self.line_sts().contains(LineStsFlags::OUTPUT_EMPTY) {}
                    self.data.write(8);
                    while !self.line_sts().contains(LineStsFlags::OUTPUT_EMPTY) {}
                    self.data.write(b' ');
                    while !self.line_sts().contains(LineStsFlags::OUTPUT_EMPTY) {}
                    self.data.write(8)
                }
                _ => {
                    while !self.line_sts().contains(LineStsFlags::OUTPUT_EMPTY) {}
                    self.data.write(data);
                }
            }
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.send(byte);
        }
        Ok(())
    }
}


lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        unsafe { 
            let mut serial_port = SerialPort::new(SERIAL_IO_PORT); 
            serial_port.init();
            Mutex::new(serial_port)
        }
    };
}

#[doc(hidden)]
pub fn print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).expect("Printing to serial failed");
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::serial::print(format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(
        concat!($fmt, "\n"), $($arg)*));
}