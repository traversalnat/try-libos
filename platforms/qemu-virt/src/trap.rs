#![allow(unused)]
const SSTATUS_SIE: usize = 1 << 1; // Supervisor Interrupt Enable

/// tp reg, core's hartid (core number)
#[inline]
fn r_tp() -> usize {
    let ret: usize;
    unsafe {
        core::arch::asm!("mv {}, tp",out(reg)ret);
    }
    ret
}

/// need read mhartid to tp in machine mode
#[inline]
fn r_mhartid() -> usize {
    let ret: usize;
    unsafe {
        core::arch::asm!("csrr {}, mhartid",out(reg)ret);
    }
    ret
}

fn r_sstatus() -> usize {
    let mut sstatus: usize;
    unsafe { core::arch::asm!("csrr {}, sstatus", out(reg) sstatus) };
    sstatus
}

fn w_sstatus(sstatus: usize) {
    unsafe { core::arch::asm!("csrw sstatus, {}", in(reg) sstatus) };
}

pub fn intr_on() {
    w_sstatus(r_sstatus() | SSTATUS_SIE);
}

pub fn intr_off() {
    w_sstatus(r_sstatus() & !SSTATUS_SIE);
}

/// 恢复中断状态至 sstatus
pub fn pop_on(sstatus: usize) {
    w_sstatus(sstatus);
}

/// 关闭中断，返回关闭之前中断状态
pub fn push_off() -> usize {
    let sstatus = r_sstatus();
    intr_off();
    sstatus
}

/// tp 需要在 M 态就从 hartid 寄存器保存至每一个CPU 的 tp 寄存器中
/// cpu_id, 目前只有一个 CPU，返回0
pub fn cpuid() -> usize {
    0
}
