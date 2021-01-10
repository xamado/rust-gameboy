use crate::iomapped::IOMapped;
use std::str;

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

impl IOMapped for MBC0 {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => {
                self.data[address as usize]
            },
            _ => panic!("Invalid ROM read")
        }
        
    }

    fn write_byte(&mut self, address: u16, _data: u8) {
        panic!("Invalid ROM write {:#06x}", address)
    }
}

pub struct MBC1 {
    data: Vec<u8>,
    ram: Vec<u8>,
    ram_enabled: bool,
    mode: u8,
    bank1: u8,
    bank2: u8,
    num_rom_banks: u8,
    num_ram_banks: u8
}

impl MBC1 {
    pub fn new(rom_size: u8, ram_size: u8, data: &[u8]) -> Self {
        // calculate number of rom banks
        let data_size = (0x8000 << rom_size) as usize;
        let num_rom_banks = ((data_size as u32) / 0x4000) as u8;
        
        // and ram banks
        let (num_ram_banks, vec_ram_size) = match ram_size {
            0 => (0, 0),
            1 => (1, 0x800),
            2 => (1, 0x2000),
            3 => (4, 0x8000),
            _ => panic!("Invalid RAM size for MBC1")
        };
        
        Self {
            data: data.to_vec(),
            mode: 0,
            ram_enabled: false,
            ram: vec!(0; vec_ram_size), 
            bank1: 1,
            bank2: 0,
            num_rom_banks,
            num_ram_banks,
        }
    }
}

impl IOMapped for MBC1 {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                if self.mode == 0 { 
                    self.data[address as usize]
                } 
                else { 
                    let bank: u32 = ((self.bank2 << 5) as u32) % (self.num_rom_banks as u32);
                    let idx: u32 = (bank * 0x4000) + (address as u32);
                    self.data[idx as usize]
                }                
            },

            0x4000..=0x7FFF => {
                let bank: u32 = (((self.bank2 as u32) << 5 | (self.bank1 as u32)) as u32) % (self.num_rom_banks as u32);
                let idx: u32 = (bank * 0x4000) + ((address - 0x4000) as u32);
                self.data[idx as usize]
            },

            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    let ram_bank: u32 = if self.mode == 0 || self.num_ram_banks <= 1 { 0 } else { (self.bank2 & 0x3) as u32 };
                    let ram_addr: u32 = (ram_bank * 0x2000) + ((address - 0xA000) as u32);
                    self.ram[ram_addr as usize]
                }
                else {
                    0xff
                }
            },

            _ => panic!("Invalid ROM read")
        }
        
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            0x0000..=0x1FFF => {
                self.ram_enabled = data == 0x0A;
            },

            0x2000..=0x3FFF => {
                self.bank1 = if (data & 0x1F) == 0 { 0x01 } else { data & 0x1F };
            },
            
            // RAM bank number / Upper bits of rom bank number
            0x4000..=0x5FFF => { 
                self.bank2 = data & 0x3;
            },
            
            0x6000..=0x7FFF => { 
                self.mode = data & 0x1;
            },

            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    let ram_bank: u32 = if self.mode == 0 || self.num_ram_banks <= 1 { 0 } else { (self.bank2 & 0x3) as u32 };
                    let ram_addr: u32 = (ram_bank * 0x2000) + ((address - 0xA000) as u32);
                    self.ram[ram_addr as usize] = data;

                    // println!("RAM{}:{:#04x} {:#04x}", ram_bank, address, data);
                }
            },

            _ => panic!("Invalid ROM write {:#06x}", address)
        }
    }
}

pub struct MBC3 {
    data: Vec<u8>,
    ram: Vec<u8>,
    ram_enabled: bool,
    mode: u8,
    rom_bank: u8,
    ram_bank: u8,
    num_rom_banks: u8,
    num_ram_banks: u8
}

impl MBC3 {
    pub fn new(rom_size: u8, ram_size: u8, data: &[u8]) -> Self {
        // calculate number of rom banks
        let data_size = (0x8000 << rom_size) as usize;
        let num_rom_banks = ((data_size as u32) / 0x4000) as u8;
        
        // and ram banks
        let (num_ram_banks, vec_ram_size) = match ram_size {
            0 => (0, 0),
            1 => (1, 0x800),
            2 => (1, 0x2000),
            3 => (4, 0x8000),
            _ => panic!("Invalid RAM size for MBC1")
        };
        
        Self {
            data: data.to_vec(),
            mode: 0,
            ram_enabled: false,
            ram: vec!(0; vec_ram_size), 
            rom_bank: 1,
            ram_bank: 0,
            num_rom_banks,
            num_ram_banks,
        }
    }
}

impl IOMapped for MBC3 {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                // if self.mode == 0 { 
                    self.data[address as usize]
                // } 
                // else { 
                //     let bank: u32 = ((self.bank2 << 5) as u32) % (self.num_rom_banks as u32);
                //     let idx: u32 = (bank * 0x4000) + (address as u32);
                //     self.data[idx as usize]
                // }                
            },

            0x4000..=0x7FFF => {
                let bank: u32 = (self.rom_bank as u32) % (self.num_rom_banks as u32);
                let idx: u32 = (bank * 0x4000) + ((address - 0x4000) as u32);
                self.data[idx as usize]
            },

            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    let ram_bank: u32 = if self.mode == 0 || self.num_ram_banks <= 1 { 0 } else { (self.ram_bank & 0x3) as u32 };
                    let ram_addr: u32 = (ram_bank * 0x2000) + ((address - 0xA000) as u32);
                    self.ram[ram_addr as usize]
                }
                else {
                    0xff
                }
            },

            _ => panic!("Invalid ROM read")
        }
        
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            0x0000..=0x1FFF => {
                self.ram_enabled = data == 0x0A;
            },

            0x2000..=0x3FFF => {
                self.rom_bank = if data == 0 { 1 } else { data };
            },
            
            // RAM bank number / RTC register select
            0x4000..=0x5FFF => { 
                self.ram_bank = data & 0x3;
            },
            
            0x6000..=0x7FFF => { 
                self.mode = data & 0x1;
            },

            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    let ram_bank: u32 = if self.mode == 0 || self.num_ram_banks <= 1 { 0 } else { (self.ram_bank & 0x3) as u32 };
                    let ram_addr: u32 = (ram_bank * 0x2000) + ((address - 0xA000) as u32);
                    self.ram[ram_addr as usize] = data;

                    // println!("RAM{}:{:#04x} {:#04x}", ram_bank, address, data);
                }
            },

            _ => panic!("Invalid ROM write {:#06x}", address)
        }
    }
}

pub struct ROM {
    mbc: Option<Box<dyn IOMapped>>
}

pub struct CartridgeHeader {
    title: String,
    manufacturer: [u8; 4],
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
        let bytes = std::fs::read(&filename).unwrap();
        // let length = bytes.len();

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
            }
            _ => panic!("Unsupported Cart type: {:#04x}", cart_type)
        };
        
        //println!("Loaded ROM {}: {} bytes read. Type: {}. Banks: {}", filename, length, cart_type, self.num_rom_banks);
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