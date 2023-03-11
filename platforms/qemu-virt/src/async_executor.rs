extern crate alloc;

use alloc::{boxed::Box, collections::LinkedList, sync::Arc};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use spin::Mutex;


pub use futures::{self, future::poll_fn, join};

type PinBoxFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub(crate) type Queue = Arc<Mutex<LinkedList<PinBoxFuture>>>;

/// Runtime definition
#[derive(Clone)]
pub(crate) struct Runtime {
    pub(crate) task_queue: Queue,
}

impl Runtime {
    pub fn task_pop_front(&self) -> Option<PinBoxFuture> {
        self.task_queue.lock().pop_front()
    }

    pub fn task_push_back(&self, task: PinBoxFuture) {
        self.task_queue.lock().push_back(task)
    }
}

pub struct Runner {
    runtime: Runtime, // 运行时
    ticks: u8,        // 协程 poll 次数
}

impl Runner {
    pub fn new() -> Self {
        let runtime = Runtime {
            task_queue: Arc::new(Mutex::new(LinkedList::new())),
        };
        Self { runtime, ticks: 0 }
    }

    pub fn append<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.runtime.task_push_back(Box::pin(future));
    }

    pub fn ticks(&self) -> u8 {
        self.ticks
    }

    /// TODO: 执行协程调度算法
    pub fn run_and_sched(&mut self) {
        let waker = async_task::waker_fn(|| {});

        let mut cx = Context::from_waker(&waker);

        while let Some(mut handle) = self.runtime.task_pop_front() {
            let check_handle = unsafe { Pin::new_unchecked(&mut handle) };
            match Future::poll(check_handle, &mut cx) {
                Poll::Ready(_) => {
                    continue;
                }
                Poll::Pending => {
                    self.runtime.task_push_back(handle);
                }
            };

            self.ticks += 1;
        }
    }
}
