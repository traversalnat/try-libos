use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use net::*;

pub struct Recv {
    sock: SocketHandle,
}

impl Recv {
    pub fn new(sock: SocketHandle) -> Self {
        Self {
            sock
        }
    }
}

impl Future for Recv {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if sys_sock_status(self.sock).can_recv {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}


pub struct Send {
    sock: SocketHandle,
}

impl Send {
    pub fn new(sock: SocketHandle) -> Self {
        Self {
            sock
        }
    }
}

impl Future for Send {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if sys_sock_status(self.sock).can_send {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub async fn async_recv(sock: SocketHandle, va: &mut [u8]) -> Option<usize> {
    Recv::new(sock).await;
    sys_sock_recv(sock, va)
}

pub async fn async_send(sock: SocketHandle, va: &mut [u8]) -> Option<usize> {
    Send::new(sock).await;
    sys_sock_send(sock, va)
}
