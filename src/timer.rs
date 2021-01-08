use std::rc::Rc;
use core::cell::RefCell;

use crate::iomapped::IOMapped;
use crate::memorybus::MemoryBus;
use crate::cpu::Interrupts;

pub struct Timer {
    bus: Rc<RefCell<MemoryBus>>,
    timer_enabled: bool,
    timer_frequency: u8,
    timer_counter: u8,
    timer_modulo: u8,
    timer_overflow: bool,
    timer_overflow_counter: u8,
    internal_counter: u16,
    prev_and_result: u8
}

const TIMER_FREQ_BIT : [u8; 4] = [
    9, // 0b00 ~ 4096 Hz
    3, // 0b01 ~ 262144 Hz
    5, // 0b10 ~ 65536 Hz
    7, // 0b11 ~ 16384 Hz
];

impl Timer {
    pub fn new(bus: Rc<RefCell<MemoryBus>>) -> Self {
        Self {
            bus,
            timer_enabled: false,
            timer_frequency: 0,
            timer_counter: 0,
            timer_modulo: 0,
            timer_overflow: false,
            timer_overflow_counter: 0,

            internal_counter: 0xABCC,
            prev_and_result: 0,
        }
    }

    pub fn step_clock(&mut self) {
        self.internal_counter = self.internal_counter.wrapping_add(1);

        // TIMA overflow resets after 4 cycles
        if self.timer_overflow {
            self.timer_overflow_counter += 1;

            if self.timer_overflow_counter > 4 {
                self.timer_counter = self.timer_modulo;

                // raise the Timer interrupt
                let iif = self.bus.borrow().read_byte(0xFF0F) | (1 << Interrupts::Timer as u8);
                self.bus.borrow_mut().write_byte(0xFF0F, iif);

                self.timer_overflow = false;
                self.timer_overflow_counter = 0;
            }
        }

        let bit = TIMER_FREQ_BIT[self.timer_frequency as usize];
        let and_result = ((self.internal_counter & (1 << bit)) >> bit) & (self.timer_enabled as u16);

        // check for falling edge
        if and_result == 0 && self.prev_and_result == 1 {
            if self.timer_counter == 0xFF {
                self.timer_overflow = true;
                self.timer_overflow_counter = 0;
            }
            self.timer_counter = self.timer_counter.wrapping_add(1);
        }

        self.prev_and_result = and_result as u8;
    }
}

impl IOMapped for Timer {
    fn read_byte(&self, address: u16) -> u8 {
        let r = match address {
            // FF04 DIV
            0xFF04 => (self.internal_counter >> 8) as u8, 

            // FF05 TIMA - Timer Counter (r/w)
            0xFF05 => self.timer_counter,

            // FF06 TMA - Timer Modulo (r/w) 
            0xFF06 => self.timer_modulo, 

            // FF07 TAC - Timer Control (r/w)
            0xFF07 => self.timer_frequency | ((self.timer_enabled as u8) << 2),
            
            _ => panic!("Invalid Timer read")
        };

        // println!("Timer R: {:#06x},{:#04x}", address, r);

        r
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        // println!("Timer W: {:#06x},{:#04x}", address, data);

        match address {
            // FF04 DIV
            // 0xFF04 => self.divider_clocks = 0,
            0xFF04 => self.internal_counter = 0,

            // FF05 TIMA - Timer Counter (r/w)
            0xFF05 => {
                // writing to TIMA in between overflow trigger prevents the overflow interrupt
                if self.timer_overflow_counter != 4 {
                    self.timer_counter = data;
                    self.timer_overflow = false;
                    self.timer_overflow_counter = 0;
                }
            },

            // FF06 TMA - Timer Modulo (r/w) 
            0xFF06 => self.timer_modulo = data,

            // FF07 TMC - Timer Control (r/w)
            0xFF07 => {
                self.timer_frequency = data & 0x3;
                self.timer_enabled = data & 0x4 != 0;
            },
            _ => println!("Timer: Invalid write")
        }
    }
}