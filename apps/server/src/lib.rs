#![no_std]

pub extern crate alloc;

use alloc::{boxed::Box, vec};

use core::time::Duration;
use executor::async_wait;
use net::*;
use stdio::{log::info, *};
use thread::append_task;

async fn echo(sender: SocketHandle) {
    let mut rx = vec![0; 1024];
    loop {
        info!("try recv");
        match async_recv(sender, rx.as_mut_slice()).await {
            Ok(size) => {
                async_send(sender, &mut rx[..size]).await;
                info!("send {size}");
            }
            Err(e) => {
                async_sock_close(sender).await;
                info!("echo stop {:#?}", e);
                break;
            }
        }
    }
}

pub async fn app_main() {
    let mut listener = async_listen(6000).await.unwrap();
    loop {
        info!("wait for new connection");
        let sender = async_accept(&mut listener).await.expect("accept error");
        info!("new connection");
        append_task(echo(sender));
    }
}
