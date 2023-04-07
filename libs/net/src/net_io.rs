use core::{
    future::poll_fn,
    task::{Context, Poll},
};

use stdio::log::info;

use crate::*;

fn async_accept_poll(cx: &mut Context<'_>, listener: &mut TcpListener) -> Poll<SocketHandle> {
    match listener.accept() {
        Some(handle) => Poll::Ready(handle),
        _ => {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

pub async fn async_listen(port: u16) -> Option<TcpListener> {
    let sock = sys_sock_create();
    sys_sock_listen(sock, port)
}

pub async fn async_accept(listener: &mut TcpListener) -> SocketHandle {
    poll_fn(|cx| async_accept_poll(cx, listener)).await
}

fn async_recv_poll(cx: &mut Context<'_>, sock: SocketHandle, va: &mut [u8]) -> Poll<Result<usize>> {
    let size = sys_sock_recv(sock, va)?;
    if size == 0 {
        // sys_sock_register_recv(cx, sock);
        cx.waker().wake_by_ref();
        Poll::Pending
    } else {
        Poll::Ready(Ok(size))
    }
}

pub async fn async_recv(sock: SocketHandle, va: &mut [u8]) -> Result<usize> {
    poll_fn(|cx| async_recv_poll(cx, sock, va)).await
}

fn async_send_poll(cx: &mut Context<'_>, sock: SocketHandle, va: &mut [u8]) -> Poll<Result<usize>> {
    let size = sys_sock_send(sock, va)?;
    if size == 0 {
        // sys_sock_register_send(cx, sock);
        cx.waker().wake_by_ref();
        Poll::Pending
    } else {
        Poll::Ready(Ok(size))
    }
}

pub async fn async_send(sock: SocketHandle, va: &mut [u8]) -> Result<usize> {
    poll_fn(|cx| async_send_poll(cx, sock, va)).await
}

fn async_connect_poll(cx: &mut Context<'_>, sock: SocketHandle) -> Poll<()> {
    if sys_sock_status(sock).is_establised {
        Poll::Ready(())
    } else {
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

pub async fn async_connect(sock: SocketHandle, remote_endpoint: IpEndpoint) -> Result<()> {
    match sys_sock_connect(sock, remote_endpoint) {
        Ok(()) => Ok(poll_fn(|cx| async_connect_poll(cx, sock)).await),
        Err(e) => Err(e),
    }
}
