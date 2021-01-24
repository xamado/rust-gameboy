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

struct JoystickRegisters {
    data: u8,
}

struct JoystickState {
    state: u8,
}

pub struct Joystick {
    state: RefCell<JoystickState>,
    registers: RefCell<JoystickRegisters>,
}

impl Joystick {
    pub fn new() -> Self {
        Self {
            registers: RefCell::new(JoystickRegisters {
                data: 0xCF,
            }),
            state: RefCell::new(JoystickState {
                state: 0xFF,
            }),
        }
    }

    pub fn inject(&self, bus: &MemoryBus, b : JoystickButton, is_pressed: bool) {
        let mut state = self.state.borrow_mut();
        if is_pressed {
            state.state &= !(b as u8);
        }
        else {
            state.state |= b as u8;
        }

        // raise the joypad interrupt
        let iif = bus.read_byte(0xFF0F) | (1 << Interrupts::Joypad as u8);
        bus.write_byte(0xFF0F, iif);
    }
}

impl IOMapped for Joystick {
    fn read_byte(&self, _address: u16) -> u8 {
        let registers = self.registers.borrow();
        let state = self.state.borrow();

        if registers.data & (1 << 5) == 0 { // select button keys
            0xC0 | registers.data & 0x30 | state.state & 0x0F
        }
        else if registers.data & (1 << 4) == 0 { // select direction keys
            0xC0 | registers.data & 0x30 | ((state.state & 0xF0) >> 4)
        }
        else {
            0xC0 | registers.data
        }
    }

    fn write_byte(&self, _address: u16, data: u8) {
        let mut registers = self.registers.borrow_mut();

        registers.data = 0xC0 | (data & 0x30);
    }
}