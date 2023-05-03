#![no_std]

use spin::Once;

/// 定义时间
pub trait Timer: Sync {
    /// 当前时间
    fn get_time_us(&self) -> usize;
}

/// 库找到输出的方法：保存一个对象引用，这是一种单例。
static TIMER: Once<&'static dyn Timer> = Once::new();

/// 用户调用这个函数设置输出的方法。
pub fn init(time: &'static dyn Timer) {
    TIMER.call_once(|| time);
}

pub fn get_time_us() -> usize {
    TIMER.wait().get_time_us()
}

pub fn get_time_ms() -> usize {
    TIMER.wait().get_time_us() / 1000
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(u64);

impl Instant {
    pub fn now() -> Instant {
        Instant(get_time_us() as u64)
    }
}
