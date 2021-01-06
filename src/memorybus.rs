use std::ops::RangeInclusive;
use crate::iomapped::IOMappedRef;

pub struct MemoryBus {
    mappings: Vec<(RangeInclusive<u16>, IOMappedRef)>
}

impl MemoryBus {
    pub fn new() -> Self {
        Self {
            mappings: vec!(),
        }
    }

    pub fn map(&mut self, range: RangeInclusive<u16>, mapped: IOMappedRef) {
        self.mappings.push((range, mapped));
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        for (r, i) in self.mappings.iter() {
            if r.contains(&address) {
                return i.borrow().read_byte(address);
            }
        }

        panic!("Invalid address");
    }

    pub fn write_byte(&mut self, address: u16, data: u8) {
        if address == 0xFF50 {
            self.mappings.remove(self.mappings.iter().position(|x| (*x).0 == (0x0000..=0x00FF)).expect("needle not found"));
        }

        for (r, i) in self.mappings.iter() {
            if r.contains(&address) {
                return i.borrow_mut().write_byte(address, data);
            }
        }
    }
}