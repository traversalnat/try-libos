#### libs/net 模块：
    要求使用 PhyNet , MACADDR 进行初始化, 并且使用线程在后台定时 poll。该模块来自 cs3210-rustos 实验lab5, 实现该实验后得到该模块，现有的函数仅支持 tcp 连接, 经实验可以与有独立IP地址外部服务器上的echo server 通信。
    模块在后台会自动发起 dhcp 请求, 目前在 qemu-virt 上通过 e1000 与 qemu 自带 dhcp 服务器通信可以自动获取 ip 地址。
    在 guest 平台层实验时，通过直接使用 macOS 的 en0 数据链路层收发数据包, 也可以获取到 dhcp 服务器的 ip 地址并与外部网络通信，但存在问题：宿主操作系统无法与 guest 通信, 并且 ping 不通，查看网络数据包发现似乎宿主操作系统似乎没有发送数据包(或许是单网卡双 mac 地址存在逻辑问题)。

PhyNet 要求实现 PhyNet Trait
```rust
pub trait PhyNet: Sync {
    fn receive(&self, buf: &mut [u8]) -> usize;
    fn transmit(&self, buf: &mut [u8]);
    fn can_send(&self) -> bool;
    fn can_recv(&self) -> bool;
}
```

```rust
fn init_ethernet() {
    net::init(&PhyNet, &MACADDR);
    // 网络栈需要定时poll
    schedule_with_delay(Duration::from_millis(100), move || {
        let val = rdtime() as i64;
        net::ETHERNET.poll(net::Instant::from_millis(val));
        // smoltcp 建议使用 poll_delay 来确认下一次的 poll 时间，这里是为了加快实验速度
    });
}
```

使用:
```rust
    let receiver = sys_sock_create();
    let remote_endpoint = IpEndpoint::new(IpAddress::v4(192, 168, 0, 2), 6000);
    if let Ok(_) = sys_sock_connect(receiver, remote_endpoint) {};
    println!("connected");

    unsafe {
        let mut tx: String = "hello, world".to_owned();
        let mut rx = vec![0 as u8; tx.len()];

        println!("read status");
        while !sys_sock_status(receiver).can_send {}
        println!("sending");
        if let Some(size) = sys_sock_send(receiver, tx.as_bytes_mut()) {
            println!("send {size} words");
        }

        while !sys_sock_status(receiver).can_recv {}
        println!("recving");
        if let Some(size) = sys_sock_recv(receiver, rx.as_mut_slice()) {
            println!("receive {size} words");
        }

        sys_sock_close(receiver);
    };

```

#### common/executor 模块（未完成）
    目前 common/executor 模块只是一个单线程异步任务运行时, 借助async_task 和 futures 提供的工具实现, 稍微改造可得到具有线程池的异步任务运行时，但是考虑到 no_std 环境下没有标准线程创建函数，就此作罢。
    已经独立了一个模块 https://github.com/traversalnat/nostd_runtime.git

使用
```rust
use executor::{spawn, run, block_on, join, async_yield};

fn main() {
    let handle_1 = spawn(async {
        loop {
            println!("AAAAAA");
            async_yield().await;
        }
    });

    let handle_2 = spawn(async {
        loop {
            println!("BBBBBB");
            async_yield().await;
        }
    });

    block_on(async {
        join!(handle_1, handle_2);
    });
}
```
