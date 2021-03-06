use std::str;

pub struct BootROM {
    data: Vec<u8>
}

impl BootROM {
    pub fn new() -> Self {
        Self {
            data: vec!(0; 0)
        }
    }

    pub fn open(&mut self, filename : &str) {
        let bytes = std::fs::read(&filename).unwrap();
        self.data = bytes;
        
        println!("Loaded BOOTROM {}: {} bytes read.", filename, self.len());
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        self.data[address as usize]       
    }
}