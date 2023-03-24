#![no_std]

pub extern crate alloc;

use alloc::vec;
use executor::async_block_on;
use thread::append_task;

use net::*;
use stdio::{log::info, *};

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
        if !sys_sock_status(sender).is_active {
            info!("echo stopped");
            break;
        }
    }
}

pub fn app_main() {
    async_block_on(async move {
        let mut listener = async_listen(6000).await.unwrap();
        loop {
            let sender = async_accept(&mut listener).await;
            append_task(echo(sender));
        }
    });
}
