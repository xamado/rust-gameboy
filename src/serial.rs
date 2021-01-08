use crate::iomapped::IOMapped;

pub struct Serial {
}

impl Serial {
    pub fn new() -> Self {
        Self {
            
        }
    }
}

impl IOMapped for Serial {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0xFF01 => 0,
            0xFF02 => 0x7E,
            _ => unreachable!()
        }
    }

    fn write_byte(&mut self, _address: u16, data: u8) {
        println!("serial: {:#04x}", data);
    }
}