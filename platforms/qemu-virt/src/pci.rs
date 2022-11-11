use stdio::log;
use volatile::Volatile;

pub fn pci_init() {
    for dev in 0..32 {
        let off = dev << 11;
        let base_addr = 0x30000000 + off;
        let base = unsafe { core::slice::from_raw_parts_mut(base_addr as *mut Volatile<u32>, 10) };

        let id = base[0].read();
        // e1000
        if id == 0x100e8086 {
            log::info!("{id:x} found e1000");
            base[1].write(7);
            for i in 0..6 {
                let old = base[4 + i].read();
                base[4 + i].write(0xffffffff);
                base[4 + i].write(old);
            }
            // e1000 register address
            base[4].write(0x40000000);

            crate::e1000::init(0x40000000, 0x10000 * 4);
            break;
        }
    }
}
