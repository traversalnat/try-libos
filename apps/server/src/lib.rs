#![no_std]

use alloc::{borrow::ToOwned, string::String};
use mem::*;
use net::*;
use stdio::*;

pub fn app_main() {
    let sender = sys_sock_create();
    let receiver = sys_sock_create();
    let remote_endpoint = IpEndpoint::new(IpAddress::v4(127, 0, 0, 1), 1234);
    sys_sock_listen(sender, 1234).unwrap();
    if let Ok(_) = sys_sock_connect(receiver, remote_endpoint) {};

    unsafe {
        let mut tx: String = "hello, world".to_owned();
        let mut rx = [0 as u8; 12];

        while !sys_sock_status(sender).can_send {}
        if let Some(size) = sys_sock_send(sender, tx.as_bytes_mut()) {
            println!("send {size} words");
        }

        while !sys_sock_status(receiver).can_recv {}
        if let Some(size) = sys_sock_recv(receiver, &mut rx) {
            println!("receive {size} words");
        }
        assert_eq!("hello, world", core::str::from_utf8_unchecked(&rx));
        println!("{}", core::str::from_utf8_unchecked(&rx));
    };
}
