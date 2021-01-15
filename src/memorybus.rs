use std::ops::RangeInclusive;
use hashbrown::HashMap;

pub type WriteHandler = Box<dyn Fn(u16, u8)>;
pub type ReadHandler = Box<dyn Fn(u16) -> u8>;

pub struct MemoryBus {
    write_addr_mappings: HashMap<u16, WriteHandler>,
    read_addr_mappings: HashMap<u16, ReadHandler>,
    write_rng_mappings: Vec<(RangeInclusive<u16>, WriteHandler)>,
    read_rng_mappings: Vec<(RangeInclusive<u16>, ReadHandler)>,
}

impl MemoryBus {
    pub fn new() -> Self {
        Self {
            write_addr_mappings: HashMap::new(),
            read_addr_mappings: HashMap::new(),
            write_rng_mappings: vec!(),
            read_rng_mappings: vec!(),
        }
    }

    pub fn map_range_write<F: Fn(u16, u8) + 'static>(&mut self, range: RangeInclusive<u16>, f: F) {
        self.write_rng_mappings.push((range, Box::new(f)));
    }

    pub fn map_range_read<F: Fn(u16) -> u8 + 'static>(&mut self, range: RangeInclusive<u16>, f: F) {
        self.read_rng_mappings.push((range, Box::new(f)));
    }

    pub fn map_address_write<F: Fn(u16, u8) + 'static>(&mut self, addr: u16, f: F) {
        self.write_addr_mappings.insert(addr, Box::new(f));
    }

    pub fn map_address_read<F: Fn(u16) -> u8 + 'static>(&mut self, addr: u16, f: F) {
        self.read_addr_mappings.insert(addr, Box::new(f));
    }

    pub fn unmap_range_read(&mut self, range: RangeInclusive<u16>) {
        if let Some(pos) = self.read_rng_mappings.iter().position(|x| (*x).0 == (range)) {
            let _ = self.read_rng_mappings.remove(pos);
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        if self.read_addr_mappings.contains_key(&address) {
            return self.read_addr_mappings[&address](address);
        }
        else {
            for (r, i) in self.read_rng_mappings.iter() {
                if r.contains(&address) {
                    return i(address);
                }
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
        if self.write_addr_mappings.contains_key(&address) {
            self.write_addr_mappings[&address](address, data);
        }
        else {
            for (r, i) in self.write_rng_mappings.iter() {
                if r.contains(&address) {
                    i(address, data);
                }
            }
        }
    }
}