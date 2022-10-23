#![no_std]

extern crate alloc;

use alloc::{
    alloc::{alloc, dealloc},
    collections::LinkedList,
    sync::Arc,
    vec,
    vec::Vec,
};
use core::alloc::Layout;
use kernel_context::LocalContext;
use spin::{Lazy, Mutex};

const STACK_SIZE: usize = 0x8000;

/// 正在运行的线程
pub static RUN_THREADS: Lazy<Mutex<LinkedList<Arc<Mutex<TaskControlBlock>>>>> =
    Lazy::new(|| Mutex::new(LinkedList::new()));

/// 所有的线程
pub static THREADS: Lazy<Mutex<Vec<Arc<Mutex<TaskControlBlock>>>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

pub(crate) static CURRENT: Lazy<Mutex<Arc<Mutex<TaskControlBlock>>>> =
    Lazy::new(|| Mutex::new(Arc::new(Mutex::new(TaskControlBlock::ZERO))));

pub fn move_run(ctx: Arc<Mutex<TaskControlBlock>>) {
    ctx.lock().status = TaskStatus::Ready;
    RUN_THREADS.lock().push_back(ctx);
}

/// 返回当前 thread
/// 由于 ctx.lock().excute() 执行当前线程会锁住 ctx, 这里强制 unlock
pub fn current_thread() -> Arc<Mutex<TaskControlBlock>> {
    let mut lock = CURRENT.lock();
    unsafe {
        (*lock).force_unlock();
    }
    (*lock).clone()
}

pub fn set_current_thread(ctx: Arc<Mutex<TaskControlBlock>>) {
    let mut lock = CURRENT.lock();
    *lock = ctx;
}

/// 线程状态
#[derive(PartialEq, Clone)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Blocking,
    Finish,
}

/// 任务控制块。
///
/// 包含任务的上下文、状态和资源。
pub struct TaskControlBlock {
    /// 上下文
    ctx: LocalContext,
    /// 栈底部地址
    stack: usize,
    /// 返回值
    pub exit_code: Option<i32>,
    /// 状态
    pub status: TaskStatus,
}

impl TaskControlBlock {
    pub const ZERO: Self = Self {
        ctx: LocalContext::empty(),
        stack: 0,
        exit_code: None,
        status: TaskStatus::UnInit,
    };

    /// 初始化一个任务。
    pub fn init(&mut self, entry: usize) {
        self.ctx = LocalContext::thread(entry, true);
        self.stack =
            unsafe { alloc(Layout::from_size_align(STACK_SIZE, STACK_SIZE).unwrap()) } as usize;
        *self.ctx.sp_mut() = self.stack + STACK_SIZE;
        self.status = TaskStatus::Ready;
    }

    /// 让线程执行另一个函数，不重新分配栈
    pub fn reinit(&mut self, entry: usize) {
        self.ctx = LocalContext::thread(entry, true);
        let stack = self.stack as *mut u8;
        unsafe {
            stack.write_bytes(0, STACK_SIZE);
        }
        *self.ctx.sp_mut() = self.stack + STACK_SIZE;
        self.status = TaskStatus::Ready;
    }

    /// 执行此任务。
    #[inline]
    pub unsafe fn execute(&mut self) {
        self.ctx.execute();
    }

    /// 执行此任务。
    #[inline]
    pub unsafe fn execute_yield(&mut self) {
        self.ctx.execute_yield();
    }
}

impl Drop for TaskControlBlock {
    fn drop(&mut self) {
        if self.stack != 0 {
            let layout = Layout::from_size_align(STACK_SIZE, STACK_SIZE).unwrap();
            let ptr = self.stack as *mut u8;
            unsafe {
                dealloc(ptr, layout);
            }
        }
    }
}
