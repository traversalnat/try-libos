// 内核堆内存
pub const MM_SIZE: usize = 32 << 20;
// 物理内存容量
pub const MEMORY: usize = 128 << 20 - 1;

// 单个线程可容纳协程数量
pub const TASKNUM: usize = 300;

// SYSCALL
pub const SYSCALL_SLEEP: usize = 101;
pub const SYSCALL_GET_TID: usize = 102;
pub const SYSCALL_APPEND_TASK: usize = 103;
pub const SYSCALL_YIELD: usize = 104;
pub const SYSCALL_EXIT: usize = 105;

// STACK_SIZE FOR THREAD
pub const STACK_SIZE: usize = 0x8000;

// TIMER
pub const CLOCK_FREQ: usize = 12500000;
pub const TICKS_PER_SEC: usize = 100;
pub const MILLI_PER_SEC: usize = 1_000;
pub const MICRO_PER_SEC: usize = 1_000_000;

// virt
pub const MACADDR: [u8; 6] = [0x12, 0x13, 0x89, 0x89, 0xdf, 0x53];

// IO coroutine executor's tid
pub const IO_TASK_TID: usize = 0;
