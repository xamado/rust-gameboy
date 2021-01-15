
use crate::rom::mbc::MBC;

pub struct MBC0 {
    data: Vec<u8>,
}

impl MBC0 {
    pub fn new(d: &[u8]) -> Self {
        Self {
            data: d.to_vec()
        }
    }
}

impl MBC for MBC0 {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => {
                self.data[address as usize]
            },
            _ => panic!("Invalid ROM read")
        }
        
    }

    fn write_byte(&mut self, _address: u16, _data: u8) {
        // panic!("Invalid ROM write {:#06x}", address)
    }
}