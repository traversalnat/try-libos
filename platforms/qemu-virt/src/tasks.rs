#![allow(unused)]

extern crate alloc;

use crate::{async_executor::Runner, mm::KAllocator, thread::TCBlock};
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::vec;
use alloc::sync::Arc;
use spin::{Lazy, Mutex};
use core::future::Future;

// MLFQ 层数
pub const NUM_SLICES_LEVELS: usize = 5;

// MLFQ
pub static QUEUES: Lazy<Mutex<Vec<VecDeque<Task, KAllocator>, KAllocator>>> =
    Lazy::new(|| {
        let mut v = Vec::new_in(KAllocator);
        for _ in 0..NUM_SLICES_LEVELS {
            v.push(VecDeque::new_in(KAllocator));
        }
        Mutex::new(v)
    });

/// Task 包含一个线程与一个协程队列
pub struct Task {
    tcb: TCBlock, // 线程控制块
    ex: Arc<Mutex<Runner>>,   // 协程执行器
    pub slice: usize,    // 时间片数量 [1, NUM_SLICES_LEVELS]
}

impl Task {
    pub fn ticks(&self) -> u8 {
        unsafe {
            self.ex.force_unlock();
        }
        self.ex.lock().ticks()
    }

    pub fn append<F>(&mut self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.ex.lock().append(f)
    }

    pub fn run(&self) {
        unsafe {
            self.tcb.lock().execute();
        }
    }
}

pub(crate) fn spawn<F>(f: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let mut ex = Arc::new(Mutex::new(Runner::new()));
    ex.lock().append(f);

    let ex2 = ex.clone();
    let tcb = crate::thread::spawn(move || {
        ex2.lock().run_and_sched();
    });

    let task = Task {
        tcb: tcb,
        ex: ex,
        slice: 1,
    };

    add_task_to_queue(task);
}

// Add a process to the highest priority queue.
pub fn add_task_to_queue(task: Task) {
    let level: usize = task.slice - 1;
    QUEUES.lock()[level].push_back(task);
}
