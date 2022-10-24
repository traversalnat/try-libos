extern crate alloc;

use alloc::{
    alloc::{alloc, dealloc},
    sync::Arc,
    vec, string::String,
};
use core::alloc::Layout;
use core::ptr::NonNull;
use device_tree::{util::SliceRead, DeviceTree, Node};
use virtio_drivers::*;
use spin::{Lazy, Mutex};

use stdio::*;

// pub static NetDevice: Lazy<Mutex<Option<VirtIONet::<HalImpl, MmioTransport>>>> =
//     Lazy::new(|| None);

const PAGE_SIZE: usize = 4096;

pub struct HalImpl;

impl Hal for HalImpl {
    fn dma_alloc(pages: usize) -> PhysAddr {
        let layout = Layout::from_size_align(PAGE_SIZE * pages, PAGE_SIZE).unwrap();
        unsafe {
            alloc(layout) as usize
        }
    }

    fn dma_dealloc(paddr: PhysAddr, pages: usize) -> i32 {
        let layout = Layout::from_size_align(PAGE_SIZE * pages, PAGE_SIZE).unwrap();
        unsafe {
            dealloc(paddr as *mut u8, layout);
        }
        0
    }

    fn phys_to_virt(paddr: PhysAddr) -> VirtAddr {
        paddr
    }

    fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
        vaddr
    }
}

pub fn init(device_tree_paddr: usize) {
    init_dt(device_tree_paddr);
}

fn init_dt(dtb: usize) {
    log::info!("device tree @ {:#x}", dtb);
    #[repr(C)]
    struct DtbHeader {
        be_magic: u32,
        be_size: u32,
    }
    let header = unsafe { &*(dtb as *const DtbHeader) };
    let magic = u32::from_be(header.be_magic);
    const DEVICE_TREE_MAGIC: u32 = 0xd00dfeed;
    assert_eq!(magic, DEVICE_TREE_MAGIC);
    let size = u32::from_be(header.be_size);
    let dtb_data = unsafe { core::slice::from_raw_parts(dtb as *const u8, size as usize) };
    let dt = DeviceTree::load(dtb_data).expect("failed to parse device tree");
    walk_dt_node(&dt.root);
}

fn walk_dt_node(dt: &Node) {
    if let Ok(compatible) = dt.prop_str("compatible") {
        if compatible == "virtio,mmio" {
            virtio_probe(dt);
        }
    }
    for child in dt.children.iter() {
        walk_dt_node(child);
    }
}

fn virtio_probe(node: &Node) {
    if let Some(reg) = node.prop_raw("reg") {
        let paddr = reg.as_slice().read_be_u64(0).unwrap();
        let size = reg.as_slice().read_be_u64(8).unwrap();
        let vaddr = paddr;
        log::info!("walk dt addr={:#x}, size={:#x}", paddr, size);
        log::info!("Device tree node {:?}", node);
        let header = NonNull::new(vaddr as *mut VirtIOHeader).unwrap();
        match unsafe { MmioTransport::new(header) } {
            Err(e) => log::error!("Error creating VirtIO MMIO transport: {}", e),
            Ok(transport) => {
                log::info!(
                    "Detected virtio MMIO device with vendor id {:#X}, device type {:?}, version {:?}",
                    transport.vendor_id(),
                    transport.device_type(),
                    transport.version(),
                );
                virtio_device(transport);
            }
        }
    }
}

fn virtio_device(transport: impl Transport) {
    match transport.device_type() {
        // DeviceType::Block => virtio_blk(transport),
        // DeviceType::GPU => virtio_gpu(transport),
        // DeviceType::Input => virtio_input(transport),
        DeviceType::Network => virtio_net(transport),
        t => log::info!("Unrecognized virtio device: {:?}", t),
    }
}

fn virtio_net<T: Transport>(transport: T) {
    let mut net = VirtIONet::<HalImpl, T>::new(transport).unwrap();
    // let lock = NetDevice.lock();
    // *lock = Some(net);
}
