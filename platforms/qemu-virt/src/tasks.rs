#![allow(unused)]

extern crate alloc;

use crate::{
    async_executor::{Executor, PinBoxFuture, Task as AsyncTask},
    mm::KAllocator,
    syscall::{sys_exit, sys_get_tid},
    thread,
    thread::{TCBlock, TaskStatus},
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
pub const NUM_LEVELS: usize = 2;

// MLFQ
struct MlfqStruct {
    queue: Vec<VecDeque<Task, KAllocator>, KAllocator>,
    task: Option<Task>, // 时间片未使用完毕的任务
    level: usize,       // 0..NUM_SLICES_LEVELS
}

impl MlfqStruct {
    pub fn next_task(&mut self) -> Option<Task> {
        if self.task.is_some() {
            return core::mem::replace(&mut self.task, None);
        }

        // info!("=========");
        // for i in 0..NUM_LEVELS {
        //     info!("{i} {}", self.queue[i].len());
        // }
        // info!("=========");
        
        for _ in 0..NUM_LEVELS {
            let level = self.level;
            self.level += 1;
            self.level %= NUM_LEVELS;

            if let Some(task) = self.queue[level].pop_front() {
                return Some(task);
            }
        }

        None
    }

    pub fn get_task_by_tid(&mut self, tid: usize) -> Option<Task> {
        let queue = &mut self.queue;
        for i in 0..NUM_LEVELS {
            for j in 0..queue[i].len() {
                if queue[i][j].tid == tid {
                    queue[i].swap(0, j);
                    return queue[i].pop_front();
                }
            }
        }
        None
    }

    pub fn add_task_to_queue(&mut self, task: Task) {
        if task.io {
            self.queue[0].push_back(task);
        } else {
            self.queue[NUM_LEVELS - 1].push_back(task);
        }
    }

    pub fn add_task_transient(&mut self, task: Task) {
        match self.task {
            None => {
                let old = core::mem::replace(&mut self.task, Some(task));
            }
            Some(_) => {
                let task = core::mem::replace(&mut self.task, Some(task));
                self.add_task_to_queue(task.expect("error of unwrap"));
            }
        }
    }
}

// MLFQ
static MLFQ: Lazy<Mutex<MlfqStruct>> = Lazy::new(|| {
    let mut v = Vec::new_in(KAllocator);
    for _ in 0..NUM_LEVELS {
        v.push(VecDeque::new_in(KAllocator));
    }
    Mutex::new(MlfqStruct {
        queue: v,
        task: None,
        level: 0,
    })
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
    /// is I/O task
    pub io: bool,
}

impl Task {
    pub fn new(tcb: TCBlock, executor: Arc<Mutex<Executor>>, is_io: bool) -> Task {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let tid = NEXT_ID.fetch_add(1, Ordering::Relaxed);

        Task {
            tid: tid,
            tcb: tcb,
            executor,
            io: is_io
        }
    }
}

impl Task {
    pub fn status(&self) -> TaskStatus {
        self.tcb.lock().status
    }

    pub fn set_status(&mut self, status: TaskStatus) {
        self.tcb.lock().status = status;
    }

    pub fn ticks(&self) -> usize {
        // safe
        unsafe {
            self.executor.force_unlock();
        }
        self.executor.lock().ticks()
    }

    // create new Task steal tasks in Task's executor
    pub fn steal(&mut self) -> Option<Task> {
        unsafe {
            self.executor.force_unlock();
        }

        if let Some(executor) = self.executor.lock().steal() {
            let executor = Arc::new(Mutex::new(executor));
            let thread_executor = executor.clone();
            let tcb = thread::spawn(move || {
                thread_executor.lock().run();
                sys_exit();
            });

            return Some(Self::new(tcb, executor, true));
        }
        None
    }

    /// append the GLOBAL_BOXED_FUTURE to executor
    /// the GLOBAL_BOXED_FUTURE will set by the syscall
    pub fn append(&self) {
        unsafe {
            self.executor.force_unlock();
        }
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

pub(crate) fn spawn<F>(f: F, is_io: bool) -> usize
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

    let task = Task::new(tcb, executor, is_io);
    let tid = task.tid;

    add_task_to_queue(task);

    tid
}

/// Add a process to the highest priority queue.
pub fn add_task_to_queue(task: Task) {
    MLFQ.lock().add_task_to_queue(task);
}

pub fn add_task_transient(task: Task) {
    MLFQ.lock().add_task_transient(task);
}

pub fn get_task_from_queue() -> Option<Task> {
    MLFQ.lock().next_task()
}

/// get task by tid
pub fn get_task_by_tid(tid: usize) -> Option<Task> {
    MLFQ.lock().get_task_by_tid(tid)
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
