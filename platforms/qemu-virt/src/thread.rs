#![no_std]

extern crate alloc;

use kernel_context::{LocalContext};
use alloc::{alloc::alloc, collections::LinkedList, sync::Arc};
use core::alloc::Layout;
use spin::{Lazy, Mutex};

const STACK_SIZE: usize = 0x8000;

// 处理器
pub static PROCESSOR : Lazy<Mutex<Processor>> = Lazy::new(|| {
    Mutex::new(Processor::new())
});

// 保存所有的线程
pub static THREADS: Lazy<Mutex<LinkedList<TaskControlBlock>>> = Lazy::new(|| {
    Mutex::new(LinkedList::new())
});

pub struct Processor {
    pub current: Option<Arc<TaskControlBlock>>,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
        }
    }

    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }

    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(|task| Arc::clone(task))
    }
}

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
