use std::rc::Rc;
use core::cell::RefCell;

use crate::iomapped::IOMapped;
use crate::memorybus::MemoryBus;
use crate::cpu::Interrupts;

pub struct Timer {
    bus: Rc<RefCell<MemoryBus>>,
    divider_clocks: u16,
    divider_counter: u8,
    timer_enabled: bool,
    timer_frequency: u8,
    timer_counter: u8,
    timer_clocks: u16,
    timer_modulo: u8,
    timer_overflow: bool
}

const TIMER_FREQ_CYCLES : [u16; 4] = [
    1024,   // 4096 hz
    16,     // 262144 hz
    64,     // 65536 hz
    256,    // 16384 hz
];

impl Timer {
    pub fn new(bus: Rc<RefCell<MemoryBus>>) -> Self {
        Self {
            bus,
            divider_clocks: 0,
            divider_counter: 0,
            timer_enabled: false,
            timer_clocks: 0,
            timer_frequency: 0,
            timer_counter: 0,
            timer_modulo: 0,
            timer_overflow: false
        }
    }

    pub fn step(&mut self, elapsed_cycles: u8) {
        // overflow and reset are delayed 1 cycle
        if self.timer_overflow {
            self.timer_counter = self.timer_modulo;

            // raise the Timer interrupt
            let iif = self.bus.borrow().read_byte(0xFF0F) | (1 << Interrupts::Timer as u8);
            self.bus.borrow_mut().write_byte(0xFF0F, iif);

            self.timer_overflow = false;
        }

        if self.timer_enabled {
            self.timer_clocks += (elapsed_cycles * 4) as u16;
            
            while self.timer_clocks >= TIMER_FREQ_CYCLES[self.timer_frequency as usize] {
                self.timer_clocks -= TIMER_FREQ_CYCLES[self.timer_frequency as usize];

                let prev = self.timer_counter;
                self.timer_counter = self.timer_counter.wrapping_add(1);
                if self.timer_counter < prev {
                    self.timer_overflow = true;
                }
            }
        }

        self.divider_clocks = self.divider_clocks.wrapping_add(elapsed_cycles as u16);

        while self.divider_clocks >= 16
        {
            self.divider_clocks -= 16;

            // tick divider reg
            if self.divider_clocks == 16 {
                self.divider_counter = self.divider_counter.wrapping_add(1);
            }
        }
    }

    fn set_clock_frequency(&mut self, frequency: u8) {
        self.timer_frequency = frequency;
    }
}

impl IOMapped for Timer {
    fn read_byte(&self, address: u16) -> u8 {
        let r = match address {
            // FF04 DIV
            0xFF04 => self.divider_counter, 

            // FF05 TIMA - Timer Counter (r/w)
            0xFF05 => self.timer_counter,

            // FF06 TMA - Timer Modulo (r/w) 
            0xFF06 => self.timer_modulo, 

            // FF07 TMC - Timer Control (r/w)
            0xFF07 => self.timer_frequency | (if self.timer_enabled { 0x4 } else { 0 }),
            
            _ => panic!("Invalid Timer read")
        };

        println!("Timer R: {:#06x},{:#04x}", address, r);

        r
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        println!("Timer W: {:#06x},{:#04x}", address, data);

        match address {
            // FF04 DIV
            0xFF04 => self.divider_clocks = 0,

            // FF05 TIMA - Timer Counter (r/w)
            0xFF05 => {
                // writing to TIMA in between overflow trigger prevents the overflow interrupt
                self.timer_counter = data;
                self.timer_overflow = false;
            },

            // FF06 TMA - Timer Modulo (r/w) 
            0xFF06 => self.timer_modulo = data,

            // FF07 TMC - Timer Control (r/w)
            0xFF07 => {
                let req_freq = data & 0x3;
                if self.timer_frequency != req_freq {
                    self.set_clock_frequency(req_freq);
                }

                self.timer_enabled = data & 0x4 != 0;
            },
            _ => println!("Timer: Invalid write")
        }
    }
}