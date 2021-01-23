use std::rc::Rc;
use core::cell::RefCell;
use closure::closure;

use crate::iomapped::IOMapped;
use crate::memorybus::MemoryBus;
use crate::memory::Memory;
use crate::cpu::CPU;
use crate::rom::ROM;
use crate::bootrom::BootROM;
use crate::ppu::PPU;
use crate::apu::APU;
use crate::screen::Screen;
use crate::joystick::Joystick;
use crate::timer::Timer;
use crate::serial::Serial;
use crate::debugger::Debugger;

#[derive(Clone, Copy, PartialEq)]
pub enum GameBoyModel {
    DMG,
    GBC
}

pub struct Machine {
    model: GameBoyModel,
    cpu: Rc<RefCell<CPU>>,
    ppu: Rc<RefCell<PPU>>,
    apu: Rc<RefCell<APU>>,
    ram1: Rc<RefCell<Memory>>,
    ram2: Rc<RefCell<Memory>>,
    hram: Rc<RefCell<Memory>>,
    bootrom: Rc<RefCell<BootROM>>,
    rom: Rc<RefCell<ROM>>,
    screen: Rc<RefCell<Screen>>,
    joystick: Rc<RefCell<Joystick>>,
    bus: Rc<RefCell<MemoryBus>>,
    timer: Rc<RefCell<Timer>>,
    serial: Rc<RefCell<Serial>>,
    debugger: Option<Box<Debugger>>,
}

impl Machine {
    pub fn new(rom: ROM) -> Self {
        let model = rom.get_rom_type();

        let bus = Rc::new(RefCell::new(MemoryBus::new()));
        let screen = Rc::new(RefCell::new(Screen::new(model)));

        let ram2 = match model {
            GameBoyModel::DMG => Rc::new(RefCell::new(Memory::new(0xD000, 0x1000, 1))),
            GameBoyModel::GBC => Rc::new(RefCell::new(Memory::new(0xD000, 0x7000, 7))),
        };

        Self {
            model,
            bootrom: Rc::new(RefCell::new(BootROM::new())),
            rom: Rc::new(RefCell::new(rom)),
            ram1: Rc::new(RefCell::new(Memory::new(0xC000, 0x1000, 1))),
            ram2,
            hram: Rc::new(RefCell::new(Memory::new(0xFF80, 0x7F, 1))),
            bus: bus.clone(),
            joystick: Rc::new(RefCell::new(Joystick::new(Rc::clone(&bus)))),
            screen: screen.clone(),
            cpu: Rc::new(RefCell::new(CPU::new(model, Rc::clone(&bus)))),
            ppu: Rc::new(RefCell::new(PPU::new(model, Rc::clone(&bus), screen))),
            apu: Rc::new(RefCell::new(APU::new())),
            timer: Rc::new(RefCell::new(Timer::new(Rc::clone(&bus)))),
            serial: Rc::new(RefCell::new(Serial::new())),
            debugger: None,
        }
    }
 
