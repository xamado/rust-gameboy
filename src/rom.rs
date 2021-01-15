use crate::iomapped::IOMapped;
use std::str;

mod mbc;
mod mbc0;
mod mbc1;
mod mbc3;
mod mbc5;

use crate::rom::mbc::MBC;
use crate::rom::mbc0::MBC0;
use crate::rom::mbc1::MBC1;
use crate::rom::mbc3::MBC3;
use crate::rom::mbc5::MBC5;

pub struct ROM {
    mbc: Option<Box<dyn MBC>>
}

impl Default for ROM {
    fn default() -> Self {
        ROM::new()
    }
}

impl ROM {
    pub fn new() -> Self {
        Self {
            mbc: None
        }
    }

    pub fn open(&mut self, filename : &str) {
        let bytes = std::fs::read(&filename).expect("Failed to open ROM");

        let cart_type = bytes[0x0147];
        let rom_size = bytes[0x0148];
        let ram_size = bytes[0x0149];

        self.mbc = match cart_type {
            0x00 => {
                Some(Box::new(MBC0::new(&bytes)))
            },
            0x01 | 0x02 | 0x03 => {
                Some(Box::new(MBC1::new(rom_size, ram_size, &bytes)))
            },
            0x11 | 0x12 | 0x13 => {
                Some(Box::new(MBC3::new(rom_size, ram_size, &bytes)))
            },
            0x19 | 0x1A | 0x1B | 0x1C | 0x1D | 0x1E => {
                Some(Box::new(MBC5::new(rom_size, ram_size, &bytes)))
            }
            _ => panic!("Unsupported Cart type: {:#04x}", cart_type)
        };
        
        println!("Loaded ROM {}: {} bytes read. Type: {}.", filename, bytes.len(), cart_type);
    }

    pub fn get_ram_contents(&self) -> Option<&Vec<u8>> {
        if let Some(mbc) = &self.mbc {
            mbc.get_ram_contents()
        }
        else {
            None
        }   
    }

    pub fn set_ram_contents(&mut self, ram: &[u8]) {
        if let Some(mbc) = &mut self.mbc {
            mbc.set_ram_contents(&ram);
        }
    }
}

impl IOMapped for ROM {
    fn read_byte(&self, address: u16) -> u8 {
        if let Some(mbc) = &self.mbc {
            mbc.read_byte(address)
        }
        else {
            0
        }
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        if let Some(mbc) = &mut self.mbc {
            mbc.write_byte(address, data);
        }
    }
}