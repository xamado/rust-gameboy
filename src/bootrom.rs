use crate::iomapped::IOMapped;
use std::str;

pub struct BootROM {
    data: Vec<u8>
}

impl BootROM {
    pub fn new() -> Self {
        Self {
            data: vec!(0; 0x100)
        }
    }

    pub fn open(&mut self, filename : &str) {
        let bytes = std::fs::read(&filename).unwrap();
        let length = bytes.len();
        self.data.copy_from_slice(&bytes);
        
        println!("Loaded BOOTROM {}: {} bytes read.", filename, length);
    }
}

impl IOMapped for BootROM {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x00FF => {
                self.data[address as usize]
            },

            _ => panic!("Invalid ROM read")
        }
        
    }

    fn write_byte(&mut self, address: u16, _data: u8) {
        panic!("Invalid BOOTROM write {:#06x}", address);
    }
}