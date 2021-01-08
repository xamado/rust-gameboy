use crate::iomapped::IOMapped;

pub struct ROM {
    data: Vec<u8>,
    memory_mode: u8,
    rom_bank: u8,
    ram_enabled: bool,
    ram_bank: u8,
    ram: Vec<u8>
}

impl ROM {
    pub fn new() -> Self {
        Self {
            data: vec!(0; 0x100),
            memory_mode: 1,
            rom_bank: 1,
            ram_enabled: false,
            ram_bank: 0,
            ram: vec!(0; 0x2000) // 8Kb
        }
    }

    pub fn open(&mut self, filename : &str) {
        let bytes = std::fs::read(&filename).unwrap();
        let length = bytes.len();
        self.data = bytes.to_vec();

        println!("Loaded ROM {}: {} bytes read", filename, length);
    }
}

impl IOMapped for ROM {
    fn read_byte(&self, address: u16) -> u8 {
        let bank = if self.rom_bank == 0x00 || self.rom_bank == 0x20 || self.rom_bank == 0x40 || self.rom_bank == 0x60 { self.rom_bank + 1 } else { self.rom_bank };

        match address {
            0x0000..=0x3FFF => self.data[address as usize],
            0x4000..=0x7FFF => {
                let idx: usize = (address as usize) + ((bank - 1) as usize) * 0x4000;
                self.data[idx]
            }
            0xA000..=0xBFFF => self.ram[(address - 0xA000) as usize],
            _ => panic!("Invalid ROM read")
        }
        
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            0x0000..=0x1FFF => self.ram_enabled = data == 0x0A,
            0x2000..=0x3FFF => {
                self.rom_bank = self.rom_bank & 0xE0 | data & 0x1F
            },
            
            // RAM bank number / Upper bits of rom bank number
            0x4000..=0x5FFF => { 
                if self.memory_mode == 0 {
                    self.rom_bank = (self.rom_bank & 0x1F) | ((data & 0x3) << 6)
                }
                else {
                    self.ram_bank = data & 0x3;
                }
            }
            0x6000..=0x7FFF => { self.memory_mode = data & 0x1 } // rom/ram mode select
            0xA000..=0xBFFF => {
                self.ram[(address - 0xA000) as usize] = data;
            }
            _ => panic!("Invalid ROM write {:#06x}", address)
        }
    }
}