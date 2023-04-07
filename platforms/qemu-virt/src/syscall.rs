#![allow(dead_code)]
use crate::{
    tasks::{handle_append_task, spawn, Task, GLOBAL_BOXED_FUTURE},
    thread,
    timer::sleep,
    trap::{pop_on, push_off},
    consts::*,
};
use alloc::boxed::Box;
use core::future::Future;

// 流程：调用 sys_xxx => 调用 syscall 函数并传入系统调用号和参数 => syscall 通过 e_call 函数陷入调度器 (调度器使用 handle_syscall 处理系统调用 => 调度器返回至 e_call 的下一个指令) => syscall 返回系统调用结果

/// handle syscall exception with `syscall_id` and other arguments
pub fn handle_syscall(mut task: Task) -> Option<Task> {
    let mut lock = task.tcb.lock();
    let cx = &mut lock.ctx;
    let syscall_id = cx.x(17);
    let arg0: usize = cx.x(10);
    let _arg1: usize = cx.x(11);
    let _arg2: usize = cx.x(12);

    drop(cx);
    drop(lock);

    // 部分系统调用需要直接用到 task, 但不一定将 task 返回
    // sleep 系统调用会将 task 插入到等待队列中
    use thread::TaskStatus::*;
    let (mut task, result) = match syscall_id {
        SYSCALL_SLEEP => {
            task.set_status(Blocking);
            sleep(task, arg0)
        }
        SYSCALL_GET_TID => {
            let tid = task.tid;
            (Some(task), tid)
        }
        SYSCALL_APPEND_TASK => {
            let (task, ret) = handle_append_task(task);
            (Some(task), ret)
        }
        SYSCALL_YIELD => {
            task.set_status(Blocking);
            (Some(task), 0)
        }
        SYSCALL_EXIT => (None, 0),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    };

    if task.is_some() {
        let mut lock = task.as_mut().unwrap().tcb.lock();
        let cx = &mut lock.ctx;
        *cx.x_mut(10) = result as usize;
    }

    task
}

pub fn syscall(id: usize, args: [usize; 3]) -> usize {
    let mut ret: usize;

    let sstatus = push_off();
    unsafe {
        core::arch::asm!(
            "",
            in("x10") args[0],
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id
        );
        kernel_context::e_call();

        core::arch::asm!(
            "",
            out("x10") ret
        );
    };

    pop_on(sstatus);

    ret
}

pub fn sys_sleep(sleep_ms: usize) -> usize {
    syscall(SYSCALL_SLEEP, [sleep_ms, 0, 0])
}

pub fn sys_get_tid() -> usize {
    let tid = syscall(SYSCALL_GET_TID, [0, 0, 0]);
    tid
}

pub fn sys_spawn<F>(f: F, is_io: bool) -> usize
where
    F: Future<Output = ()> + Send + 'static,
{
    let sstatus = push_off();
    let ret = spawn(f, is_io);
    pop_on(sstatus);
    ret
}

pub fn sys_append_task<F>(future: F) -> usize
where
    F: Future<Output = ()> + Send + 'static,
{
    use crate::mm::KAllocator;
    drop(core::mem::replace(
        &mut *GLOBAL_BOXED_FUTURE.lock(),
        Box::pin_in(future, KAllocator),
    ));
    syscall(SYSCALL_APPEND_TASK, [0, 0, 0])
}

pub fn sys_yield() {
    syscall(SYSCALL_YIELD, [0, 0, 0]);
}

pub fn sys_exit() {
    syscall(SYSCALL_EXIT, [0, 0, 0]);
}
