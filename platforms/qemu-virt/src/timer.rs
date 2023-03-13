#![allow(unused)]

extern crate alloc;
use crate::{
    mm::KAllocator,
    tasks::{add_task_to_queue, Task},
    trap::*,
    Virt,
};
use alloc::{collections::VecDeque, sync::Arc};
use collections::heap::Heap;
use core::cmp::Ordering;
use riscv::register::*;
use spin::{Lazy, Mutex};
use stdio::log;

pub const CLOCK_FREQ: usize = 12500000;
const TICKS_PER_SEC: usize = 100;
const MILLI_PER_SEC: usize = 1_000;
const MICRO_PER_SEC: usize = 1_000_000;

pub struct TimerCondVar {
    pub expire_ms: usize,
    pub task: Task,
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

/// TIMERS: 等待队列，操作时必须关闭中断
pub static TIMERS: Lazy<Mutex<Heap<TimerCondVar, KAllocator>>> =
    Lazy::new(|| Mutex::new(Heap::new_in(KAllocator)));

pub(crate) fn move_timer(expire_ms: usize, task: Task) {
    TIMERS.lock().push(TimerCondVar { expire_ms, task });
}

// /// 将到时线程移动至执行线程队列
pub(crate) fn check_timer() {
    let current_ms = get_time_ms();
    let mut timers = TIMERS.lock();
    while let Some(cond) = timers.peek() {
        if cond.expire_ms <= current_ms {
            if let Some(task) = timers.pop() {
                add_task_to_queue(task.task);
            }
        } else {
            break;
        }
    }
}

/// get current time in microseconds
pub fn get_time_us() -> usize {
    (time::read() / (CLOCK_FREQ / MICRO_PER_SEC)) as usize
}

/// get current time in milliseconds
pub fn get_time_ms() -> usize {
    (time::read() / (CLOCK_FREQ / MILLI_PER_SEC)) as usize
}

/// sleep current task 设计成中断
pub fn sleep(task: Task, ms: usize) -> (Option<Task>, usize) {
    let expire_ms = get_time_ms() + ms;
    move_timer(expire_ms, task);
    (None, 0)
}

