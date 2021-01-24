use core::cell::RefCell;
use crate::rom::MBC;

struct MBC1Registers {
    ram_enabled: bool,
    mode: u8,
    bank1: u8,
    bank2: u8,
}

pub struct MBC1 {
    data: RefCell<Vec<u8>>,
    ram: RefCell<Vec<u8>>,
    registers: RefCell<MBC1Registers>,
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
            data: RefCell::new(data.to_vec()),
            ram: RefCell::new(vec!(0; vec_ram_size)),
            registers: RefCell::new(MBC1Registers {
                mode: 0,
                ram_enabled: false,
                bank1: 1,
                bank2: 0,
            }),
            num_rom_banks,
            num_ram_banks,
        }
    }
}

impl MBC for MBC1 {
    fn read_byte(&self, address: u16) -> u8 {
        let registers = self.registers.borrow();
        
        
        match address {
            0x0000..=0x3FFF => {
                let rom = self.data.borrow();
                if registers.mode == 0 { 
                    rom[address as usize]
                } 
                else { 
                    let bank: u32 = ((registers.bank2 << 5) as u32) % (self.num_rom_banks as u32);
                    let idx: u32 = (bank * 0x4000) + (address as u32);
                    rom[idx as usize]
                }                
            },

            0x4000..=0x7FFF => {
                let rom = self.data.borrow();
                let bank: u32 = (((registers.bank2 as u32) << 5 | (registers.bank1 as u32)) as u32) % (self.num_rom_banks as u32);
                let idx: u32 = (bank * 0x4000) + ((address - 0x4000) as u32);
                rom[idx as usize]
            },

            0xA000..=0xBFFF => {
                if registers.ram_enabled {
                    let rom = self.data.borrow();
                    let ram_bank: u32 = if registers.mode == 0 || self.num_ram_banks <= 1 { 0 } else { (registers.bank2 & 0x3) as u32 };
                    let ram_addr: u32 = (ram_bank * 0x2000) + ((address - 0xA000) as u32);
                    rom[ram_addr as usize]
                }
                else {
                    0xff
                }
            },

            _ => panic!("Invalid ROM read")
        }
        
    }

    fn write_byte(&self, address: u16, data: u8) {
        let mut registers = self.registers.borrow_mut();

        match address {
            0x0000..=0x1FFF => {
                registers.ram_enabled = data == 0x0A;
            },

            0x2000..=0x3FFF => {
                registers.bank1 = if (data & 0x1F) == 0 { 0x01 } else { data & 0x1F };
            },
            
            // RAM bank number / Upper bits of rom bank number
            0x4000..=0x5FFF => { 
                registers.bank2 = data & 0x3;
            },
            
            0x6000..=0x7FFF => { 
                registers.mode = data & 0x1;
            },

            0xA000..=0xBFFF => {
                if registers.ram_enabled {
                    let mut ram = self.ram.borrow_mut();

                    let ram_bank: u32 = if registers.mode == 0 || self.num_ram_banks <= 1 { 0 } else { (registers.bank2 & 0x3) as u32 };
                    let ram_addr: u32 = (ram_bank * 0x2000) + ((address - 0xA000) as u32);
                    ram[ram_addr as usize] = data;

                    // println!("RAM{}:{:#04x} {:#04x}", ram_bank, address, data);
                }
            },

            _ => panic!("Invalid ROM write {:#06x}", address)
        }
    }

    fn get_ram_contents(&self) -> Option<Vec<u8>> {
        let ram = self.ram.borrow_mut();
        Some(ram.to_owned())
    }

    fn set_ram_contents(&self, data: &[u8]) {
        let mut ram = self.ram.borrow_mut();
        ram.copy_from_slice(data);
    }
}