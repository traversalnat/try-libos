use core::panic;

use pnet::datalink::Channel::Ethernet;
use pnet::datalink::{self, NetworkInterface};
use pnet::packet::ethernet::{EthernetPacket, MutableEthernetPacket};
use pnet::packet::{MutablePacket, Packet};
use pnet::packet::ipv4::MutableIpv4Packet;
use pnet::packet::udp::MutableUdpPacket;
use pnet::datalink::DataLinkSender;
use pnet::datalink::DataLinkReceiver;

pub struct EthDevice {
    tx: Box<dyn DataLinkSender>,
    rx: Box<dyn DataLinkReceiver>,
}

impl EthDevice {
    pub fn new() -> Self {
        let interface_name = "lo0";
        let interface_names_match = |iface: &NetworkInterface| iface.name == interface_name;
        let interfaces = datalink::interfaces();
        let interface = interfaces
            .into_iter()
            .filter(interface_names_match)
            .next()
            .unwrap();
        // if !interface.is_loopback() {
        //     panic!("not loopback interface");
        // }
        let (tx, rx) = match datalink::channel(&interface, Default::default()) {
            Ok(Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unhandled channel type"),
            Err(e) => panic!(
                "An error occurred when creating the datalink channel: {}",
                e
            ),
        };

        Self {
            tx,
            rx
        }
    }

    pub fn recv(&mut self, buf: &mut [u8]) -> usize {
        match self.rx.next() {
            Ok(packet) => {
                // TODO 修改 packet 信息 (lo 设备暂时不需要)
                let min_len = core::cmp::min(buf.len(), packet.len());
                unsafe {
                    core::ptr::copy(packet.as_ptr(), buf.as_mut_ptr(), min_len);
                }
                let mut ethernet_packet = MutableEthernetPacket::new(buf).unwrap();
                println!("eth {:#?}", ethernet_packet);
                let mut ip_packet = MutableIpv4Packet::new(ethernet_packet.payload_mut()).unwrap();
                println!("recv {:#?}", ip_packet);
                let mut udp_packet = MutableUdpPacket::new(ip_packet.payload_mut()).unwrap();
                println!("port {} {}", udp_packet.get_destination(), udp_packet.get_source());

                min_len
            }
            Err(e) =>  {
                panic!("{e}");
            }
        }
    }

    pub fn send(&mut self, buf: &mut [u8]) {
        println!("send function");
        let mut ethernet_packet = MutableEthernetPacket::new(buf).unwrap();
        let mut ip_packet = MutableIpv4Packet::new(ethernet_packet.payload_mut()).unwrap();
        println!("send {:#?}", ip_packet);
        self.tx.send_to(buf, None);
    }
}
