#![allow(unused)]

extern crate alloc;
use crate::thread::*;
use alloc::{
    sync::Arc,
    collections::BinaryHeap,
};
use core::cmp::Ordering;
use spin::{Lazy, Mutex};
use riscv::register::*;

pub const CLOCK_FREQ: usize = 12500000;
const TICKS_PER_SEC: usize = 100;
const MILLI_PER_SEC: usize = 1_000;
const MICRO_PER_SEC: usize = 1_000_000;

pub struct TimerCondVar {
    pub expire_ms: u128,
    pub task: Arc<Mutex<TaskControlBlock>>,
}

impl PartialEq for TimerCondVar {
    fn eq(&self, other: &Self) -> bool {
        self.expire_ms == other.expire_ms
    }
}
impl Eq for TimerCondVar {}
impl PartialOrd for TimerCondVar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let a = -(self.expire_ms as isize);
        let b = -(other.expire_ms as isize);
        Some(a.cmp(&b))
    }
}

impl Ord for TimerCondVar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

pub static TIMERS: Lazy<Mutex<BinaryHeap<TimerCondVar>>> =
    Lazy::new( || Mutex::new(BinaryHeap::<TimerCondVar>::new()) );

pub fn move_timer(expire_ms: u128, task: Arc<Mutex<TaskControlBlock>>) {
    task.lock().status = TaskStatus::Blocking;
    TIMERS.lock().push(TimerCondVar { expire_ms, task });
}

/// 将到时线程移动至执行线程队列
pub fn check_timer() {
    let current_ms = get_time_ms();
    let mut timers = TIMERS.lock();
    while let Some(timer) = timers.peek() {
        if timer.expire_ms <= current_ms {
            move_run(Arc::clone(&timer.task));
            timers.pop();
        } else {
            break;
        }
    }
}

/// get current time in microseconds
pub fn get_time_us() -> u128 {
    (time::read() / (CLOCK_FREQ / MICRO_PER_SEC)) as u128
}

/// get current time in milliseconds
pub fn get_time_ms() -> u128 {
    (time::read() / (CLOCK_FREQ / MILLI_PER_SEC)) as u128
}

/// sleep current task
pub fn sys_sleep(ms: u128) -> isize {
    let expire_ms = get_time_ms() + ms;
    let ctx = current_thread();
    move_timer(expire_ms, ctx);
    sys_yield();
    0
}

/// yield
pub fn sys_yield() {
    let ctx = current_thread();
    unsafe {
        ctx.lock().execute_yield();
    }
}
