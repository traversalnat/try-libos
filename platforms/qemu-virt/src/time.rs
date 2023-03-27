#![allow(unused)]

use riscv::register::time;

pub const CLOCK_FREQ: usize = 12500000;
const TICKS_PER_SEC: usize = 100;
const MILLI_PER_SEC: usize = 1_000;
const MICRO_PER_SEC: usize = 1_000_000;

/// get current time in microseconds
pub fn get_time_us() -> usize {
    (time::read() / (CLOCK_FREQ / MICRO_PER_SEC)) as usize
}

/// get current time in milliseconds
pub fn get_time_ms() -> usize {
    (time::read() / (CLOCK_FREQ / MILLI_PER_SEC)) as usize
}
