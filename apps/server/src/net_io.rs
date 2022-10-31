use core::future::{Future, poll_fn};
use core::pin::Pin;
use core::task::{Context, Poll};

use net::*;

pub fn async_recv_poll(cx: &mut Context<'_>, sock: SocketHandle, va: &mut [u8]) -> Poll<Option<usize>> {
    if sys_sock_status(sock).can_recv {
        Poll::Ready(sys_sock_recv(sock, va))
    } else {
        Poll::Pending
    }
}

pub async fn async_recv(sock: SocketHandle, va: &mut [u8]) -> Option<usize> {
    poll_fn(|cx| async_recv_poll(cx, sock, va)).await
}

pub fn async_send_poll(cx: &mut Context<'_>, sock: SocketHandle, va: &mut [u8]) -> Poll<Option<usize>> {
    if sys_sock_status(sock).can_send {
        Poll::Ready(sys_sock_send(sock, va))
    } else {
        Poll::Pending
    }
}

pub async fn async_send(sock: SocketHandle, va: &mut [u8]) -> Option<usize> {
    poll_fn(|cx| async_send_poll(cx, sock, va)).await
}

