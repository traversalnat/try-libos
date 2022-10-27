#![no_std]

use alloc::{borrow::ToOwned, string::String, vec};
use mem::*;
use net::*;
use stdio::*;

pub fn app_main() {
    let sender = sys_sock_create();
    // TODO: 目前只能接受一个连接，另一个连接在接入时系统会崩溃
    sys_sock_listen(sender, 6000);
    println!("listening");

    unsafe {
        let mut rx = vec![0; 1024];

        loop {
            println!("read status");
            while !sys_sock_status(sender).can_recv {}
            println!("recving");
            let mut recv_size = 0;
            if let Some(size) = sys_sock_recv(sender, rx.as_mut_slice()) {
                println!("receive {size} words");
                recv_size = size;
            }

            while !sys_sock_status(sender).can_send {}
            println!("sending");
            if let Some(size) = sys_sock_send(sender, &mut rx[..recv_size]) {
                println!("send {size} words");
            }
        }

        sys_sock_close(sender);
    };
}
