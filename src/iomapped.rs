use std::rc::Rc;
use core::cell::RefCell;

pub trait IOMapped {
    #[allow(unused)]
    fn read_byte(&self, address: u16) -> u8 { 0 }

    #[allow(unused)]
    fn write_byte(&mut self, address: u16, data: u8) {}
}

pub type IOMappedRef = Rc<RefCell<dyn IOMapped>>;