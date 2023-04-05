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
        let mut recv_size = 0;
        if let Some(size) = async_recv(sender, rx.as_mut_slice()).await {
            async_send(sender, &mut rx[..size]).await;
        } else {
            info!("echo stop");
            break;
        }
    }
}

pub async fn app_main() {
    let mut listener = async_listen(6000).await.unwrap();
    info!("wait for new connection");
    loop {
        let sender = async_accept(&mut listener).await;
        info!("new connection");
        append_task(echo(sender));
    }
}
