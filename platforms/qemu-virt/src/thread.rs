#![no_std]

extern crate alloc;

use kernel_context::{LocalContext};
use alloc::{alloc::alloc, collections::LinkedList};
use core::alloc::Layout;
use spin::{Lazy, Mutex};

const STACK_SIZE: usize = 0x8000;

// 由于只会被调度进程使用，不考虑并发安全
pub static THREADS: Lazy<Mutex<LinkedList<TaskControlBlock>>> = Lazy::new(|| {
    Mutex::new(LinkedList::new())
});

/// 任务控制块。
///
/// 包含任务的上下文、状态和资源。
pub struct TaskControlBlock {
    ctx: LocalContext,
    pub finish: bool,
}

impl TaskControlBlock {
    pub const ZERO: Self = Self {
        ctx: LocalContext::empty(),
        finish: false,
    };

    /// 初始化一个任务。
    pub fn init(&mut self, entry: usize) {
        self.ctx = LocalContext::thread(entry, true);
        let bottom =
            unsafe { alloc(Layout::from_size_align(STACK_SIZE, STACK_SIZE).unwrap()) } as usize;
        *self.ctx.sp_mut() = bottom + STACK_SIZE;
    }

    /// 执行此任务。
    #[inline]
    pub unsafe fn execute(&mut self) {
        self.ctx.execute();
    }
}
