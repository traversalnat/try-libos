use bimap::BiMap;
use core::panic;
use std::collections::HashMap;
use std::hash::Hash;

use pnet::datalink::Channel::Ethernet;
use pnet::datalink::DataLinkReceiver;
use pnet::datalink::DataLinkSender;
use pnet::datalink::{self, NetworkInterface};
use pnet::packet::ethernet::{EthernetPacket, MutableEthernetPacket};
use pnet::packet::ipv4::MutableIpv4Packet;
use pnet::packet::udp::MutableUdpPacket;
use pnet::packet::{MutablePacket, Packet};

struct MacIP {
    pub dmac: [u8; 6],
    pub smac: [u8; 6],
    pub sip: [u8; 4],
    pub dip: [u8; 4],
}

impl MacIP {
    pub fn new() -> Self {
        let dmac: [u8; 6] = [0; 6];
        let smac: [u8; 6] = [0; 6];
        let sip: [u8; 4] = [0; 4];
        let dip: [u8; 4] = [0; 4];

        Self {
            dmac,
            smac,
            sip,
            dip,
        }
    }

    pub fn new_from_buf(buf: &[u8]) -> Self {
        let mut dmac: [u8; 6] = [0; 6];
        let mut smac: [u8; 6] = [0; 6];
        let mut sip: [u8; 4] = [0; 4];
        let mut dip: [u8; 4] = [0; 4];
        dmac.copy_from_slice(&buf[DMAC_BEGIN..=DMAC_END]);
        smac.copy_from_slice(&buf[SMAC_BEGIN..=SMAC_END]);
        sip.copy_from_slice(&buf[SIP_BEGIN..=SIP_END]);
        dip.copy_from_slice(&buf[DIP_BEGIN..=DIP_END]);

        Self {
            dmac,
            smac,
            sip,
            dip,
        }
    }
}

pub struct EthDevice {
    tx: Box<dyn DataLinkSender>,
    rx: Box<dyn DataLinkReceiver>,
    map: BiMap<[u8; 2], [u8; 2]>,
    mac_ip: MacIP,
}

const TARGETMAC: [u8; 6] = [0xf4, 0x5c, 0x89, 0x89, 0xdf, 0x53];
const MACADDR: [u8; 6] = [0x12, 0x12, 0x12, 0x12, 0x12, 0x12];
const IPADDR: [u8; 4] = [10, 42, 117, 181];
// const TARGETIP: [u8; 4] = [127, 0, 0, 1];
const DMAC_BEGIN: usize = 0;
const DMAC_END: usize = 5;
const SMAC_BEGIN: usize = 6;
const SMAC_END: usize = 11;
const IP_BEGIN: usize = 14;
const IP_END: usize = 33;
const IPSUM_BEGIN: usize = 24;
const IPSUM_END: usize = 25;
const SIP_BEGIN: usize = 26;
const SIP_END: usize = 29;
const DIP_BEGIN: usize = 30;
const DIP_END: usize = 33;
const SPORT_BEGIN: usize = 34;
const SPORT_END: usize = 35;
const DPORT_BEGIN: usize = 37;
const DPORT_END: usize = 38;

impl EthDevice {
    pub fn new() -> Self {
        let interface_name = "en0";
        let interface_names_match = |iface: &NetworkInterface| iface.name == interface_name;
        let interfaces = datalink::interfaces();
        let interface = interfaces
            .into_iter()
            .filter(interface_names_match)
            .next()
            .unwrap();
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
            rx,
            map: BiMap::new(),
            mac_ip: MacIP::new(),
        }
    }

    pub fn recv(&mut self, buf: &mut [u8]) -> usize {
        match self.rx.next() {
            Ok(packet) => {
                let min_len = core::cmp::min(buf.len(), packet.len());
                buf[..min_len].copy_from_slice(&packet[..min_len]);
                // port
                let mut port = [0; 2];
                port.copy_from_slice(&buf[SPORT_BEGIN..=SPORT_END]);
                match self.map.get_by_right(&port) {
                    Some(vport) => {
                        buf[SPORT_BEGIN..=SPORT_END].copy_from_slice(vport);
                    }
                    None => {
                        return 0;
                    }
                }
                min_len
            }
            Err(e) => {
                panic!("{e}");
            }
        }
    }

    // 大端序 port
    fn get_available_port(vport: &[u8; 2]) -> [u8; 2] {
        let vport: u16 = (vport[0] as u16) << 8 | (vport[0] as u16);
        let port = match std::net::TcpListener::bind(format!("0.0.0.0:{}", vport)) {
            Ok(_) => vport,
            _ => std::net::TcpListener::bind("0.0.0.0:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port(),
        };
        println!("used port {}", port);
        [(port >> 8) as u8, port as u8]
    }

    fn ip_cksum(addr: &[u8]) -> u16 {
        let mut len = addr.len();
        let mut i = 0;
        let mut sum: u32 = 0;
        while len > 1 {
            let number = ((addr[i] as u16) << 8) | addr[i+1] as u16;
            sum += number as u32;
            i += 2;
            len -= 2;
        }

        if len == 1 {
            sum += addr[i] as u32;
        }
        
        sum = (sum & 0xffff) + (sum >> 16);
        sum += (sum >> 16);
        !sum as u16
    }

    pub fn send(&mut self, buf: &mut [u8]) {
        self.mac_ip = MacIP::new_from_buf(buf);
        // mac addr
        buf[SMAC_BEGIN..=SMAC_END].copy_from_slice(&TARGETMAC);
        buf[SIP_BEGIN..=SIP_END].copy_from_slice(&IPADDR);
        // checksum
        buf[IPSUM_BEGIN..=IPSUM_END].fill(0);
        let checksum = Self::ip_cksum(&buf[IP_BEGIN..=IP_END]);
        buf[IPSUM_BEGIN..=IPSUM_END].copy_from_slice(&[(checksum >> 8)as u8, checksum as u8]);

        // port
        let mut vport = [0; 2];
        vport.copy_from_slice(&buf[SPORT_BEGIN..=SPORT_END]);
        match self.map.get_by_left(&vport) {
            Some(port) => buf[SPORT_BEGIN..=SPORT_END].copy_from_slice(port),
            None => {
                let real_port = Self::get_available_port(&vport);
                self.map.insert(vport, real_port);
                buf[SPORT_BEGIN..=SPORT_END].copy_from_slice(&real_port);
            }
        }
        self.tx.send_to(buf, None);
    }
}
