use crate::iomapped::IOMapped;

pub struct Memory {
    data: Vec<u8>,
    base_addr: u16,
}

impl Memory {
    pub fn new(base_addr: u16, size: usize, banks: u8) -> Self {
        Self {
            data: vec![0; size],
            base_addr,
        }
    }
}

impl IOMapped for Memory {
    fn read_byte(&self, address: u16) -> u8 {
        self.data[(address - self.base_addr) as usize]
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        self.data[(address - self.base_addr) as usize] = data;
    }
}