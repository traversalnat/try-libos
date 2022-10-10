#![no_std]

mod ethernet;

pub use ethernet::GlobalEthernetDriver;

use spin::Once;

/// 这个接口定义了网络物理层receive, transmit
pub trait PhyNet: Sync {
    // 将数据全部发送出去
    fn receive(&self, buf: &mut [u8]) -> usize;
    fn transmit(&self, buf: &mut [u8]);
}

// 网络物理设备
static PHYNET: Once<&'static dyn PhyNet> = Once::new();

pub static ETHERNET: GlobalEthernetDriver = GlobalEthernetDriver::uninitialized();

/// 主要是给 obj 确认使用哪个 platform 提供的函数来注入 PhyNet
pub fn init(net: &'static dyn PhyNet) {
    // TODO 使用 PHYNET 提供的发送、接收 raw packet 的方法重写 Loopback 设备
    PHYNET.call_once(|| net);
    ETHERNET.initialize();
}

// TODO 提供与 socket 交互的 api
