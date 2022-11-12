#![feature(linkage)]

mod basic;
mod eth;

pub use basic::MacOS as PlatformImpl;
use basic::ETH_DEVICE;
use core::time::Duration;
pub use eth::MACADDR;
use executor;
pub use platform::Platform;
use std::process::exit;
use stdio::{self};

#[linkage = "weak"]
#[no_mangle]
fn obj_main() {
    panic!()
}

#[no_mangle]
fn main() {
    // mem is not needed in std environment
    // stdio
    stdio::set_log_level(option_env!("LOG"));
    stdio::init(&Stdio);

    executor::init(&basic::Executor);

    PlatformImpl::spawn(|| loop {
        ETH_DEVICE.lock().async_recv();
        PlatformImpl::wait(Duration::from_millis(100));
    });

    obj_main();
    exit(0);
}

struct Stdio;
impl stdio::Stdio for Stdio {
    #[inline]
    fn put_char(&self, c: u8) {
        PlatformImpl::console_putchar(c);
    }

    #[inline]
    fn put_str(&self, s: &str) {
        PlatformImpl::console_put_str(s);
    }

    #[inline]
    fn get_char(&self) -> u8 {
        PlatformImpl::console_getchar()
    }
}
