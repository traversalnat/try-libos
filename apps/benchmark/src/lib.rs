#![no_std]
#![allow(dead_code)]
#![macro_use]

use core::time::Duration;

use alloc::{vec, vec::Vec};

use alloc::boxed::Box;
use crossbeam_queue::ArrayQueue;
use executor::{
    async_timeout, async_wait_some, async_yield,
    futures::{future::try_join, FutureExt},
    select_biased,
};
use futures::{
    future::{select, Select},
    pin_mut, Future,
};
use spin::Lazy;
use thread::{append_task, spawn};

use mem::*;
use net::*;
use stdio::log::info;
use timer::get_time_ms;

static IO_TIME: Lazy<ArrayQueue<usize>> = Lazy::new(|| ArrayQueue::new(120));

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
    let mut tx = vec!['x' as u8; 1024];
    let mut rx = vec![0 as u8; 1024];
    for _i in 0..10 {
        async_send(sender, tx.as_mut_slice()).await;
        async_recv(sender, rx.as_mut_slice()).await;
        if !sys_sock_status(sender).is_active {
            break;
        }
    }
    sys_sock_close(sender);
}

async fn echo_client_one(sender: SocketHandle) {
    let mut tx = vec!['x' as u8; 1024];
    let mut rx = vec![0 as u8; 1024];
    let begin: usize = get_time_ms();
    async_send(sender, tx.as_mut_slice()).await;
    async_recv(sender, rx.as_mut_slice()).await;
    let end: usize = get_time_ms();
    info!("CU {}", end - begin);
    // info!("END {end}");
    IO_TIME.push(end - begin);
    sys_sock_close(sender);
}

async fn echo_client_basic(_index: usize, sender: SocketHandle) {
    let mut tx = vec!['x' as u8; 1024];
    let mut rx = vec![0 as u8; 1024];
    let mut begin: usize;
    let end: usize = get_time_ms();
    let mut old_end: usize;
    for i in 0..10 {
        begin = get_time_ms();
        async_send(sender, tx.as_mut_slice()).await;
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

// pub async fn select_one<A, B>(fut1: A, fut2: B) -> Select<A, B>
// where
//     A: Future,
//     B: Future,
// {
//     select(Box::pin(fut1), Box::pin(fut2))
// }

pub async fn app_main() {
    // 创建10个I/O密集型任务和10个计算密集型任务
    let remote_endpoint = IpEndpoint::new(IpAddress::v4(47, 92, 33, 237), 6000);
    const LOOP_SIZE: usize = 100;

    let begin = get_time_ms();
    info!("ALL {begin}");

    for _ in 0..100 {
        let _tid = spawn(
            async move {
                fib(37);
            },
            false,
        );
    }

    // 10个计时I/O密集型任务组成的一个线程
    for _ in 0..LOOP_SIZE {
        let conn = sys_sock_create();
        if let Ok(_) = async_connect(conn, remote_endpoint).await {
            append_task(echo_client_one(conn));
        }
    }

    match async_timeout(
        async_wait_some(|| IO_TIME.len() == LOOP_SIZE),
        Duration::from_secs(5),
    )
    .await
    {
        _ => {
            let mut vec: Vec<usize> = Vec::new();
            while let Ok(i) = IO_TIME.pop() {
                vec.push(i);
            }

            info!("{:#?}, {}", vec, vec.len());
        }
    }
}
