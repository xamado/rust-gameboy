use crate::iomapped::IOMapped;
use core::cell::RefCell;

struct MemoryRegisters {
    pub ff70: u8,
}

struct MemoryState {
    selected_bank: u16,
}

pub struct Memory {
    data: RefCell<Vec<u8>>,
    base_addr: u16,
    bank_size: u16,
    banks: u8,
    state: RefCell<MemoryState>,
    registers: RefCell<MemoryRegisters>,
}

impl Memory {
    pub fn new(base_addr: u16, size: usize, banks: u8) -> Self {
        Self {
            data: RefCell::new(vec![0; size]),
            base_addr,
            banks,
            bank_size: (size as u16) / (banks as u16),
            state: RefCell::new(MemoryState {
                selected_bank: 0,
            }),
            registers: RefCell::new(MemoryRegisters {
                ff70: 0,
            }),
        }
    }

    pub fn switch_bank(&self, data: u8) {
        let mut state = self.state.borrow_mut();
        
        if self.banks > 0 {
            state.selected_bank = (data % self.banks) as u16;
        }
        else {
            state.selected_bank = 0;
        }
    }

    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0xFF70 => {
                let registers = self.registers.borrow_mut();
                registers.ff70
            },
            _ => panic!("Invalid register read")
        }
    }

    pub fn write_register(&self, addr: u16, data: u8) {
        match addr {
            0xFF70 => {
                let mut registers = self.registers.borrow_mut();
                registers.ff70 = data & 0x7;

                let bank = if (data & 0x7) != 0 { (data & 0x7) - 1 } else { 0 };
                self.switch_bank(bank);
            },
            _ => panic!("Invalid register write")
        }
    }
}

impl IOMapped for Memory {
    fn read_byte(&self, address: u16) -> u8 {
        let state = self.state.borrow();

        let addr: u16 = (state.selected_bank * self.bank_size) + (address - self.base_addr);
        let data = self.data.borrow();
        data[addr as usize]
    }

    fn write_byte(&self, address: u16, value: u8) {
        let state = self.state.borrow();

        let addr: u16 = (state.selected_bank * self.bank_size) + (address - self.base_addr);
        let mut data = self.data.borrow_mut();
        data[addr as usize] = value;
    }
}