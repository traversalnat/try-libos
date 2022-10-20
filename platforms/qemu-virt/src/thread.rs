#![no_std]

extern crate alloc;

use kernel_context::{LocalContext};
use spin::{Mutex, Once};
use alloc::vec::Vec;
use alloc::vec;

static THREADS: Once<Mutex<Vec<LocalContext>>> = Once::new();

struct thread;

impl thread {
    pub fn init() {
        THREADS.call_once(|| Mutex::new(vec![]));
        let mut scheduling = LocalContext::thread(Self::schedule as _, false);
        unsafe {
            scheduling.execute();
        }
    }

    pub fn add_thread(f: fn()) {
        let t = LocalContext::thread(f as _, false);
        THREADS.wait().lock().push(t);
    }

    extern "C" fn schedule() -> ! {
        unreachable!()
    }
}
