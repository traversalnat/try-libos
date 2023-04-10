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

use crate::{syscall::sys_yield, TASKNUM};

pub type PinBoxFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct AsyncTaskId(u64);

impl AsyncTaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        AsyncTaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct AsyncTask {
    id: AsyncTaskId,
    future: PinBoxFuture,
    io: bool, // 默认不是 I/O 任务
}

impl AsyncTask {
    pub fn new(future: impl Future<Output = ()> + Send + 'static) -> AsyncTask {
        AsyncTask {
            id: AsyncTaskId::new(),
            future: Box::pin(future),
            io: false,
        }
    }
}

struct TaskWaker {
    task_id: AsyncTaskId,
    task_queue: Arc<ArrayQueue<AsyncTaskId>>,
}

impl TaskWaker {
    fn new(task_id: AsyncTaskId, task_queue: Arc<ArrayQueue<AsyncTaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        match self.task_queue.push(self.task_id) {
            Err(_) => {
                panic!("task_queue full {}", self.task_queue.len());
            }
            _ => {}
        }
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
    tasks: BTreeMap<AsyncTaskId, AsyncTask>,
    task_queue: Arc<ArrayQueue<AsyncTaskId>>,
    waker_cache: BTreeMap<AsyncTaskId, Waker>,
    current: AsyncTaskId,
    ticks: usize,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(TASKNUM)),
            waker_cache: BTreeMap::new(),
            current: AsyncTaskId(0),
            ticks: 0,
        }
    }
}

impl Executor {
    pub fn spawn(&mut self, task: AsyncTask) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("task with same ID already in tasks");
        }
        self.task_queue.push(task_id).expect("queue full");
    }

    pub fn ticks(&self) -> usize {
        self.ticks
    }

    pub fn queue_len(&self) -> usize {
        self.tasks.len()
    }

    fn run_ready_tasks(&mut self) {
        let tasks = &mut self.tasks;
        let task_queue = &mut self.task_queue;
        let waker_cache = &mut self.waker_cache;

        while let Ok(task_id) = task_queue.pop() {
            self.current = task_id;
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
                Poll::Pending => {
                    task.io = true; // task is I/O task
                }
            }

            self.ticks += 1;
        }
    }

    /// steal all tasks in task_queue
    pub fn steal(&mut self) -> Option<Executor> {
        let task_queue = &mut self.task_queue;
        let tasks = &mut self.tasks;
        let waker_cache = &mut self.waker_cache;

        let current = match tasks.get_mut(&self.current) {
            Some(task) => task,
            None => return None,
        };

        // if current task don't has waker_cache, move it to new thread
        if task_queue.len() > 1 && !current.io {
            let new_task_queue: Arc<ArrayQueue<AsyncTaskId>> = Arc::new(ArrayQueue::new(TASKNUM));
            let mut new_tasks: BTreeMap<AsyncTaskId, AsyncTask> = BTreeMap::new();
            while let Ok(task_id) = task_queue.pop() {
                new_task_queue.push(task_id).expect("ArrayQueue push error");
                if let Some(task) = tasks.remove(&task_id) {
                    new_tasks.insert(task_id, task);
                }
                // 由于协程执行器变化，需要清除当前执行器中的 waker 缓存
                waker_cache.remove(&task_id);
            }

            return Some(Executor {
                task_queue: new_task_queue,
                tasks: new_tasks,
                waker_cache: BTreeMap::new(),
                current: AsyncTaskId(0),
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
