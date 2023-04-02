extern crate alloc;

use alloc::boxed::Box;

use alloc::{collections::BTreeMap, sync::Arc};


use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll, Waker},
};
use crossbeam_queue::ArrayQueue;

pub use futures::{self, future::poll_fn, join};

use crate::syscall::sys_yield;

pub type PinBoxFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

const TASKNUM: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct Task {
    id: TaskId, // new
    future: PinBoxFuture,
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + Send + 'static) -> Task {
        Task {
            id: TaskId::new(), // new
            future: Box::pin(future),
        }
    }
}

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task_queue full");
    }
}

impl alloc::task::Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}

/// Runtime definition
pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
    ticks: usize,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(TASKNUM)),
            waker_cache: BTreeMap::new(),
            ticks: 0,
        }
    }
}

impl Executor {
    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("task with same ID already in tasks");
        }
        self.task_queue.push(task_id).expect("queue full");
    }

    pub fn ticks(&self) -> usize {
        self.ticks
    }

    fn run_ready_tasks(&mut self) {
        let tasks = &mut self.tasks;
        let task_queue = &mut self.task_queue;
        let waker_cache = &mut self.waker_cache;

        while let Ok(task_id) = task_queue.pop() {
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue, // task no longer exists
            };
            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, task_queue.clone()));
            let mut context = Context::from_waker(waker);
            let handle = Pin::new(&mut task.future);
            match handle.poll(&mut context) {
                Poll::Ready(()) => {
                    // task done -> remove it and its cached waker
                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }

            self.ticks += 1;
        }
    }

    /// steal all tasks in task_queue
    pub fn steal(&mut self) -> Option<Executor> {
        let task_queue = &mut self.task_queue;
        let tasks = &mut self.tasks;
        let waker_cache = &mut self.waker_cache;

        if task_queue.len() > 1 {
            let new_task_queue: Arc<ArrayQueue<TaskId>> = Arc::new(ArrayQueue::new(TASKNUM));
            let mut new_tasks: BTreeMap<TaskId, Task> = BTreeMap::new();
            while let Ok(task_id) = task_queue.pop() {
                new_task_queue.push(task_id).expect("ArrayQueue push error");
                let task = tasks.remove(&task_id).unwrap();
                // 由于协程执行器变化，需要清除当前执行器中的 waker 缓存
                waker_cache.remove(&task_id);
                new_tasks.insert(task_id, task);
            }

            return Some(Executor {
                task_queue: new_task_queue,
                tasks: new_tasks,
                waker_cache: BTreeMap::new(),
                ticks: 0,
            });
        }
        None
    }

    pub fn run(&mut self) {
        loop {
            self.run_ready_tasks();
            if self.tasks.len() == 0 {
                break;
            } else {
                sys_yield();
            }
        }
    }
}
