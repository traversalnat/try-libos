#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use spin::Once;

pub trait Thread: Sync {
    fn spawn(&self, f: Box<dyn FnOnce() + Send>);
    fn yields(&self);
}

static THREAD: Once<&'static dyn Thread> = Once::new();

pub fn init(tr: &'static dyn Thread) {
    THREAD.call_once(|| tr);
}

pub fn spawn<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    THREAD.wait().spawn(Box::new(f));
}

pub fn yields() {
    THREAD.wait().yields();
}
