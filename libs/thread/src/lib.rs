#![no_std]
extern crate alloc;

use core::pin::Pin;
use alloc::boxed::Box;
use spin::Once;
use core::future::Future;

pub trait Thread: Sync {
    fn spawn(&self, f: Pin<Box<dyn Future<Output = ()> + Send>>, is_io: bool) -> usize;
    fn append_task(&self, f: Pin<Box<dyn Future<Output = ()> + Send>>) -> usize;
    fn yields(&self);
}

static THREAD: Once<&'static dyn Thread> = Once::new();

pub fn init(tr: &'static dyn Thread) {
    THREAD.call_once(|| tr);
}

pub fn spawn<F>(f: F, is_io: bool) -> usize
where
    F: Future<Output = ()> + Send + 'static,
{
    THREAD.wait().spawn(Box::pin(f), is_io)
}

// append_task to current thread
pub fn append_task<F>(f: F) -> usize
where
    F: Future<Output = ()> + Send + 'static,
{
    THREAD.wait().append_task(Box::pin(f))
}

pub fn yields() {
    THREAD.wait().yields();
}
