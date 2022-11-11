#![no_std]
extern crate alloc;

mod utils;

use alloc::{boxed::Box, collections::LinkedList, sync::Arc};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use spin::{Mutex, Once};


pub use futures::{self, future::poll_fn, join};
pub use utils::async_yield;

/// Executor trait
pub trait Executor: Sync + Send {
    fn sys_cpus(&self) -> usize {
        1
    }

    fn sys_spawn(&self, f: Box<dyn FnOnce() + Send>);

    fn sys_yield(&self);
}

/// EXECUTOR
static EXECUTOR: Once<&'static dyn Executor> = Once::new();

/// init EXECUTOR
pub fn init(executor: &'static dyn Executor) {
    EXECUTOR.call_once(|| executor);
}

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
    runtime: Runtime,
}

impl Runner {
    pub fn new() -> Self {
        let runtime = Runtime {
            task_queue: Arc::new(Mutex::new(LinkedList::new())),
        };
        Self { runtime }
    }

    /// Spawns a future on the executor.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.runtime.task_push_back(Box::pin(future));
    }

    // one thread executor
    pub fn block_on<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let waker = async_task::waker_fn(|| {});

        let mut cx = Context::from_waker(&waker);

        self.spawn(future);

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
        }
    }
}
