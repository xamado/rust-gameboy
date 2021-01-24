pub trait IOMapped {
    #[allow(unused)]
    fn read_byte(&self, address: u16) -> u8 { 0 }

    #[allow(unused)]
    fn write_byte(&self, address: u16, data: u8) {}
}