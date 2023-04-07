use core::{
    future::poll_fn,
    task::{Context, Poll},
};

use stdio::log::info;

use crate::*;

pub async fn async_listen(port: u16) -> Result<TcpListener> {
    let sock = sys_sock_create();
    sys_sock_listen(sock, port)
}

fn async_accept_poll(
    cx: &mut Context<'_>,
    listener: &mut TcpListener,
) -> Poll<Result<SocketHandle>> {
    if sys_sock_status(listener.handle).is_establised {
        return Poll::Ready(Ok(listener.accept()?));
    }
    sys_sock_register_send(cx, listener.handle);
    Poll::Pending
}

pub async fn async_accept(listener: &mut TcpListener) -> Result<SocketHandle> {
    poll_fn(|cx| async_accept_poll(cx, listener)).await
}

fn async_recv_poll(cx: &mut Context<'_>, sock: SocketHandle, va: &mut [u8]) -> Poll<Result<usize>> {
    let size = sys_sock_recv(sock, va)?;
    if size == 0 {
        sys_sock_register_recv(cx, sock);
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
        sys_sock_register_send(cx, sock);
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
        sys_sock_register_recv(cx, sock);
        Poll::Pending
    }
}

pub async fn async_connect(sock: SocketHandle, remote_endpoint: IpEndpoint) -> Result<()> {
    match sys_sock_connect(sock, remote_endpoint) {
        Ok(()) => Ok(poll_fn(|cx| async_connect_poll(cx, sock)).await),
        Err(e) => Err(e),
    }
}

fn async_close_poll(cx: &mut Context<'_>, sock: SocketHandle) -> Poll<()> {
    if sys_sock_status(sock).is_open {
        sys_sock_close(sock);
    }

    if sys_sock_status(sock).state == TcpState::Closed {
        sys_sock_release(sock);
        return Poll::Ready(());
    }

    sys_sock_register_send(cx, sock);
    Poll::Pending
}

pub async fn async_sock_close(sock: SocketHandle) {
    poll_fn(|cx| async_close_poll(cx, sock)).await;
}
