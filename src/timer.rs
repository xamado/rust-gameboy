use core::cell::RefCell;

use crate::iomapped::IOMapped;
use crate::memorybus::MemoryBus;
use crate::cpu::Interrupts;

struct TimerRegisters {
    internal_counter: u16,
    timer_enabled: bool,
    timer_frequency: u8,
    timer_counter: u8,
    timer_modulo: u8,
    timer_overflow: bool,
    timer_overflow_counter: u8,
}

pub struct Timer {
    registers: RefCell<TimerRegisters>,
    prev_and_result: RefCell<u8>
}

const TIMER_FREQ_BIT : [u8; 4] = [
    9, // 0b00 ~ 4096 Hz
    3, // 0b01 ~ 262144 Hz
    5, // 0b10 ~ 65536 Hz
    7, // 0b11 ~ 16384 Hz
];

impl Timer {
    pub fn new() -> Self {
        Self {
            registers: RefCell::new(TimerRegisters {
                timer_enabled: false,
                timer_frequency: 0,
                timer_counter: 0,
                timer_modulo: 0,
                timer_overflow: false,
                timer_overflow_counter: 0,
                internal_counter: 0x17CC, // why ?
            }),

            prev_and_result: RefCell::new(0),
        }
    }

    pub fn tick(&self, bus: &MemoryBus) {
        let mut registers = self.registers.borrow_mut();

        registers.internal_counter = registers.internal_counter.wrapping_add(1);

        // TIMA overflow resets after 4 cycles
        if registers.timer_overflow {
            registers.timer_overflow_counter += 1;

            if registers.timer_overflow_counter >= 4 {
                registers.timer_counter = registers.timer_modulo;

                // raise the Timer interrupt
                let iif = bus.read_byte(0xFF0F) | (1 << Interrupts::Timer as u8);
                bus.write_byte(0xFF0F, iif);

                registers.timer_overflow = false;
                registers.timer_overflow_counter = 0;
            }
        }

        let bit = TIMER_FREQ_BIT[registers.timer_frequency as usize];
        let and_result = ((registers.internal_counter & (1 << bit)) >> bit) & (registers.timer_enabled as u16);

        // check for falling edge
        let mut prev_result = self.prev_and_result.borrow_mut();
        if and_result == 0 && *prev_result == 1 {
            if registers.timer_counter == 0xFF {
                registers.timer_overflow = true;
                registers.timer_overflow_counter = 0;
            }
            registers.timer_counter = registers.timer_counter.wrapping_add(1);
        }

        *prev_result = and_result as u8;
    }
}

impl IOMapped for Timer {
    fn read_byte(&self, address: u16) -> u8 {
        let registers = self.registers.borrow();

        match address {
            // FF04 DIV
            0xFF04 => (registers.internal_counter >> 8) as u8, 

            // FF05 TIMA - Timer Counter (r/w)
            0xFF05 => registers.timer_counter,

            // FF06 TMA - Timer Modulo (r/w) 
            0xFF06 => registers.timer_modulo, 

            // FF07 TAC - Timer Control (r/w)
            0xFF07 => 0xF8 | registers.timer_frequency | ((registers.timer_enabled as u8) << 2),
            
            _ => panic!("Invalid Timer read")
        }
    }

    fn write_byte(&self, address: u16, data: u8) {
        let mut registers = self.registers.borrow_mut();

        match address {
            // FF04 DIV
            0xFF04 => registers.internal_counter = 0,

            // FF05 TIMA - Timer Counter (r/w)
            0xFF05 => {
                // writing to TIMA in between overflow trigger prevents the overflow interrupt
                // TODO: For this to work properly we need cycle accurate emulation...
                if registers.timer_overflow_counter != 4 {
                    registers.timer_counter = data;
                    registers.timer_overflow = false;
                    registers.timer_overflow_counter = 0;
                }
            },

            // FF06 TMA - Timer Modulo (r/w) 
            0xFF06 => registers.timer_modulo = data,

            // FF07 TMC - Timer Control (r/w)
            0xFF07 => {
                registers.timer_frequency = data & 0x3;
                registers.timer_enabled = data & 0x4 != 0;
            },
            _ => println!("Timer: Invalid write")
        }
    }
}