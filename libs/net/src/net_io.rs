use core::{
    future::poll_fn,
    task::{Context, Poll},
};

use stdio::log::info;

use crate::*;

fn async_accept_poll(_cx: &mut Context<'_>, listener: &mut TcpListener) -> Poll<SocketHandle> {
    match listener.accept() {
        Some(handle) => Poll::Ready(handle),
        _ => Poll::Pending,
    }
}

pub async fn async_listen(port: u16) -> Option<TcpListener> {
    let sock = sys_sock_create();
    sys_sock_listen(sock, port)
}

pub async fn async_accept(listener: &mut TcpListener) -> SocketHandle {
    poll_fn(|cx| async_accept_poll(cx, listener)).await
}

fn async_recv_poll(
    _cx: &mut Context<'_>,
    sock: SocketHandle,
    va: &mut [u8],
) -> Poll<Option<usize>> {
    if sys_sock_status(sock).can_recv {
        Poll::Ready(sys_sock_recv(sock, va))
    } else {
        Poll::Pending
    }
}

pub async fn async_recv(sock: SocketHandle, va: &mut [u8]) -> Option<usize> {
    poll_fn(|cx| async_recv_poll(cx, sock, va)).await
}

fn async_send_poll(
    _cx: &mut Context<'_>,
    sock: SocketHandle,
    va: &mut [u8],
) -> Poll<Option<usize>> {
    if sys_sock_status(sock).can_send {
        Poll::Ready(sys_sock_send(sock, va))
    } else {
        Poll::Pending
    }
}

pub async fn async_send(sock: SocketHandle, va: &mut [u8]) -> Option<usize> {
    poll_fn(|cx| async_send_poll(cx, sock, va)).await
}

fn async_connect_poll(
    _cx: &mut Context<'_>,
    sock: SocketHandle,
) -> Poll<()> {
    if sys_sock_status(sock).is_establised {
        Poll::Ready(())
    } else {
        Poll::Pending
    }
}

pub async fn async_connect(sock: SocketHandle, remote_endpoint: IpEndpoint) -> Result<(), String> {
    match sys_sock_connect(sock, remote_endpoint) {
        Ok(()) => Ok(poll_fn(|cx| async_connect_poll(cx, sock)).await),
        Err(e) => Err(e),
    }
}