    pub fn start(&mut self, skip_bootrom: bool) {
        let bus = self.bus.borrow();

        if !skip_bootrom {
            match self.model {
                GameBoyModel::DMG => {
                    self.bootrom.borrow_mut().open("DMG_ROM.bin");
                    bus.map_range_read(0x0000..=0x00FF, closure!(clone self.bootrom, |addr| bootrom.borrow().read_byte(addr)));
                }
                GameBoyModel::GBC => {
                    self.bootrom.borrow_mut().open("CGB_ROM.bin");
                    bus.map_range_read(0x0000..=0x00FF, closure!(clone self.bootrom, |addr| bootrom.borrow().read_byte(addr)));
                    bus.map_range_read(0x0200..=0x08FF, closure!(clone self.bootrom, |addr| bootrom.borrow().read_byte(addr)));
                }
            }           
        }
        
        // 0000-7FFF - ROM 
        bus.map_range_read(0x0000..=0x7FFF, closure!(clone self.rom, |addr| rom.borrow().read_byte(addr)));
        bus.map_range_write(0x0000..=0x7FFF, closure!(clone self.rom, |addr, data| rom.borrow_mut().write_byte(addr, data)));

        // 8000-9FFF - VRAM
        bus.map_range_read(0x8000..=0x9FFF, closure!(clone self.ppu, |addr| ppu.borrow().read_byte(addr)));
        bus.map_range_write(0x8000..=0x9FFF, closure!(clone self.ppu, |addr, data| ppu.borrow_mut().write_byte(addr, data)));

        // A000-BFFF - ROM External RAM
        bus.map_range_read(0xA000..=0xBFFF, closure!(clone self.rom, |addr| rom.borrow().read_byte(addr)));
        bus.map_range_write(0xA000..=0xBFFF, closure!(clone self.rom, |addr, data| rom.borrow_mut().write_byte(addr, data)));

        // C000-CFFF - WRAM Bank 0
        bus.map_range_read(0xC000..=0xCFFF, closure!(clone self.ram1, |addr| ram1.borrow().read_byte(addr)));
        bus.map_range_write(0xC000..=0xCFFF, closure!(clone self.ram1, |addr, data| ram1.borrow_mut().write_byte(addr, data)));

        // D000-DFFF - WRAM Banks 1-7
        bus.map_range_read(0xD000..=0xDFFF, closure!(clone self.ram2, |addr| ram2.borrow().read_byte(addr)));
        bus.map_range_write(0xD000..=0xDFFF, closure!(clone self.ram2, |addr, data| ram2.borrow_mut().write_byte(addr, data)));

        // E000-EFFF - "ECHO RAM" WRAM Bank 0 
        bus.map_range_read(0xE000..=0xEFFF, closure!(clone self.ram1, |addr| ram1.borrow().read_byte(addr - 0xE000 + 0xC000)));
        bus.map_range_write(0xE000..=0xEFFF, closure!(clone self.ram1, |addr, data| ram1.borrow_mut().write_byte(addr - 0xE000 + 0xC000, data)));

        // F000-FDFF - "ECHO RAM" WRAM Banks 1-7 
        bus.map_range_read(0xF000..=0xFDFF, closure!(clone self.ram2, |addr| ram2.borrow().read_byte(addr - 0xF000 + 0xD000)));
        bus.map_range_write(0xF000..=0xFDFF, closure!(clone self.ram2, |addr, data| ram2.borrow_mut().write_byte(addr - 0xF000 + 0xD000, data)));

        // FE00-FE9F - OAM Table
        bus.map_range_read(0xFE00..=0xFE9F, closure!(clone self.ppu, |addr| ppu.borrow().read_oam_byte(addr - 0xFE00)));
        bus.map_range_write(0xFE00..=0xFE9F, closure!(clone self.ppu, |addr, data| ppu.borrow_mut().write_oam_byte(addr - 0xFE00, data)));

        // FEA0-FEFF Unusable

        // FF00 - Joystick Register
        bus.map_address_read(0xFF00, closure!(clone self.joystick, |addr| joystick.borrow().read_byte(addr)));
        bus.map_address_write(0xFF00, closure!(clone self.joystick, |addr, data| joystick.borrow_mut().write_byte(addr, data)));

        // FF01-FF02 - Serial 
        bus.map_range_read(0xFF01..=0xFF02, closure!(clone self.serial, |addr| serial.borrow().read_byte(addr)));
        bus.map_range_write(0xFF01..=0xFF02, closure!(clone self.serial, |addr, data| serial.borrow_mut().write_byte(addr, data)));

        // FF04-FF07 - Timer
        bus.map_range_read(0xFF04..=0xFF07, closure!(clone self.timer, |addr| timer.borrow().read_byte(addr)));
        bus.map_range_write(0xFF04..=0xFF07, closure!(clone self.timer, |addr, data| timer.borrow_mut().write_byte(addr, data)));

        // FF0F - Interrupt Enable
        bus.map_address_read(0xFF0F, closure!(clone self.cpu, |addr| cpu.borrow().read_byte(addr)));
        bus.map_address_write(0xFF0F, closure!(clone self.cpu, |addr, data| cpu.borrow_mut().write_byte(addr, data)));

        // FF10-FF3F - APU 
        bus.map_range_read(0xFF10..=0xFF3F, closure!(clone self.apu, |addr| apu.borrow().read_byte(addr)));
        bus.map_range_write(0xFF10..=0xFF3F, closure!(clone self.apu, |addr, data| apu.borrow_mut().write_byte(addr, data)));

        // FF40-FF4B - PPU Registers
        bus.map_range_read(0xFF40..=0xFF4B, closure!(clone self.ppu, |addr| ppu.borrow().read_byte(addr)));
        bus.map_range_write(0xFF40..=0xFF4B, closure!(clone self.ppu, |addr, data| ppu.borrow_mut().write_byte(addr, data)));

        // FF4F - VRAM Bank Register
        if let GameBoyModel::GBC = self.model {
            bus.map_address_read(0xFF4F, closure!(clone self.ppu, |_addr| ppu.borrow().get_vram_bank()));
            bus.map_address_write(0xFF4F, closure!(clone self.ppu, |_addr, data| ppu.borrow_mut().set_vram_bank(data & 0x1)));
        }

        // FF50 - DISABLE BOOTROM
        bus.map_address_write(0xFF50, closure!(clone self.bus, |_addr, _data| {
            bus.borrow().unmap_range_read(0x0000..=0x00FF);
            bus.borrow().unmap_range_read(0x0200..=0x08FF);
        }));

        // FF51-FF55 - HDMA Transfer (GBC)
        match self.model {
            GameBoyModel::GBC => {
                bus.map_range_write(0xFF51..=0xFF55, closure!(clone self.ppu, |addr, data| {
                    ppu.borrow_mut().write_byte(addr, data);
                }));
        
                bus.map_range_read(0xFF51..=0xFF55, closure!(clone self.ppu, |addr| {
                    ppu.borrow().read_byte(addr)
                }));
            },
            _ => {
                bus.map_range_read(0xFF51..=0xFF55, closure!(clone self.ppu, |_addr| {
                    0xFF
                }));
            }
        }

        // FF68 - FF6A - Palette Data
        if let GameBoyModel::GBC = self.model {
            bus.map_range_write(0xFF68..=0xFF6B, closure!(clone self.ppu, |addr, data| ppu.borrow_mut().write_byte(addr, data)));
            bus.map_range_read(0xFF68..=0xFF6B, closure!(clone self.ppu, |addr| ppu.borrow().read_byte(addr)));
        };

        // FF70 - WRAM Bank Switch Register
        if let GameBoyModel::GBC = self.model {
            bus.map_address_write(0xFF70, closure!(clone self.ram2, |_addr, data| {
                ram2.borrow_mut().ff70 = data & 0x7;

                let bank = if (data & 0x7) != 0 { (data & 0x7) - 1 } else { 0 };
                ram2.borrow_mut().switch_bank(bank);
            }));

            bus.map_address_read(0xFF70, closure!(clone self.ram2, |_addr| (0xF8 | ram2.borrow().ff70)));
        }
        
        // FF80-FFFE - HIGH RAM
        bus.map_range_read(0xFF80..=0xFFFE, closure!(clone self.hram, |addr| hram.borrow().read_byte(addr)));
        bus.map_range_write(0xFF80..=0xFFFE, closure!(clone self.hram, |addr, data| hram.borrow_mut().write_byte(addr, data)));

        // FFFF Interrupt Register
        bus.map_address_read(0xFFFF, closure!(clone self.cpu, |addr| cpu.borrow().read_byte(addr)));
        bus.map_address_write(0xFFFF, closure!(clone self.cpu, |addr, data| cpu.borrow_mut().write_byte(addr, data)));

        // Advance PC to 0x100 if we are skipping the bootrom
        self.cpu.borrow_mut().set_initial_state(skip_bootrom);
        self.ppu.borrow_mut().set_initial_state(skip_bootrom);
    }

