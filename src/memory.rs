use crate::iomapped::IOMapped;

pub struct Memory {
    data: Vec<u8>
}

impl Memory {
    pub fn new() -> Self {
        Self {
            data: vec![0; 0x8000]
        }
    }
}

impl IOMapped for Memory {
    fn read_byte(&self, address: u16) -> u8 {
        self.data[(address - 0x8000) as usize]
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        self.data[(address - 0x8000) as usize] = data;
    }
}