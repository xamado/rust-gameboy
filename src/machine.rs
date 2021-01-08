use std::rc::Rc;
use core::cell::RefCell;

use crate::memorybus::MemoryBus;
use crate::memory::Memory;
use crate::cpu::CPU;
use crate::rom::ROM;
use crate::ppu::PPU;
use crate::screen::Screen;
use crate::joystick::Joystick;
use crate::timer::Timer;
use crate::serial::Serial;

pub struct Machine {
    cpu: Rc<RefCell<CPU>>,
    ppu: Rc<RefCell<PPU>>,
    ram: Rc<RefCell<Memory>>,
    bootrom: Rc<RefCell<ROM>>,
    rom: Rc<RefCell<ROM>>,
    screen: Rc<RefCell<Screen>>,
    joystick: Rc<RefCell<Joystick>>,
    bus: Rc<RefCell<MemoryBus>>,
    timer: Rc<RefCell<Timer>>,
    serial: Rc<RefCell<Serial>>
}

impl Machine {
    pub fn new() -> Self {
        let bus = Rc::new(RefCell::new(MemoryBus::new()));
        let screen = Rc::new(RefCell::new(Screen::new()));

        Self {
            bootrom: Rc::new(RefCell::new(ROM::new())),
            rom: Rc::new(RefCell::new(ROM::new())),
            ram: Rc::new(RefCell::new(Memory::new())),
            bus: bus.clone(),
            joystick: Rc::new(RefCell::new(Joystick::new(Rc::clone(&bus)))),
            screen: screen.clone(),
            cpu: Rc::new(RefCell::new(CPU::new(Rc::clone(&bus)))),
            ppu: Rc::new(RefCell::new(PPU::new(Rc::clone(&bus), screen))),
            timer: Rc::new(RefCell::new(Timer::new(Rc::clone(&bus)))),
            serial: Rc::new(RefCell::new(Serial::new()))
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
        
        self.bus.borrow_mut().map(0x0000..=0x7FFF, self.rom.clone());  
        self.bus.borrow_mut().map(0x8000..=0x9FFF, self.ram.clone());   // VRAM
        self.bus.borrow_mut().map(0xA000..=0xBFFF, self.rom.clone());   // External RAM
        self.bus.borrow_mut().map(0xC000..=0xDFFF, self.ram.clone());   // Internal RAM
        // 0xE000..=0xFDFF Unusable
        self.bus.borrow_mut().map(0xFE00..=0xFE9F, self.ram.clone());   // OAM Table
        // 0XFEA0..=0xFEFF Unusable
        self.bus.borrow_mut().map(0xFF00..=0xFF00, self.joystick.clone());
        self.bus.borrow_mut().map(0xFF01..=0xFF02, self.serial.clone());
        self.bus.borrow_mut().map(0xFF04..=0xFF07, self.timer.clone());
        self.bus.borrow_mut().map(0xFF0F..=0xFF0F, self.ram.clone());
        self.bus.borrow_mut().map(0xFF40..=0xFF49, self.ppu.clone());
        // 0xFF4c..=0xFF7F Unusable
        self.bus.borrow_mut().map(0xFF80..=0xFFFE, self.ram.clone());   // HIGH RAM
        self.bus.borrow_mut().map(0xFFFF..=0xFFFF, self.ram.clone());   // Interrupt Register // TODO: Map this to cpu directly?
    }

    pub fn load_rom(&self, file: &str) {
        self.rom.borrow_mut().open(file);
    }

    pub fn update_frame(&self) {
        let mut ly = self.bus.borrow().read_byte(0xFF44);

        while ly != 0 {
            self.step();

            ly = self.bus.borrow().read_byte(0xFF44);
        }

        while ly < 144 {
            self.step();

            ly = self.bus.borrow().read_byte(0xFF44);
        }
    }

    pub fn get_screen(&self) -> &Rc<RefCell<Screen>> {
        &self.screen
    }

    pub fn get_joystick(&self) -> &Rc<RefCell<Joystick>> {
        &self.joystick
    }

    fn step(&self) {
        let cpu_cycles = self.cpu.borrow_mut().step();
        let clocks = cpu_cycles * 4;

        self.ppu.borrow_mut().step(clocks);

        for _ in 0..clocks {
            self.timer.borrow_mut().step_clock();
        }
    }
}