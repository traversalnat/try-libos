extern crate alloc;

use core::panic;

use pnet::datalink::{self, Channel::Ethernet, DataLinkReceiver, DataLinkSender, NetworkInterface};

use alloc::{collections::LinkedList, vec, vec::Vec};
use spin::{Lazy, Mutex};

pub struct EthDevice {
    tx: Box<dyn DataLinkSender>,
    rx: Box<dyn DataLinkReceiver>,
}

pub const MACADDR: [u8; 6] = [0x12, 0x13, 0x89, 0x89, 0xdf, 0x53];
const DMAC_BEGIN: usize = 0;
const DMAC_END: usize = 5;
const ETH_TYPE_BEGIN: usize = 12;
const ETH_TYPE_END: usize = 13;

static RECV_RING: Lazy<Mutex<LinkedList<Vec<u8>>>> = Lazy::new(|| Mutex::new(LinkedList::new()));

impl EthDevice {
    pub fn new() -> Self {
        // TODO 注入 网卡地址
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

        Self { tx, rx }
    }

    fn is_valid_packet(&self, buf: &mut [u8]) -> bool {
        let mut is_valid = false;
        // check mac
        if buf[DMAC_BEGIN..=DMAC_END] == MACADDR {
            is_valid = true;
        }

        if buf[ETH_TYPE_BEGIN] == 8 && buf[ETH_TYPE_END] == 6 {
            // arp packet
            is_valid = true;
        }

        is_valid
    }

    pub fn recv(&mut self, buf: &mut [u8]) -> usize {
        if let Some(block) = RECV_RING.lock().pop_back() {
            let min_len = core::cmp::min(block.len(), buf.len());
            buf[..min_len].copy_from_slice(&block[..min_len]);
            return min_len;
        }
        0
    }

    pub fn async_recv(&mut self) {
        if let Ok(packet) = self.rx.next() {
            let mut buf = vec![0u8; packet.len()];
            buf.copy_from_slice(&packet);
            if self.is_valid_packet(&mut buf) {
                RECV_RING.lock().push_back(buf);
            }
        }
    }

    pub fn send(&mut self, buf: &mut [u8]) {
        self.tx.send_to(buf, None);
    }
}
