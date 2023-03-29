#![allow(unused)]

extern crate alloc;

use crate::{
    async_executor::{Executor, Task as AsyncTask, PinBoxFuture},
    mm::KAllocator,
    syscall::{sys_exit, sys_get_tid},
    thread,
    thread::TCBlock,
};
use alloc::{boxed::Box, collections::VecDeque, sync::Arc, vec, vec::Vec};
use core::{
    future::{self, Future},
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};
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
    pub executor: Arc<Mutex<Executor>>,
    /// time slice
    pub slice: usize, // 时间片数量 [1, NUM_SLICES_LEVELS]
}

impl Task {
    pub fn ticks(&self) -> usize {
        unsafe {self.executor.force_unlock();}
        self.executor.lock().ticks()
    }

    /// append the GLOBAL_BOXED_FUTURE to executor
    /// the GLOBAL_BOXED_FUTURE will set by the syscall
    pub fn append(&self) {
        unsafe {self.executor.force_unlock();}
        let mut lock = GLOBAL_BOXED_FUTURE.lock();
        let boxed_future = core::mem::replace(&mut *lock, Box::pin(async {}));
        self.executor.lock().spawn(AsyncTask::new(boxed_future));
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
    let mut executor = Arc::new(Mutex::new(Executor::new()));
    executor.lock().spawn(AsyncTask::new(f));
    let thread_executor = executor.clone();

    let tcb = thread::spawn(move || {
        thread_executor.lock().run();
        sys_exit();
    });

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
    let tid = NEXT_ID.fetch_add(1, Ordering::Relaxed);

    let task = Task {
        tid: tid,
        tcb: tcb,
        executor,
        slice: 1,
    };

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
    let mut lock = QUEUES.lock();
    for i in 0..NUM_SLICES_LEVELS {
        for j in 0..lock[i].len() {
            if lock[i][j].tid == tid {
                lock[i].swap(0, j);
                return lock[i].pop_front();
            }
        }
    }

    None
}

/// append task (GLOBAL_BOXED_FUTURE) to task of tid
pub fn handle_append_task(task: Task, tid: usize) -> (Task, usize) {
    let mut ret = usize::MAX;

    if tid == task.tid {
        task.append();
        ret = tid;
    } else {
        if let Some(task) = get_task_by_tid(tid) {
            ret = tid;
            task.append();
            add_task_to_queue(task);
        }
    }

    (task, ret)
}
