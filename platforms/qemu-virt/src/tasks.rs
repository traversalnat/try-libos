extern crate alloc;

use alloc::{boxed::Box, collections::LinkedList};
use stdio::log::info;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use spin::{Lazy, Mutex};

pub use futures::{self, future::poll_fn, join};

type PinBoxFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub static TASK_QUEUE: Lazy<Mutex<LinkedList<PinBoxFuture>>> =
    Lazy::new(|| Mutex::new(LinkedList::new()));

pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    TASK_QUEUE.lock().push_back(Box::pin(future));
}

// one thread executor
pub fn block_on<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let waker = async_task::waker_fn(|| {});

    let mut cx = Context::from_waker(&waker);

    TASK_QUEUE.lock().push_front(Box::pin(future));

    loop {
        let task = TASK_QUEUE.lock().pop_front();
        if let Some(mut handle) = task {
            let check_handle = unsafe { Pin::new_unchecked(&mut handle) };
            match Future::poll(check_handle, &mut cx) {
                Poll::Ready(_) => {
                    continue;
                }
                Poll::Pending => {
                    TASK_QUEUE.lock().push_back(handle);
                }
            };
        }
    }
}
