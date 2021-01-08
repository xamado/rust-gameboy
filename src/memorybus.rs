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

        // reading from unmapped areas has different behaviours...
        // TODO: Improve this with actual mappings
        match address {
            0xFEA0..=0xFEFF => 0x00,
            0xFF00..=0xFF7F => 0xFF,
            _ => 0
        }
    }

    pub fn write_byte(&mut self, address: u16, data: u8) {
        if address == 0xFF50 {
            if let Some(pos) = self.mappings.iter().position(|x| (*x).0 == (0x0000..=0x00FF)) {
                self.mappings.remove(pos);
            }
        }

        for (r, i) in self.mappings.iter() {
            if r.contains(&address) {
                return i.borrow_mut().write_byte(address, data);
            }
        }
    }
}