use alloc::collections::LinkedList;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use alloc::boxed::Box;
use spin::{Lazy, Mutex};

pub use futures::join;

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

    pub fn pop_back(&self) -> Option<Task> {
        self.task_queue.lock().pop_back()
    }

    pub fn push_back(&self, task: Task) {
        self.task_queue.lock().push_back(task);
    }

    pub fn push_front(&self, task: Task) {
        self.task_queue.lock().push_front(task);
    }
}

static RUNTIME: Lazy<Mutex<Runtime>> = Lazy::new(|| {
    let mut runtime = Runtime {
        task_queue: Arc::new(Mutex::new(LinkedList::new())),
    };

    // change 2 to CPUS NUM
    for _ in 0..2 {
        crate::thread::spawn(move || loop {
            let task = match RUNTIME.lock().pop_front() {
                Some(t) => t,
                _ => continue,
            };
            task.run();
        });
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
                crate::timer::sys_yield();
            }
        };
    }
}
