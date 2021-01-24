use core::cell::RefCell;
use std::fmt;

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
    screen: Screen,
    hardware: MemoryBus,
    debugger: Option<Box<Debugger>>,
}

impl Machine {
    pub fn new(rom: ROM, force_model: Option<GameBoyModel>) -> Self {
        let model = match force_model {
            Some(model) => model,
            None => rom.get_rom_type()
        };

        Self {
            model,
            hardware: MemoryBus {
                bootrom_enabled: RefCell::new(false),
                bootrom: BootROM::new(),
                model,
                timer: Timer::new(),
                cpu: CPU::new(model),
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
            },
            screen: Screen::new(model),
            debugger: None,
        }
    }
 
    pub fn start(&mut self, skip_bootrom: bool) {
        *(self.hardware.bootrom_enabled.borrow_mut()) = !skip_bootrom;
        if !skip_bootrom {
            match self.model {
                GameBoyModel::DMG => {
                    self.hardware.bootrom.open("DMG_ROM.bin");
                }
                GameBoyModel::GBC => {
                    self.hardware.bootrom.open("CGB_ROM.bin");
                }
            }
        }
        
        // Advance PC to 0x100 if we are skipping the bootrom
        self.hardware.cpu.set_initial_state(skip_bootrom);
        self.hardware.ppu.set_initial_state(skip_bootrom);
    }

    pub fn stop(&mut self) {
        self.hardware.rom.close();
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

    pub fn inject_input(&self, b : JoystickButton, is_pressed: bool) {
        self.hardware.joystick.inject(&self.hardware, b, is_pressed);
    }

    pub fn get_audio_buffer(&mut self) -> Vec<i16> {
        self.hardware.apu.consume_audio_samples()
    }
    
    pub fn step(&mut self) {
        if let Some(debugger) = &self.debugger {
            if debugger.is_stopped() {
                return;
            }
        }

        self.tick();

        if let Some(debugger) = &self.debugger {
            debugger.process(&self.hardware.cpu, &self.hardware.ppu, &self.hardware);
        }
    }

    pub fn is_stopped(&self) -> bool {
        if let Some(debugger) = &self.debugger {
            return debugger.is_stopped();
        }

        false
    }

    fn tick(&mut self) {
        let cpu_cycles = self.hardware.cpu.tick(&self.hardware);
        let clocks = cpu_cycles * 4;

        for _ in 0..clocks {
            self.hardware.timer.tick(&self.hardware);
            self.hardware.ppu.tick(&self.hardware, &mut self.screen);
            self.hardware.apu.tick();
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
                debugger.stop(&self.hardware.cpu, &self.hardware.ppu);
            }
        }
    }

    pub fn debugger_step(&mut self) {
        self.tick();

        if let Some(debugger) = &self.debugger {
            debugger.print_trace(&self.hardware.cpu, &self.hardware.ppu);
        }
    }
}
