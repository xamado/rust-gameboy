use crate::machine::GameBoyModel;
use crate::cpu::{CPUInterrupts};
use crate::ppu::PPU;
use crate::apu::APU;
use crate::timer::Timer;
use crate::rom::ROM;
use crate::memory::Memory;
use crate::joystick::Joystick;
use crate::bootrom::BootROM;
use crate::serial::Serial;

pub struct CPUMemoryBus<'a> {
    pub model: GameBoyModel,
    pub bootrom_enabled: &'a mut bool,
    pub ppu: &'a mut PPU,
    pub apu: &'a mut APU,
    pub ram1: &'a mut Memory,
    pub ram2: &'a mut Memory,
    pub hram: &'a mut Memory,
    pub bootrom: &'a mut BootROM,
    pub rom: &'a mut ROM,
    pub joystick: &'a mut Joystick,
    pub timer: &'a mut Timer,
    pub serial: &'a mut Serial,
    pub interrupts: &'a mut CPUInterrupts,
}

impl<'a> CPUMemoryBus<'a> {
    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x00FF if *self.bootrom_enabled => self.bootrom.read_byte(addr),
            0x0200..=0x08FF if *self.bootrom_enabled => self.bootrom.read_byte(addr),

            // 0000-7FFF - ROM 
            0x0000..=0x7FFF => self.rom.read_byte(addr),

            // 8000-9FFF - VRAM
            0x8000..=0x9FFF => self.ppu.read_byte(addr),

            // A000-BFFF - ROM External RAM
            0xA000..=0xBFFF => self.rom.read_byte(addr),

            // C000-CFFF - WRAM Bank 0
            0xC000..=0xCFFF => self.ram1.read_byte(addr),

            // D000-DFFF - WRAM Banks 1-7
            0xD000..=0xDFFF => self.ram2.read_byte(addr),

            // E000-EFFF - "ECHO RAM" WRAM Bank 0 
            0xE000..=0xEFFF => self.ram1.read_byte(addr - 0xE000 + 0xC000),

            // F000-FDFF - "ECHO RAM" WRAM Banks 1-7 
            0xF000..=0xFDFF => self.ram2.read_byte(addr - 0xF000 + 0xD000),

            // FE00-FE9F - OAM Table
            0xFE00..=0xFE9F => self.ppu.read_oam_byte(addr - 0xFE00),

            // FEA0-FEFF Unusable
            0xFEA0..=0xFEFF => 0x00,

            // FF00 - Joystick Register
            0xFF00 => self.joystick.read_byte(addr),

            // FF01-FF02 - Serial 
            0xFF01..=0xFF02 => self.serial.read_byte(addr),

            // FF04-FF07 - Timer
            0xFF04..=0xFF07 => self.timer.read_byte(addr),

            // FF0F - Interrupt Enable
            0xFF0F => self.interrupts.read_byte(addr),

            // FF10-FF3F - APU 
            0xFF10..=0xFF3F => self.apu.read_byte(addr),

            // FF40-FF4B - PPU Registers
            0xFF40..=0xFF4B => self.ppu.read_byte(addr),

            // FF4F - VRAM Bank Register (GBC)
            0xFF4F if self.model == GameBoyModel::GBC => self.ppu.get_vram_bank(),

            // FF51-FF55 - HDMA Transfer (GBC)
            0xFF51..=0xFF55 if self.model == GameBoyModel::GBC => self.ppu.read_byte(addr),

            // FF68 - FF6A - Palette Data (GBC)
            0xFF68..=0xFF6B if self.model == GameBoyModel::GBC => self.ppu.read_byte(addr),

            // FF70 - WRAM Bank Switch Register
            0xFF70 if self.model == GameBoyModel::GBC => self.ram2.read_register(addr),

            // FF80-FFFE - HIGH RAM
            0xFF80..=0xFFFE => self.hram.read_byte(addr),

            // FFFF Interrupt Register
            0xFFFF => self.interrupts.read_byte(addr),

