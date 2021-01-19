use crate::iomapped::IOMapped;

pub struct Memory {
    data: Vec<u8>,
    base_addr: u16,
    bank_size: u16,
    banks: u8,
    selected_bank: u16,
    pub ff70: u8,
}

impl Memory {
    pub fn new(base_addr: u16, size: usize, banks: u8) -> Self {
        Self {
            data: vec![0; size],
            base_addr,
            banks,
            bank_size: (size as u16) / (banks as u16),
            selected_bank: 0,
            ff70: 0,
        }
    }

    pub fn switch_bank(&mut self, data: u8) {
        if self.banks > 0 {
            self.selected_bank = (data % self.banks) as u16;
        }
        else {
            self.selected_bank = 0;
        }
    }
}

impl IOMapped for Memory {
    fn read_byte(&self, address: u16) -> u8 {
        let addr: u16 = (self.selected_bank * self.bank_size) + (address - self.base_addr);
        self.data[addr as usize]
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        let addr: u16 = (self.selected_bank * self.bank_size) + (address - self.base_addr);
        self.data[addr as usize] = data;
    }
}