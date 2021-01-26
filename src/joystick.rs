use crate::cpu::{Interrupts, CPUInterrupts};

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
    state: u8,
    data: u8,
}

impl Joystick {
    pub fn new() -> Self {
        Self {
            data: 0xCF,
            state: 0xFF,
        }
    }

    pub fn inject(&mut self, interrupts: &mut CPUInterrupts, b : JoystickButton, is_pressed: bool) {
        if is_pressed {
            self.state &= !(b as u8);
        }
        else {
            self.state |= b as u8;
        }

        // raise the joypad interrupt
        interrupts.raise_interrupt(Interrupts::Joypad);
    }

    pub fn read_byte(&self, _address: u16) -> u8 {
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

    pub fn write_byte(&mut self, _address: u16, data: u8) {
        self.data = 0xC0 | (data & 0x30);
    }
}