//! 内核上下文控制。

#![no_std]
#![feature(naked_functions, asm_const)]
#![deny(warnings, missing_docs)]
#![feature(asm_sym)]

/// 不同地址空间的上下文控制。
#[cfg(feature = "foreign")]
pub mod foreign;

/// 线程上下文。
#[derive(Clone)]
#[repr(C)]
pub struct LocalContext {
    sctx: usize,    // sscratch, 用于保存调度上下文 sp 
    x: [usize; 31], // x1-x31, 其中 x2 为线程上下文 sp
    sepc: usize,
    /// 是否以特权态切换。
    pub supervisor: bool,
    /// 线程中断是否开启。
    pub interrupt: bool,
}

impl LocalContext {
    /// 创建空白上下文。
    #[inline]
    pub const fn empty() -> Self {
        Self {
            sctx: 0,
            x: [0; 31],
            supervisor: false,
            interrupt: false,
            sepc: 0,
        }
    }

    /// 初始化指定入口的用户上下文。
    ///
    /// 切换到用户态时会打开内核中断。
    #[inline]
    pub const fn user(pc: usize) -> Self {
        Self {
            sctx: 0,
            x: [0; 31],
            supervisor: false,
            interrupt: true,
            sepc: pc,
        }
    }

    /// 初始化指定入口的内核上下文。
    #[inline]
    pub const fn thread(pc: usize, interrupt: bool) -> Self {
        Self {
            sctx: 0,
            x: [0; 31], // x1 - x31; x0 是 zero 寄存器无需保安村
            supervisor: true,
            interrupt,
            sepc: pc,
        }
    }

    /// 读取用户通用寄存器。
    #[inline]
    pub fn x(&self, n: usize) -> usize {
        self.x[n - 1]
    }

    /// 修改用户通用寄存器。
    #[inline]
    pub fn x_mut(&mut self, n: usize) -> &mut usize {
        &mut self.x[n - 1]
    }

    /// 读取用户参数寄存器。
    #[inline]
    pub fn a(&self, n: usize) -> usize {
        self.x(n + 10)
    }

    /// 修改用户参数寄存器。
    #[inline]
    pub fn a_mut(&mut self, n: usize) -> &mut usize {
        self.x_mut(n + 10)
    }

    /// 读取用户栈指针。
    #[inline]
    pub fn ra(&self) -> usize {
        self.x(1)
    }

    /// 读取用户栈指针。
    #[inline]
    pub fn sp(&self) -> usize {
        self.x(2)
    }

    /// 修改用户栈指针。
    #[inline]
    pub fn sp_mut(&mut self) -> &mut usize {
        self.x_mut(2)
    }

    /// 当前上下文的 pc。
    #[inline]
    pub fn pc(&self) -> usize {
        self.sepc
    }

    /// 修改上下文的 pc。
    #[inline]
    pub fn pc_mut(&mut self) -> &mut usize {
        &mut self.sepc
    }

    /// 将 pc 移至下一条指令。
    ///
    /// # Notice
    ///
    /// 假设这一条指令不是压缩版本。
    #[inline]
    pub fn move_next(&mut self) {
        self.sepc = self.sepc.wrapping_add(4);
    }

    /// 执行此线程，并返回 `sstatus`。
    ///
    /// # Safety
    ///
    /// 将修改 `sscratch`、`sepc`、`sstatus` 和 `stvec`。
    #[inline]
    pub unsafe fn execute(&mut self) -> usize {
        let mut sstatus = build_sstatus(self.supervisor, self.interrupt);
        core::arch::asm!(
            "   csrrw {sscratch}, sscratch, {sscratch}
                csrw  sepc    , {sepc}
                csrw  sstatus , {sstatus}
                addi  sp, sp, -8
                sd    ra, (sp)
                call  {execute_naked}
                ld    ra, (sp)
                addi  sp, sp,  8
                csrw  sscratch, {sscratch}
                csrr  {sepc}   , sepc
                csrr  {sstatus}, sstatus
            ",
            sscratch      = in       (reg) self,    // self.sctx init with 0
            sepc          = inlateout(reg) self.sepc,
            sstatus       = inlateout(reg) sstatus,
            execute_naked = sym execute_naked,
        );
        sstatus
    }

    /// 主动让出 CPU
    pub unsafe fn execute_yield(&self) {
        core::arch::asm!("ebreak");
    }
}

#[inline]
fn build_sstatus(supervisor: bool, interrupt: bool) -> usize {
    let mut sstatus: usize;
    // 只是读 sstatus，安全的
    unsafe { core::arch::asm!("csrr {}, sstatus", out(reg) sstatus) };
    const PREVILEGE_BIT: usize = 1 << 8;
    const INTERRUPT_BIT: usize = 1 << 5;
    match supervisor {
        false => sstatus &= !PREVILEGE_BIT,
        true => sstatus |= PREVILEGE_BIT,
    }
    match interrupt {
        false => sstatus &= !INTERRUPT_BIT,
        true => sstatus |= INTERRUPT_BIT,
    }
    sstatus
}

core::arch::global_asm!(include_str!("execute.S"));

extern "C" {
    /// Switch to the context of `next_task_cx_ptr`, saving the current context
    /// in `current_task_cx_ptr`.
    pub fn execute_naked();
}
