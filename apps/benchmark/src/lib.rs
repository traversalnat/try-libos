#![no_std]

use alloc::vec;
use thread::{append_task, spawn};

use mem::*;
use net::*;
use stdio::log::info;
use timer::get_time_ms;

// 计算密集型任务
fn fib(n: i32) -> i32 {
    if n <= 1 {
        return n;
    } else {
        return fib(n - 1) + fib(n - 2);
    }
}

// IO 密集型任务
async fn echo_client(sender: SocketHandle) {
    let mut tx = vec!['x' as u8; 1200];
    let mut rx = vec![0 as u8; 1200];
    for _i in 0..10 {
        async_send(sender, unsafe { tx.as_mut_slice() }).await;
        async_recv(sender, rx.as_mut_slice()).await;
        if !sys_sock_status(sender).is_active {
            break;
        }
    }
    sys_sock_close(sender);
}

async fn echo_client_one(sender: SocketHandle) {
    let mut tx = vec!['x' as u8; 1200];
    let mut rx = vec![0 as u8; 1200];
    let begin: usize = get_time_ms();
    async_send(sender, unsafe { tx.as_mut_slice() }).await;
    async_recv(sender, rx.as_mut_slice()).await;
    let end: usize = get_time_ms();
    info!("CU {}", end - begin);
    info!("END {end}");
    sys_sock_close(sender);
}

async fn echo_client_basic(_index: usize, sender: SocketHandle) {
    let mut tx = vec!['x' as u8; 1200];
    let mut rx = vec![0 as u8; 1200];
    let mut begin: usize = 0;
    let end: usize = get_time_ms();
    let mut old_end: usize = end;
    for i in 0..10 {
        begin = get_time_ms();
        async_send(sender, unsafe { tx.as_mut_slice() }).await;
        async_recv(sender, rx.as_mut_slice()).await;
        old_end = end;
        let end = get_time_ms();
        // info!("wait CU{i} {}", begin - old_end);
        info!("CU{i}: {}", end - begin);
        if !sys_sock_status(sender).is_active {
            break;
        }
    }
    sys_sock_close(sender);
}

pub async fn app_main() {
    // 创建10个I/O密集型任务和10个计算密集型任务

    let remote_endpoint = IpEndpoint::new(IpAddress::v4(47, 92, 33, 237), 6000);

    let begin = get_time_ms();
    info!("ALL {begin}");
    // 10个计时I/O密集型任务组成的一个线程
    spawn(async move {
        for _i in 0..10 {
            let conn = sys_sock_create();
            if let Ok(_) = async_connect(conn, remote_endpoint).await {
                append_task(echo_client_one(conn));
            }
        }
    });

    for i in 0..10 {
        spawn(async move {
            let begin = get_time_ms();
            fib(37);
            let end = get_time_ms();
            info!("EU{i}: {}", end - begin);
            info!("END {end}");
        });
    }
}
