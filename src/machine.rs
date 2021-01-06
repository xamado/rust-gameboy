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

pub struct Machine {
    cpu: Rc<RefCell<CPU>>,
    ppu: Rc<RefCell<PPU>>,
    ram: Rc<RefCell<Memory>>,
    bootrom: Rc<RefCell<ROM>>,
    rom: Rc<RefCell<ROM>>,
    screen: Rc<RefCell<Screen>>,
    joystick: Rc<RefCell<Joystick>>,
    bus: Rc<RefCell<MemoryBus>>,
    timer: Rc<RefCell<Timer>>
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
            timer: Rc::new(RefCell::new(Timer::new(Rc::clone(&bus))))
        }
    }

    pub fn start(&mut self) {
        // self.bootrom.borrow_mut().open("DMG_ROM.bin");
        // self.rom.borrow_mut().open("Tetris (World) (Rev A).gb");
        self.rom.borrow_mut().open("Super Mario Land (World).gb");
        // self.rom.borrow_mut().open("cpu_instrs.gb");
        // self.rom.borrow_mut().open("Dr. Mario (World).gb");
        // self.rom.borrow_mut().open("TESTGAME.gb");
        // self.rom.borrow_mut().open("opus5.gb");

        // self.bus.borrow_mut().map(0x0000..=0x00FF, self.bootrom.clone());
        self.bus.borrow_mut().map(0x0000..=0x7FFF, self.rom.clone());
        self.bus.borrow_mut().map(0xFF00..=0xFF00, self.joystick.clone());
        self.bus.borrow_mut().map(0xFF04..=0xFF07, self.timer.clone());
        self.bus.borrow_mut().map(0xFF40..=0xFF49, self.ppu.clone());
        self.bus.borrow_mut().map(0x8000..=0xFFFF, self.ram.clone());     
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
        let instr_cycles = self.cpu.borrow_mut().step();
        let cpu_cycles = instr_cycles * 4;

        self.ppu.borrow_mut().step(cpu_cycles);
        self.timer.borrow_mut().step(cpu_cycles);
    }
}