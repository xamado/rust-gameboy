use std::str;
use std::path::PathBuf;
use std::fs::File;
use std::io::prelude::*;

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
use crate::machine::GameBoyModel;

pub struct ROM {
    rom_type: GameBoyModel,
    filename: String,
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
            rom_type: GameBoyModel::DMG,
            filename: String::new(),
            mbc: None
        }
    }

    pub fn open(&mut self, filename : &str) {
        // open the rom file
        self.filename = filename.to_owned();
        let bytes = std::fs::read(&filename).expect("Failed to open ROM");

        let gbc_mode = bytes[0x143];
        self.rom_type = match gbc_mode {
            0x80 | 0xC0 => GameBoyModel::GBC,
            // 0x80 => GameBoyModel::DMG, // 0x80 is playable on GBC... but we default to DMG mode
            // 0xC0 => GameBoyModel::GBC,
            _ => GameBoyModel::DMG
        };

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
        
        if let Some(mbc) = &mut self.mbc {
            // load ram contents if present
            let mut path = PathBuf::from(filename);
            path.set_extension("sav");

            if path.exists() {
                let bytes = std::fs::read(&path).expect("Failed to open RAM");
                mbc.set_ram_contents(&bytes);
            }
        }

        println!("Loaded ROM {}: {} bytes read. Type: {}.", filename, bytes.len(), cart_type);
    }

    pub fn get_rom_type(&self) -> GameBoyModel {
        self.rom_type
    }
    
    pub fn close(&self) {
        let mut path = PathBuf::from(self.filename.to_owned());
        path.set_extension("sav");
        
        if let Some(mbc) = &self.mbc {
            if let Some(ram) = mbc.get_ram_contents() {
                let mut file = File::create(path).expect("Failed to create SAV file");
                file.write_all(&ram[0..ram.len()]).expect("Failed to write to SAV file");
            }
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        if let Some(mbc) = &self.mbc {
            mbc.read_byte(address)
        }
        else {
            0
        }
    }

    pub fn write_byte(&mut self, address: u16, data: u8) {
        if let Some(mbc) = &mut self.mbc {
            mbc.write_byte(address, data);
        }
    }
}
