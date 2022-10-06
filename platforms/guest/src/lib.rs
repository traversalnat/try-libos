#![feature(linkage)]

use std::{io::{Read, Write}, process::exit, vec::Vec};

pub use platform::Platform;
pub use MacOS as PlatformImpl;

#[linkage = "weak"]
#[no_mangle]
fn obj_main() {
    panic!()
}

#[no_mangle]
fn main() {
    obj_main();
    exit(0);
}

pub struct MacOS;

impl platform::Platform for MacOS {
    #[inline]
    fn console_getchar() -> u8 {
        let mut buf = [0; 1];
        let mut stdin = std::io::stdin();
        if let Ok(_e) = stdin.read(&mut buf) {
            buf[0]
        } else {
            unimplemented!("stdin broken")
        }
    }

    #[inline]
    fn console_putchar(c: u8) {
        let buf = [c; 1];
        let mut stdout = std::io::stdout();
        if let Ok(_e) = stdout.write(&buf) {}
    }

    #[inline]
    fn frequency() -> usize {
        0
    }

    #[inline]
    fn rdtime() -> usize {
        0
    }

    #[inline]
    fn shutdown(error: bool) {
        if error {
            exit(-1)
        } else {
            exit(0)
        }
    }
}
