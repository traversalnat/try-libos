#![allow(unused)]

extern crate alloc;

use crate::{
    async_executor::{PinBoxFuture, Runner},
    mm::KAllocator,
    syscall::{sys_exit, sys_get_tid},
    thread::TCBlock,
};
use alloc::{boxed::Box, collections::VecDeque, sync::Arc, vec, vec::Vec};
use core::future::{self, Future};
use spin::{Lazy, Mutex};
use stdio::log::{self, info};

// MLFQ 层数
pub const NUM_SLICES_LEVELS: usize = 5;

// MLFQ
pub static QUEUES: Lazy<Mutex<Vec<VecDeque<Task, KAllocator>, KAllocator>>> = Lazy::new(|| {
    let mut v = Vec::new_in(KAllocator);
    for _ in 0..NUM_SLICES_LEVELS {
        v.push(VecDeque::new_in(KAllocator));
    }
    Mutex::new(v)
});

static GLOBAL_TID: Mutex<usize> = Mutex::new(0);

/// 用于存放系统调用传入的 future
pub static GLOBAL_BOXED_FUTURE: Lazy<Mutex<PinBoxFuture>> =
    Lazy::new(|| Mutex::new(Box::pin(async {})));

/// Task 包含一个线程与一个协程队列
pub struct Task {
    /// ID
    pub tid: usize,
    /// 线程控制块
    pub tcb: TCBlock,
    /// 协程执行器
    pub ex: Arc<Mutex<Runner>>,
    pub slice: usize, // 时间片数量 [1, NUM_SLICES_LEVELS]
}

impl Task {
    pub fn ticks(&self) -> u8 {
        unsafe {
            self.ex.force_unlock();
        }
        self.ex.lock().ticks()
    }

    /// append the GLOBAL_BOXED_FUTURE to executor
    /// the GLOBAL_BOXED_FUTURE will set by the syscall
    pub fn append(&self) {
        let mut lock = GLOBAL_BOXED_FUTURE.lock();
        let boxed_future = core::mem::replace(&mut *lock, Box::pin(async {}));
        self.ex.lock().append(boxed_future);
    }

    pub fn run(&self) {
        unsafe {
            self.tcb.lock().execute();
        }
    }
}

pub(crate) fn spawn<F>(f: F) -> usize
where
    F: Future<Output = ()> + Send + 'static,
{
    let mut ex = Arc::new(Mutex::new(Runner::new()));
    ex.lock().append(Box::pin(f));

    let ex2 = ex.clone();
    let tcb = crate::thread::spawn(move || {
        // WARN: 锁会被调度器强制释放
        ex2.lock().run_and_sched();
        sys_exit();
    });

    let tid: usize = *GLOBAL_TID.lock();

    let task = Task {
        tid: tid,
        tcb: tcb,
        ex: ex,
        slice: 1,
    };

    *GLOBAL_TID.lock() += 1;

    add_task_to_queue(task);

    tid
}

/// Add a process to the highest priority queue.
pub fn add_task_to_queue(task: Task) {
    // let level: usize = task.slice - 1;
    QUEUES.lock()[0].push_back(task);
}

/// get task by tid
pub fn get_task_by_tid(tid: usize) -> Option<Task> {
    for i in 0..NUM_SLICES_LEVELS {
        let mut lock = QUEUES.lock();
        for j in 0..lock[i].len() {
            if lock[i][j].tid == tid {
                lock[i].swap(0, j);
                return lock[i].pop_front();
            }
        }
    }

    None
}
