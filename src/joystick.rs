use std::rc::Rc;
use core::cell::RefCell;
use crate::memorybus::MemoryBus;
use crate::cpu::Interrupts;
use crate::iomapped::IOMapped;

#[allow(unused)]
pub enum JoystickButton {
    A = 1,
    B = 1 << 1,
    Select = 1 << 2,
    Start = 1 << 3,
    Right = 1 << 4,
    Left = 1 << 5,
    Up = 1 << 6,
    Down = 1 << 7
}

pub struct Joystick {
    bus: Rc<RefCell<MemoryBus>>,
    state: u8,
    data: u8,
}

impl Joystick {
    pub fn new(bus: Rc<RefCell<MemoryBus>>) -> Self {
        Self {
            data: 0xCF,
            state: 0xFF,
            bus
        }
    }

    pub fn inject(&mut self, b : JoystickButton, is_pressed: bool) {
        if is_pressed {
            self.state &= !(b as u8);
        }
        else {
            self.state |= b as u8;
        }

        // raise the joypad interrupt
        let iif = self.bus.borrow().read_byte(0xFF0F) | (1 << Interrupts::Joypad as u8);
        self.bus.borrow().write_byte(0xFF0F, iif);
    }
}

impl IOMapped for Joystick {
    fn read_byte(&self, _address: u16) -> u8 {
        if self.data & (1 << 5) == 0 { // select button keys
            0xC0 | self.data & 0x30 | self.state & 0x0F
        }
        else if self.data & (1 << 4) == 0 { // select direction keys
            0xC0 | self.data & 0x30 | ((self.state & 0xF0) >> 4)
        }
        else {
            0xC0 | self.data
        }
    }

    fn write_byte(&mut self, _address: u16, data: u8) {
        self.data = 0xC0 | (data & 0x30);
    }
}