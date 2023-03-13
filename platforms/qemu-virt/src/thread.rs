#![allow(unused)]

extern crate alloc;

use crate::mm::KAllocator;
use alloc::{
    alloc::{alloc, dealloc},
    boxed::Box,
    collections::VecDeque,
    fmt, format,
    sync::Arc,
    vec::Vec,
};
use core::alloc::Layout;
use kernel_context::LocalContext;
use spin::{Lazy, Mutex};
use stdio::log::{self, info};

const STACK_SIZE: usize = 0x8000;

pub type TCBlock = Arc<Mutex<TaskControlBlock>>;

/// 线程状态
#[derive(PartialEq, Clone, Debug)]
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
    pub ctx: LocalContext,
    /// 栈底部地址
    pub stack: usize,
    /// 返回值
    pub exit_code: Option<i32>,
    /// 状态
    pub status: TaskStatus,
}

impl fmt::Debug for TaskControlBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskControlBlock")
            .field("context", &self.ctx)
            .field("stack_top", &format!("{:X}", &(&self.stack + STACK_SIZE)))
            .field("stack_bottom", &format!("{:X}", &self.stack))
            .field("exit_code", &self.exit_code)
            .field("status", &self.status)
            .finish()
    }
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

    /// 初始化一个任务。
    pub fn init_with_arg(&mut self, entry: usize, arg: usize) {
        self.init(entry);
        *self.ctx.a_mut(0) = arg;
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

    pub fn move_next(&mut self) {
        self.ctx.move_next();
    }

    /// 执行此任务。
    #[inline]
    pub unsafe fn execute(&mut self) {
        self.ctx.execute();
    }
}

impl Drop for TaskControlBlock {
    fn drop(&mut self) {
        if self.stack != 0 {
            unsafe {
                dealloc(self.stack as *mut u8, Layout::from_size_align(STACK_SIZE, STACK_SIZE).unwrap());
            }
        }
    }
}

struct ThreadRunner<F>
where
    F: FnOnce() + Send + 'static,
{
    tcb: TCBlock,
    closure: Option<F>,
}
pub trait ThreadRun {
    fn run(&mut self);
}

impl<F> ThreadRun for ThreadRunner<F>
where
    F: FnOnce() + Send + 'static,
{
    fn run(&mut self) {
        let closure = self.closure.take().expect("you can't run a thread twice!");
        (closure)();
        self.tcb.lock().status = TaskStatus::Finish;
    }
}

pub type BoxedThreadRun = Box<dyn ThreadRun>;

fn leak_boxed_thread_run(b: Box<BoxedThreadRun>) -> usize {
    Box::leak(b) as *mut _ as usize
}

fn restore_boxed_thread_run(a: usize) -> Box<BoxedThreadRun> {
    unsafe { Box::from_raw(a as *mut BoxedThreadRun) }
}

extern "C" fn run_boxed_thread(arg: usize) {
    let mut boxed_thread_run = restore_boxed_thread_run(arg);
    boxed_thread_run.run();
    drop(boxed_thread_run);
}

/// 创建一个线程，并返回 TCBlock
pub fn spawn<F>(f: F) -> TCBlock
where
    F: FnOnce() + Send + 'static,
{
    let mut t = TaskControlBlock::ZERO;
    t.status = TaskStatus::Ready;

    let t = Arc::new(Mutex::new(t));

    let runner = ThreadRunner {
        tcb: t.clone(),
        closure: Some(f),
    };

    let box_runner: Box<BoxedThreadRun> = Box::new(Box::new(runner));
    let arg = leak_boxed_thread_run(box_runner);

    t.lock().init_with_arg(run_boxed_thread as usize, arg);

    t
}
