use crate::rom::MBC;

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

impl MBC for MBC1 {
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

    fn get_ram_contents(&self) -> Option<&Vec<u8>> {
        Some(&self.ram)
    }

    fn set_ram_contents(&mut self, ram: &[u8]) {
        self.ram.copy_from_slice(ram);
    }
}