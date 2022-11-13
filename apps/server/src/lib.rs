#![no_std]

mod net_io;

pub extern crate alloc;

use alloc::{vec};
use executor::{async_spawn, async_block_on};

use net::*;
use net_io::{async_accept, async_recv, async_send};
use spin::Lazy;
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
    let sender = sys_sock_create();
    let mut listener = sys_sock_listen(sender, 6000).unwrap();
    async_block_on(async move {
        loop {
            let sender = async_accept(&mut listener).await;
            async_spawn(echo(sender));
        }
    });
}
