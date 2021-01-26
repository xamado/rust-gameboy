use std::fmt;

use crate::bus::{CPUMemoryBus, PPUMemoryBus};
use crate::memory::Memory;
use crate::cpu::{CPU, CPUInterrupts};
use crate::rom::ROM;
use crate::bootrom::BootROM;
use crate::ppu::PPU;
use crate::apu::APU;
use crate::screen::Screen;
use crate::joystick::Joystick;
use crate::timer::Timer;
use crate::serial::Serial;
use crate::debugger::Debugger;
use crate::joystick::JoystickButton;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GameBoyModel {
    DMG,
    GBC
}

impl fmt::Display for GameBoyModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Machine {
    model: GameBoyModel,
    bootrom_enabled: bool,
    screen: Screen,
    cpu: CPU,
    ppu: PPU,    
    apu: APU,
    ram1: Memory,
    ram2: Memory,
    hram: Memory,
    bootrom: BootROM,
    rom: ROM,
    joystick: Joystick,
    timer: Timer,
    serial: Serial,
    debugger: Option<Box<Debugger>>,
    interrupts: CPUInterrupts,
}

impl Machine {
    pub fn new(rom: ROM, force_model: Option<GameBoyModel>) -> Self {
        let model = match force_model {
            Some(model) => model,
            None => rom.get_rom_type()
        };

        Self {
            model,
            cpu: CPU::new(model),
            bootrom_enabled: false,
            bootrom: BootROM::new(),
            timer: Timer::new(),
            interrupts: CPUInterrupts::new(),
            ppu: PPU::new(model),
            apu: APU::new(),
            ram1: Memory::new(0xC000, 0x1000, 1),
            ram2: match model {
                GameBoyModel::DMG => Memory::new(0xD000, 0x1000, 1),
                GameBoyModel::GBC => Memory::new(0xD000, 0x7000, 7),
            },
            hram: Memory::new(0xFF80, 0x7F, 1),
            rom,
            joystick: Joystick::new(),
            serial: Serial::new(),
            screen: Screen::new(model),
            debugger: None,
        }
    }
 
    pub fn start(&mut self, skip_bootrom: bool) {
        self.bootrom_enabled = !skip_bootrom;
        if !skip_bootrom {
            match self.model {
                GameBoyModel::DMG => {
                    self.bootrom.open("DMG_ROM.bin");
                }
                GameBoyModel::GBC => {
                    self.bootrom.open("CGB_ROM.bin");
                }
            }
        }
        
        // Advance PC to 0x100 if we are skipping the bootrom
        self.cpu.set_initial_state(skip_bootrom);
        self.ppu.set_initial_state(skip_bootrom);
    }

    pub fn stop(&mut self) {
        self.rom.close();
    }

    pub fn get_model(&self) -> GameBoyModel {
        self.model
    }

    pub fn is_vblank(&self) -> bool {
        self.screen.is_vblank()
    }

    pub fn get_framebuffer(&mut self) -> &[u32] {
        self.screen.get_framebuffer()
    }

    pub fn inject_input(&mut self, b : JoystickButton, is_pressed: bool) {
        self.joystick.inject(&mut self.interrupts, b, is_pressed);
    }

    pub fn get_audio_buffer(&mut self) -> Vec<i16> {
        self.apu.consume_audio_samples()
    }
    
    pub fn step(&mut self) {
        if let Some(debugger) = &mut self.debugger {
            if debugger.is_stopped() {
                return;
            }
        }

        self.tick();

        if let Some(debugger) = &mut self.debugger {
            debugger.process(&self.cpu, &self.ppu);
        }
    }

    pub fn is_stopped(&self) -> bool {
        if let Some(debugger) = &self.debugger {
            return debugger.is_stopped();
        }

        false
    }

    fn tick(&mut self) {
        let cpu_cycles = self.cpu.tick(&mut CPUMemoryBus {
            bootrom_enabled: &mut self.bootrom_enabled,
            model: self.model,
            ppu: &mut self.ppu,
            apu: &mut self.apu,
            ram1: &mut self.ram1,
            ram2: &mut self.ram2,
            hram: &mut self.hram,
            bootrom: &mut self.bootrom,
            rom: &mut self.rom,
            joystick: &mut self.joystick,
            serial: &mut self.serial,
            timer: &mut self.timer,
            interrupts: &mut self.interrupts
        });
        let clocks = cpu_cycles * 4;

        for _ in 0..clocks {
            self.timer.tick(&mut self.interrupts);
            
            self.ppu.tick(&mut PPUMemoryBus {
                rom: &mut self.rom,
                ram1: &mut self.ram1,
                ram2: &mut self.ram2,
            }, &mut self.interrupts, &mut self.screen);

            self.apu.tick();
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
                debugger.stop(&self.cpu, &self.ppu);
            }
        }
    }

    pub fn debugger_step(&mut self) {
        self.tick();

        if let Some(debugger) = &self.debugger {
            debugger.print_trace(&self.cpu, &self.ppu);
        }
    }
}
