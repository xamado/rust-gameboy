use core::cell::RefCell;
use crate::cpu::CPU;
use crate::ppu::PPU;
use crate::memorybus::MemoryBus;

pub struct Breakpoint {
    address: u16
}

pub struct Watchpoint {
    address: u16,
    value: RefCell<u8>
}

struct DebuggerState {
    stopped: bool,
}

pub struct Debugger {
    breakpoints: Vec<Breakpoint>,
    watchpoints: Vec<Watchpoint>,
    state: RefCell<DebuggerState>,
}

impl Debugger {
    pub fn new() -> Self {
        Self {
            breakpoints: vec!(),
            watchpoints: vec!(),
            state: RefCell::new(DebuggerState {
                stopped: false
            }),
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.state.borrow().stopped
    }

    pub fn resume(&self) {
        self.state.borrow_mut().stopped = false;
    }

    pub fn stop(&self, cpu: &CPU, ppu: &PPU) {
        self.print_trace(cpu, ppu);
        self.state.borrow_mut().stopped = true;
    }

    pub fn add_breakpoint(&mut self, addr: u16) {
        self.breakpoints.push(Breakpoint {
            address: addr
        });
    }

    pub fn add_watchpoint(&mut self, addr: u16) {
        self.watchpoints.push(Watchpoint {
            address: addr,
            value: RefCell::new(0)
        });
    }

    pub fn process(&self, cpu: &CPU, ppu: &PPU, bus: &MemoryBus) {
        let cpu_state = cpu.get_debug_state();

        for b in &self.breakpoints {
            if b.address == cpu_state.pc {
                self.print_trace(&cpu, ppu);
                self.state.borrow_mut().stopped = true;
                break;
            }
        }

        for w in &self.watchpoints {
            let v = bus.read_byte(w.address);
            if v != *w.value.borrow() {
                *w.value.borrow_mut() = v;

                println!("@{:06X} Watch: {:#06X} = {:#04X}", cpu_state.pc, w.address, v);
            }
        }
    }

    pub fn print_trace(&self, cpu: &CPU, ppu: &PPU) {
        let cpu_state = cpu.get_debug_state();
        let ppu_state = ppu.get_debug_state();

        println!("@{:#06X} {} | AF: {:#06X} | BC: {:#06X} | DE: {:#06X} | HL: {:#06X} | LY: {} | STAT: {:#04X} | LCDC: {:#04X} | CNT: {}", 
            cpu_state.pc, 
            cpu_state.next_opcode, 
            cpu_state.af, 
            cpu_state.bc, 
            cpu_state.de, 
            cpu_state.hl, 
            ppu_state.ly, 
            ppu_state.stat, 
            ppu_state.lcdc, 
            ppu_state.cycles
        );
    }
}
