#![no_std]
extern crate alloc;

use alloc::collections::LinkedList;
use alloc::sync::Arc;
use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use spin::{Lazy, Mutex, Once};

pub use futures::join;

use stdio::log::info;

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

/// A spawned future and its current state.
type Task = async_task::Task<()>;

pub struct JoinHandle<R>(async_task::JoinHandle<R, ()>);

impl<R> Future for JoinHandle<R> {
    type Output = R;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.0).poll(cx) {
            Poll::Pending => {
                self.0.waker().wake();
                Poll::Pending
            }
            Poll::Ready(output) => Poll::Ready(output.expect("task failed")),
        }
    }
}

pub(crate) type Queue = Arc<Mutex<LinkedList<Task>>>;

/// Runtime definition
pub(crate) struct Runtime {
    pub(crate) task_queue: Queue,
}

impl Runtime {
    pub fn pop_front(&self) -> Option<Task> {
        self.task_queue.lock().pop_front()
    }

    pub fn push_back(&self, task: Task) {
        self.task_queue.lock().push_back(task);
    }
}

static RUNTIME: Lazy<Mutex<Runtime>> = Lazy::new(|| {
    let runtime = Runtime {
        task_queue: Arc::new(Mutex::new(LinkedList::new())),
    };

    assert!(EXECUTOR.wait().sys_cpus() >= 1);

    for _ in 0..=EXECUTOR.wait().sys_cpus() {
        EXECUTOR.wait().sys_spawn(Box::new(|| {
            loop {
                let task = match RUNTIME.lock().pop_front() {
                    Some(t) => t,
                    _ => continue,
                };
                task.run();
            }
        }));
    }

    Mutex::new(runtime)
});

/// Future yield
pub struct Yield {
    yielded: bool,
}

impl Yield {
    pub fn new() -> Self {
        Yield { yielded: false }
    }
}

impl Future for Yield {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.get_mut().yielded = true;
            Poll::Pending
        }
    }
}

pub async fn async_yield() {
    Yield::new().await;
}

/// Spawns a future on the executor.
pub fn spawn<F, R>(future: F) -> JoinHandle<R>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let (task, handle) = async_task::spawn(
        future,
        |t| {
            RUNTIME.lock().push_back(t);
        },
        (),
    );

    task.schedule();

    JoinHandle(handle)
}

pub fn block_on<F: Future>(mut future: F) -> F::Output {
    let waker = async_task::waker_fn(|| {});

    let mut cx = Context::from_waker(&waker);

    let mut future = unsafe { Pin::new_unchecked(&mut future) };
    loop {
        match Future::poll(future.as_mut(), &mut cx) {
            Poll::Ready(val) => break val,
            Poll::Pending => {
                EXECUTOR.wait().sys_yield();
            }
        };
    }
}