    pub fn stop(&mut self) {
        self.rom.borrow().close();
    }

    pub fn get_screen(&self) -> &Rc<RefCell<Screen>> {
        &self.screen
    }

    pub fn get_joystick(&self) -> &Rc<RefCell<Joystick>> {
        &self.joystick
    }

    pub fn get_audio_buffer(&self) -> Vec<i16> {
        self.apu.borrow_mut().get_audio_samples()
    }
    
    pub fn step(&mut self) {
        if let Some(debugger) = &mut self.debugger {
            if debugger.is_stopped() {
                return;
            }
        }

        self.tick();

        if let Some(debugger) = &mut self.debugger {
            debugger.process(&*self.cpu.borrow(), &*self.ppu.borrow(), &*self.bus.borrow());
        }
    }

    pub fn is_stopped(&self) -> bool {
        if let Some(debugger) = &self.debugger {
            return debugger.is_stopped();
        }

        false
    }

    fn tick(&mut self) {
        let cpu_cycles = self.cpu.borrow().tick();
        let clocks = cpu_cycles * 4;

        for _ in 0..clocks {
            self.timer.borrow_mut().tick();
            self.ppu.borrow_mut().tick();
            self.apu.borrow_mut().tick();
        }
    }

    pub fn attach_debugger(&mut self, debugger: Debugger) {
        self.debugger = Some(Box::new(debugger));
    }

    pub fn debugger_continue(&mut self) {
        if let Some(debugger) = &mut self.debugger {
            if debugger.is_stopped() {
                debugger.resume();
            }
            else {
                debugger.stop(&*self.cpu.borrow(), &*self.ppu.borrow());
            }
        }
    }

    pub fn debugger_step(&mut self) {
        self.tick();

        if let Some(debugger) = &self.debugger {
            debugger.print_trace(&*self.cpu.borrow(), &*self.ppu.borrow());
        }
    }
}
