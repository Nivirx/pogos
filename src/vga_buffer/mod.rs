use core::fmt;
use core::fmt::Write;
use core::ptr::Unique;
use spin::Mutex;
use volatile::Volatile;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LGray = 7,
    DGray = 8,
    LBlue = 9,
    LGreen = 10,
    LCyan = 11,
    LRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy)]
struct ColorCode(u8);

impl ColorCode {
    const fn new(fg: Color, bg: Color) -> ColorCode {
        ColorCode((bg as u8) << 4 | (fg as u8))
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: Unique<Buffer>,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer().chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    fn buffer(&mut self) -> &mut Buffer {
        unsafe { self.buffer.as_mut() }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let buffer = self.buffer();
                let character = buffer.chars[row][col].read();
                buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer().chars[row][col].write(blank);
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte)
        }
        Ok(())
    }
}

pub static WRITER: Mutex<Writer> = Mutex::new(Writer {
    column_position: 0,
    color_code: ColorCode::new(Color::Green, Color::Black),
    buffer: unsafe { Unique::new_unchecked(0xb8000 as *mut _) },
});

#[macro_export]
macro_rules! vga_print {
    ($($arg:tt)*) => ({
        $crate::vga_buffer::vga_print(format_args!($($arg)*));
    });
}

pub fn vga_print(args: fmt::Arguments) {
    WRITER.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! vga_println {
    ($fmt:expr) => (vga_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (vga_print!(concat!($fmt, "\n"), $($arg)*));
}

// pretty sure this is really slow...or maybe its SDL in qemu
pub fn clear_screen() {
    for _ in 0..BUFFER_HEIGHT {
        vga_println!("");
    }
}
