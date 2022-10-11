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
    println!("before connect");
    match sys_sock_connect(receiver, remote_endpoint) {
        Err(e) => {
            println!("error {e}");
        },
        _ => {println!("no error")},
    };
    println!("after connect");

    unsafe {
        let mut tx: String = "hello, world".to_owned();
        let mut rx = [0 as u8; 12];

        loop {
            sys_sock_poll();
            sys_sock_send(sender, tx.as_bytes_mut()).unwrap();
            sys_sock_recv(receiver, &mut rx).unwrap();
            print!("run to here");
            assert_eq!("hello, world", core::str::from_utf8_unchecked(&rx));
            println!("{}", core::str::from_utf8_unchecked(&rx));
        }
    };
}
