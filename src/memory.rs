
struct MemoryRegisters {
    pub ff70: u8,
}

struct MemoryState {
    selected_bank: u16,
}

pub struct Memory {
    data: Vec<u8>,
    base_addr: u16,
    bank_size: u16,
    banks: u8,
    state: MemoryState,
    registers: MemoryRegisters,
}

impl Memory {
    pub fn new(base_addr: u16, size: usize, banks: u8) -> Self {
        Self {
            data: vec![0; size],
            base_addr,
            banks,
            bank_size: (size as u16) / (banks as u16),
            state: MemoryState {
                selected_bank: 0,
            },
            registers: MemoryRegisters {
                ff70: 0,
            },
        }
    }

    pub fn switch_bank(&mut self, data: u8) {
        if self.banks > 0 {
            self.state.selected_bank = (data % self.banks) as u16;
        }
        else {
            self.state.selected_bank = 0;
        }
    }

    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0xFF70 => {
                self.registers.ff70
            },
            _ => panic!("Invalid register read")
        }
    }

    pub fn write_register(&mut self, addr: u16, data: u8) {
        match addr {
            0xFF70 => {
                self.registers.ff70 = data & 0x7;

                let bank = if (data & 0x7) != 0 { (data & 0x7) - 1 } else { 0 };
                self.switch_bank(bank);
            },
            _ => panic!("Invalid register write")
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        let addr: u16 = (self.state.selected_bank * self.bank_size) + (address - self.base_addr);
        self.data[addr as usize]
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        let addr: u16 = (self.state.selected_bank * self.bank_size) + (address - self.base_addr);
        self.data[addr as usize] = value;
    }
}