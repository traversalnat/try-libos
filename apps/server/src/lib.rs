#![no_std]

pub extern crate alloc;

use alloc::vec;

use executor::async_wait;
use futures::{select_biased, FutureExt};
use net::*;
use stdio::{log::info, *};
use thread::append_task;
use core::time::Duration;

async fn echo(sender: SocketHandle) {
    loop {
        let mut rx = vec![0; 1024];
        let mut recv_size = 0;
        info!("try recv");
        if let Some(size) = async_recv(sender, rx.as_mut_slice()).await {
            println!("receive {size} words");
            recv_size = size;
        }
        if let Some(size) = async_send(sender, &mut rx[..recv_size]).await {
            println!("send {size} words");
        }
        let status = sys_sock_status(sender);
        if !status.can_recv && !status.is_active {
            info!("echo stopped");
            sys_sock_close(sender);
            break;
        }
    }
}

pub async fn app_main() {
    let mut listener = async_listen(6000).await.unwrap();
    loop {
        info!("wait for new connection");
        let sender = async_accept(&mut listener).await;
        append_task(async move {
            select_biased! {
                _ = echo(sender).fuse() => (),
                _ = async_wait(Duration::from_secs(10)).fuse() => {
                    info!("{:#?} time out", sender);
                    sys_sock_close(sender);
                },
            };
        });
    }
}
