use crate::iomapped::IOMapped;

pub struct ROM {
    data: Vec<u8>,
    bank: u8
}

impl ROM {
    pub fn new() -> Self {
        Self {
            data: vec!(),
            bank: 1
        }
    }

    pub fn open(&mut self, filename : &str) {
        let bytes = std::fs::read(&filename).unwrap();
        let length = bytes.len();
        // self.data[0..length].copy_from_slice(&bytes[0..length]);
        self.data = bytes.to_vec();

        println!("Loaded ROM {}: {} bytes read", filename, length);
    }

    fn select_rom_bank(&mut self, data: u8) {
        let l: u8 = data & 31;
        self.bank &= 224;
        self.bank |= l;

        if self.bank == 0 {
            self.bank += 1;
        }
    }
}

impl IOMapped for ROM {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.data[address as usize],
            0x4000..=0x7FFF => self.data[(address + 0x4000 * ((self.bank - 1) as u16)) as usize],
            _ => panic!("Invalid ROM read")
        }
        
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            0x2000..=0x3FFF => self.select_rom_bank(data),
            _ => panic!("Invalid ROM write")
        }
    }
}