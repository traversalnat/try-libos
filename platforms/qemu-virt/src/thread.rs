#![allow(unused)]

extern crate alloc;

use crate::{mm::{KAllocator, HEAP_ALLOCATOR}, trap::pop_on};
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

const STACK_SIZE: usize = 0x8000;

type TCBlock = Arc<Mutex<TaskControlBlock>>;

pub struct Threads {
    run_threads: Mutex<VecDeque<TCBlock, KAllocator>>,
    current: Mutex<TCBlock>,
}

impl Threads {
    pub fn new() -> Self {
        Threads {
            run_threads: Mutex::new(VecDeque::new_in(KAllocator)),
            current: Mutex::new(Arc::new(Mutex::new(TaskControlBlock::ZERO))),
        }
    }

    pub fn current(&self) -> TCBlock {
        (*self.current.lock()).clone()
    }

    pub fn set_current(&self, ctx: TCBlock) {
        *(self.current.lock()) = ctx;
    }

    pub fn pop_run(&self) -> Option<TCBlock> {
        let ctx = self.run_threads.lock().pop_front();
        if ctx.is_some() {
            let ctx = ctx.unwrap();
            self.set_current(Arc::clone(&ctx));
            return Some(ctx);
        }
        None
    }

    pub fn peek_run(&self) -> Option<TCBlock> {
        let lock = self.run_threads.lock();
        let ctx = lock.front();
        if ctx.is_some() {
            let ctx = ctx.unwrap();
            self.set_current(Arc::clone(ctx));
            return Some(Arc::clone(ctx));
        }
        None
    }

    pub fn push_run(&self, tcx: TCBlock) {
        self.run_threads.lock().push_back(tcx);
    }
}

pub static THREADS: Lazy<Threads> = Lazy::new(|| Threads::new());

pub fn move_run(ctx: TCBlock) {
    ctx.lock().status = TaskStatus::Ready;
    THREADS.push_run(ctx);
}

/// 返回当前 thread
/// 由于 ctx.lock().excute() 执行当前线程会锁住 ctx, 这里强制 unlock
pub fn current_thread() -> TCBlock {
    let lock = THREADS.current();
    if lock.is_locked() {
        unsafe {
            lock.force_unlock();
        }
    }
    lock
}

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
    ctx: LocalContext,
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

pub fn spawn<F>(f: F)
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

    THREADS.push_run(t.clone());
}
