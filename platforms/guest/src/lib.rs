#![feature(linkage)]

mod basic;
mod eth;

pub use platform::Platform;
pub use basic::MacOS as PlatformImpl;

use std::process::exit;

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
