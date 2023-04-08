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

const LOOP_SIZE: usize = 100;

// 计算密集型任务
fn fib(n: i32) -> i32 {
    if n <= 1 {
        return n;
    } else {
        return fib(n - 1) + fib(n - 2);
    }
}

async fn echo_client_one(sender: SocketHandle) {
    let mut tx = vec!['x' as u8; 1024];
    let mut rx = vec![0 as u8; 1024];
    let begin: usize = get_time_ms();
    async_send(sender, tx.as_mut_slice())
        .await
        .expect("conn broken");
    async_recv(sender, rx.as_mut_slice())
        .await
        .expect("conn broken");
    let end: usize = get_time_ms();
    info!("CU {}", end - begin);
    IO_TIME.push(end - begin);
    async_sock_close(sender).await;
}

async fn echo_client_basic(sender: SocketHandle) {
    let mut tx = vec!['x' as u8; 1024];
    let mut rx = vec![0 as u8; 1024];
    for i in 0..LOOP_SIZE {
        let begin = get_time_ms();
        async_send(sender, tx.as_mut_slice())
            .await
            .expect("conn broken");
        async_recv(sender, rx.as_mut_slice())
            .await
            .expect("conn broken");
        let end = get_time_ms();
        IO_TIME.push(end - begin);
        info!("{}", end - begin);
    }
    async_sock_close(sender).await;
}

pub async fn app_main() {
    // 创建10个I/O密集型任务和10个计算密集型任务
    let remote_endpoint = IpEndpoint::new(IpAddress::v4(127, 0, 0, 1), 8080);
    let remote_endpoint = IpEndpoint::new(IpAddress::v4(47, 92, 33, 237), 6000);
    // let remote_endpoint = IpEndpoint::new(IpAddress::v4(192, 168, 1, 121), 6000);

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

    // let conn = sys_sock_create();
    // if let Ok(_) = async_connect(conn, remote_endpoint).await {
    //     append_task(echo_client_basic(conn));
    // }

    for _ in 0..LOOP_SIZE {
        let conn = sys_sock_create();
        async_connect(conn, remote_endpoint)
            .await
            .expect("conn broken");
        append_task(echo_client_one(conn));
    }

    let mut vec: Vec<usize> = Vec::new();
    loop {
        if let Ok(i) = IO_TIME.pop() {
            vec.push(i);
        }

        if vec.len() == LOOP_SIZE {
            break;
        }

        async_yield().await;
    }

    info!(
        "{:#?}, {}  average: {}",
        vec,
        vec.len(),
        vec.iter().sum::<usize>() / vec.len()
    );
}
