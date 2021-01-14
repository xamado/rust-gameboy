use std::rc::Rc;
use std::path::PathBuf;
use std::fs::File;
use std::io::prelude::*;
use core::cell::RefCell;

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

pub struct Machine {
    cpu: Rc<RefCell<CPU>>,
    ppu: Rc<RefCell<PPU>>,
    apu: Rc<RefCell<APU>>,
    ram: Rc<RefCell<Memory>>,
    bootrom: Rc<RefCell<BootROM>>,
    rom: Rc<RefCell<ROM>>,
    screen: Rc<RefCell<Screen>>,
    joystick: Rc<RefCell<Joystick>>,
    bus: Rc<RefCell<MemoryBus>>,
    timer: Rc<RefCell<Timer>>,
    serial: Rc<RefCell<Serial>>,
    debugger: Option<Box<Debugger>>,
    rom_filename: String,
}

impl Machine {
    pub fn new() -> Self {
        let bus = Rc::new(RefCell::new(MemoryBus::new()));
        let screen = Rc::new(RefCell::new(Screen::new()));

        Self {
            rom_filename: String::from(""),
            bootrom: Rc::new(RefCell::new(BootROM::new())),
            rom: Rc::new(RefCell::new(ROM::new())),
            ram: Rc::new(RefCell::new(Memory::new())),
            bus: bus.clone(),
            joystick: Rc::new(RefCell::new(Joystick::new(Rc::clone(&bus)))),
            screen: screen.clone(),
            cpu: Rc::new(RefCell::new(CPU::new(Rc::clone(&bus)))),
            ppu: Rc::new(RefCell::new(PPU::new(Rc::clone(&bus), screen))),
            apu: Rc::new(RefCell::new(APU::new())),
            timer: Rc::new(RefCell::new(Timer::new(Rc::clone(&bus)))),
            serial: Rc::new(RefCell::new(Serial::new())),
            debugger: None,
        }
    }

    pub fn start(&mut self, skip_bootrom: bool) {
        if !skip_bootrom {
            self.bootrom.borrow_mut().open("DMG_ROM.bin");
            self.bus.borrow_mut().map(0x0000..=0x00FF, self.bootrom.clone());
        }
        else {
            self.cpu.borrow_mut().set_start_pc(0x100);
        }
        
        // TODO: Maybe just map read/write function pointers here?? can they be member funcs?
        
        self.bus.borrow_mut().map(0x0000..=0x7FFF, self.rom.clone());  
        self.bus.borrow_mut().map(0x8000..=0x9FFF, self.ppu.clone());   // VRAM
        self.bus.borrow_mut().map(0xA000..=0xBFFF, self.rom.clone());   // External RAM
        self.bus.borrow_mut().map(0xC000..=0xDFFF, self.ram.clone());   // Internal RAM
        // 0xE000..=0xFDFF Unusable
        self.bus.borrow_mut().map(0xFE00..=0xFE9F, self.ppu.clone());   // OAM Table
        // 0XFEA0..=0xFEFF Unusable
        self.bus.borrow_mut().map(0xFF00..=0xFF00, self.joystick.clone());
        self.bus.borrow_mut().map(0xFF01..=0xFF02, self.serial.clone());
        self.bus.borrow_mut().map(0xFF04..=0xFF07, self.timer.clone());
        self.bus.borrow_mut().map(0xFF0F..=0xFF0F, self.cpu.clone());   // Interrupts
        self.bus.borrow_mut().map(0xFF10..=0xFF3F, self.apu.clone());
        self.bus.borrow_mut().map(0xFF40..=0xFF4B, self.ppu.clone());
        // 0xFF4c..=0xFF7F Unusable
        self.bus.borrow_mut().map(0xFF80..=0xFFFE, self.ram.clone());   // HIGH RAM
        self.bus.borrow_mut().map(0xFFFF..=0xFFFF, self.cpu.clone());   // Interrupts

    }

    pub fn load_rom(&mut self, file: &str) {
        self.rom.borrow_mut().open(file);
        self.rom_filename = String::from(file);

        // load ram contents if present
        let rom_filename = self.rom_filename.to_owned();
        let mut path = PathBuf::from(rom_filename);
        path.set_extension("sav");

        if path.exists() {
            let bytes = std::fs::read(&path).expect("Failed to open RAM");
            self.rom.borrow_mut().set_ram_contents(&bytes);
        }
    }


    pub fn save_status(&self) -> std::io::Result<()> {
        let rom_filename = self.rom_filename.to_owned();
        let mut path = PathBuf::from(rom_filename);
        path.set_extension("sav");
        
        if let Some(ram) = self.rom.borrow().get_ram_contents() {
            let mut file = File::create(path)?;
            file.write_all(&ram[0..ram.len()])?;
        }

        Ok(())
    }

    pub fn get_screen(&self) -> &Rc<RefCell<Screen>> {
        &self.screen
    }

    pub fn get_joystick(&self) -> &Rc<RefCell<Joystick>> {
        &self.joystick
    }

    pub fn get_audio_buffer(&self) -> Vec<u16> {
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
            debugger.process(self.cpu.borrow(), self.bus.borrow());
        }
    }

    fn tick(&mut self) {
        let cpu_cycles = self.cpu.borrow_mut().step();
        let clocks = cpu_cycles * 4;

        self.ppu.borrow_mut().step(clocks);

        let mut timer = self.timer.borrow_mut();
        for _ in 0..clocks {
            timer.step_clock();
        }

        // let apu_clocks = clocks / 2;
        for _ in 0..clocks {
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
                debugger.stop(self.cpu.borrow());
            }
        }
    }

    pub fn debugger_step(&mut self) {
        self.tick();

        if let Some(debugger) = &self.debugger {
            debugger.print_trace(&self.cpu.borrow());
        }
    }
}