            // Unmapped behaviour
            _ => 0xFF
        }
    }

    pub fn write_byte(&mut self, addr: u16, data: u8) {
        match addr {
            // 0000-7FFF - ROM 
            0x0000..=0x7FFF => self.rom.write_byte(addr, data),

            // 8000-9FFF - VRAM
            0x8000..=0x9FFF => self.ppu.write_byte(addr, data),

            // A000-BFFF - ROM External RAM
            0xA000..=0xBFFF => self.rom.write_byte(addr, data),

            // C000-CFFF - WRAM Bank 0
            0xC000..=0xCFFF => self.ram1.write_byte(addr, data),

            // D000-DFFF - WRAM Banks 1-7
            0xD000..=0xDFFF => self.ram2.write_byte(addr, data),

            // E000-EFFF - "ECHO RAM" WRAM Bank 0 
            0xE000..=0xEFFF => self.ram1.write_byte(addr - 0xE000 + 0xC000, data),

            // F000-FDFF - "ECHO RAM" WRAM Banks 1-7 
            0xF000..=0xFDFF => self.ram2.write_byte(addr - 0xF000 + 0xD000, data),

            // FE00-FE9F - OAM Table
            0xFE00..=0xFE9F => self.ppu.write_oam_byte(addr - 0xFE00, data),

            // FEA0-FEFF Unusable
            0xFEA0..=0xFEFF => { },

            // FF00 - Joystick Register
            0xFF00 => self.joystick.write_byte(addr, data),

            // FF01-FF02 - Serial 
            0xFF01..=0xFF02 => self.serial.write_byte(addr, data),

            // FF04-FF07 - Timer
            0xFF04..=0xFF07 => self.timer.write_byte(addr, data),

            // FF0F - Interrupt Enable
            0xFF0F => self.interrupts.write_byte(addr, data),

            // FF10-FF3F - APU 
            0xFF10..=0xFF3F => self.apu.write_byte(addr, data),

            // FF40-FF4B - PPU Registers
            0xFF40..=0xFF4B => self.ppu.write_byte(addr, data),

            // FF4F - VRAM Bank Register
            0xFF4F if self.model == GameBoyModel::GBC => self.ppu.set_vram_bank(data & 0x1),

            // FF50 - DISABLE BOOTROM
            0xFF50 => *self.bootrom_enabled = false,

            // FF51-FF55 - HDMA Transfer (GBC)
            0xFF51..=0xFF55 if self.model == GameBoyModel::GBC => self.ppu.write_byte(addr, data),

            // FF68 - FF6A - Palette Data (GBC)
            0xFF68..=0xFF6B if self.model == GameBoyModel::GBC => self.ppu.write_byte(addr, data),

            // FF70 - WRAM Bank Switch Register (GBC)
            0xFF70 if self.model == GameBoyModel::GBC => self.ram2.write_register(addr, data),

            // FF80-FFFE - HIGH RAM
            0xFF80..=0xFFFE => self.hram.write_byte(addr, data),

            // FFFF Interrupt Register
            0xFFFF => self.interrupts.write_byte(addr, data),

            _ => println!("Invalid write address {:#06x}", addr)
        }
    }
}

pub struct PPUMemoryBus<'a> {
    pub rom: &'a mut ROM,
    pub ram1: &'a mut Memory,
    pub ram2: &'a mut Memory,
}

impl<'a> PPUMemoryBus<'a> {
    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            // 0000-7FFF - ROM 
            0x0000..=0x7FFF => self.rom.read_byte(addr),

            // 8000-9FFF - VRAM
            0x8000..=0x9FFF => 0xFF,

            // A000-BFFF - ROM External RAM
            0xA000..=0xBFFF => self.rom.read_byte(addr),

            // C000-CFFF - WRAM Bank 0
            0xC000..=0xCFFF => self.ram1.read_byte(addr),

            // D000-DFFF - WRAM Banks 1-7
            0xD000..=0xDFFF => self.ram2.read_byte(addr),

            // E000-EFFF - "ECHO RAM" WRAM Bank 0 
            0xE000..=0xEFFF => self.ram1.read_byte(addr - 0xE000 + 0xC000),

            // F000-FDFF - "ECHO RAM" WRAM Banks 1-7 
            0xF000..=0xFFFF => self.ram2.read_byte(addr - 0xF000 + 0xD000),
        }
    }

    // pub fn write_byte(&mut self, _addr: u16, _data: u8) {
        // match addr {
        //     _ => println!("Invalid write address {:#06x}", addr)
        // }
    // }
}
