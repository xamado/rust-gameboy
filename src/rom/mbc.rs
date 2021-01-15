pub trait MBC {
    #[allow(unused)]
    fn read_byte(&self, address: u16) -> u8 { 0 }

    #[allow(unused)]
    fn write_byte(&mut self, address: u16, data: u8) {}
    
    #[allow(unused)]
    fn get_ram_contents(&self) -> Option<&Vec<u8>> { None }

    #[allow(unused)]
    fn set_ram_contents(&mut self, ram: &[u8]) { }
}