#![no_std]

pub extern crate alloc;

use alloc::vec;

use net::*;
use stdio::{log::info, *};
use thread::append_task;

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
        if !sys_sock_status(sender).is_active && !sys_sock_status(sender).can_recv {
            info!("echo stopped");
            break;
        }
    }
}

pub async fn app_main() {
    let mut listener = async_listen(6000).await.unwrap();
    loop {
        info!("wait for new connection");
        let sender = async_accept(&mut listener).await;
        append_task(echo(sender));
    }
}
