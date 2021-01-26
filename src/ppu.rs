use std::cmp::Ordering;

use crate::bus::PPUMemoryBus;
use crate::screen::Screen;
use crate::cpu::{Interrupts, CPUInterrupts};
use crate::bitutils::*;
use crate::machine::GameBoyModel;

const MAX_SCANLINES: u8 = 154;
const VBLANK_LINE: u8 = 144;
const TILE_WIDTH: u8 = 8;
const TILE_HEIGHT: u8 = 8;
const TILES_PER_ROW: u8 = 32;
const TILES_PER_COL: u8 = 32;
const TILE_SIZE: u8 = 16;

#[allow(unused)]
enum LCDCBits {
    LCDEnable = 1 << 7,
    WindowTilemapDisplaySelect = 1 << 6,
    WindowEnable = 1 << 5,
    TileDataSelect = 1 << 4,
    BackgroundTilemapDisplaySelect = 1 << 3,
    OBJSize = 1 << 2,
    OBJDisplayEnable = 1 << 1,
    BGWindowDisplayPriority = 1
}

enum STATBits {
    LYCCheckEnable = 1 << 6,
    Mode2OAMCheckEnable = 1 << 5,
    Mode1VBlankCheckEnable = 1 << 4,
    Mode0HBlankCheckEnable = 1 << 3,
    LYCComparisonSignal = 1 << 2    
}

#[derive(Copy, Clone, PartialEq)]
enum PPUMode {
    HBlank = 0,
    VBlank = 1,
    ReadOAM = 2,
    ReadVRAM = 3,
}

#[derive(Copy, Clone)]
struct OAMAttributes {
    priority: bool,
    flip_y: bool,
    flip_x: bool,
    palette: u8,
    cgb_palette: u8,
    bank: u8
}

#[derive(Copy, Clone)]
struct OAMEntry {
    y: u8,
    x: u8,
    tile: u8,
    flags: OAMAttributes
}

struct TileAttributes {
    palette: u8,
    bank: u8,
    flip_x: bool,
    flip_y: bool,
    priority: bool
}

pub struct PPUDebugState {
    pub ly: u8,
    pub stat: u8,
    pub lcdc: u8,
    pub cycles: u16,
}

struct PPURegisters {
    lcdc: u8,
    stat: u8,
    scy: u8,
    scx: u8,
    ly: u8,
    lyc: u8,
    wpx: u8,
    wpy: u8,
    bg_palette: u8,
    obj_palette0: u8,
    obj_palette1: u8,
    cgb_bg_palette_index: u8,
    cgb_bg_palette_autoincrement: bool,
    cgb_bg_palette_data: [u8; 64],
    cgb_obj_palette_index: u8,
    cgb_obj_palette_autoincrement: bool,
    cgb_obj_palette_data: [u8; 64],
    dma_oam_active: bool,
    dma_oam_source: u8,
    hdma_active: bool,
    hdma_source: u16,
    hdma_destination: u16,
    hdma_mode: u8,
    hdma_length: u8,
    vram_bank: u16,
}

struct PPUState {
    mode: PPUMode,
    line_cycles: u16,
    trigger_stat_quirk: bool,
}

pub struct PPU {
    hardware_model: GameBoyModel,
    registers: PPURegisters,
    state: PPUState,
    vram: [u8; 0x4000],
    oam: [u8; 0x100],
}

// TODO: VRAM and OAM into Vecs for copy?
// TODO: Unify state.mode and the lcdc register...

impl PPU {
    pub fn new(model: GameBoyModel) -> Self {
        Self {
            hardware_model: model,
            vram: [0; 0x4000],
            oam: [0; 0x100],
            state: PPUState {
                mode: PPUMode::ReadOAM,
                line_cycles: 0,
                trigger_stat_quirk: false,
            },
            registers: PPURegisters {
                lcdc: 0x00,
                stat: 0x84,
                scy: 0,
                scx: 0,
                ly: 0,
                lyc: 0,
                wpx: 0,
                wpy: 0,
                bg_palette: 0xFC,
                obj_palette0: 0xFF,
                obj_palette1: 0xFF,
                cgb_bg_palette_index: 0,
                cgb_bg_palette_autoincrement: false,
                cgb_bg_palette_data: [0xFF; 64],
                cgb_obj_palette_index: 0,
                cgb_obj_palette_autoincrement: false,
                cgb_obj_palette_data: [0xFF; 64],
                dma_oam_active: false,
                dma_oam_source: 0,
                hdma_active: false,
                hdma_source: 0,
                hdma_destination: 0,
                hdma_mode: 0,
                hdma_length: 0,
                vram_bank: 0,
            },            
        }
    }

    pub fn set_initial_state(&mut self, skip_bootrom: bool) {
        if skip_bootrom {
            match self.hardware_model {
                GameBoyModel::DMG => {
                    self.registers.stat = 0x85;
                    self.registers.lcdc = 0x91;
                },

                GameBoyModel::GBC => {
                    self.registers.stat = 0x81;
                    self.registers.lcdc = 0x91;
                }
            }
        }
    }

    pub fn get_debug_state(&self) -> PPUDebugState {
        PPUDebugState {
            ly: self.registers.ly,
            lcdc: self.registers.lcdc,
            stat: 0x80 | self.registers.stat,
            cycles: self.state.line_cycles
        }
    }
    
    pub fn set_vram_bank(&mut self, bank: u8) {
        match self.hardware_model {
            GameBoyModel::GBC => self.registers.vram_bank = bank as u16,
            GameBoyModel::DMG => {}
        }        
    }

    pub fn get_vram_bank(&self) -> u8 {
        match self.hardware_model {
            GameBoyModel::DMG => 0xFF,
            GameBoyModel::GBC => 0xFE & ((self.registers.vram_bank as u8) & 0x1)
        }
    }

    pub fn read_oam_byte(&self, addr: u16) -> u8 {
        self.oam[addr as usize]
    }

    pub fn write_oam_byte(&mut self, addr: u16, data: u8) {
        self.oam[addr as usize] = data;
    }

    pub fn tick(&mut self, bus: &mut PPUMemoryBus, interrupts: &mut CPUInterrupts, screen: &mut Screen) {
        // Do HDMA or DMA transfers to VRAM/OAM memory
        self.handle_dma(bus);
        self.handle_hdma(bus);
        
        // HACK: In DMG writing anything to STAT while in HBLANK or VBLANK causes bit 1 of the IF register (0xFF0F) to be set
        // Roadrash and Legend of Zerd depend on this bug
        if self.state.trigger_stat_quirk {
            // self.raise_interrupt(Interrupts::LCDStat);
            // self.trigger_stat_quirk = false;
        }

        // If LCD is not ENABLED do nothing
        if !get_flag2(self.registers.lcdc, LCDCBits::LCDEnable as u8) {
            return;
        }

        self.state.line_cycles += 1;
        
        match self.state.mode {
            // OAM access mode Mode 2
            PPUMode::ReadOAM => { 
                // wait for 80 cycles, then go to mode VRAM READ MODE
                if self.state.line_cycles == 80 {
                    self.set_mode(interrupts, PPUMode::ReadVRAM);
                }
            },

            // VRAM ACCESS - Mode 3
            PPUMode::ReadVRAM => {
                if self.state.line_cycles == 252 { // 172 cycles + 80 from mode 2
                    // draw scanline
                    self.render_scanline(screen);

                    self.set_mode(interrupts, PPUMode::HBlank);
                }
                
                // HBlank STAT interrupt happens 1 cycle before mode switch
                else if self.state.line_cycles == 248 && get_flag2(self.registers.stat, STATBits::Mode0HBlankCheckEnable as u8) {
                    interrupts.raise_interrupt(Interrupts::LCDStat);
                }
            },

            // HBLANK - Mode 0
            PPUMode::HBlank => { 
                if self.state.line_cycles == 456 {
                    self.state.line_cycles -= 456;
                    
                    // update LY
                    let ly = self.registers.ly;
                    self.registers.ly = (ly + 1) % MAX_SCANLINES;

                    // check if LY = LYC if enabled (bit 6)
                    self.check_lyc_compare(interrupts);

                    if self.registers.ly == VBLANK_LINE { // ly = 144 
                        self.set_mode(interrupts, PPUMode::VBlank);
                    }
                    else {
                        self.set_mode(interrupts, PPUMode::ReadOAM);
                    }
                }
            },

            // VBLANK - Mode 1
            PPUMode::VBlank => { 
                if self.state.line_cycles == 456 {
                    self.state.line_cycles -= 456;

                    // update LY
                    self.registers.ly = (self.registers.ly + 1) % MAX_SCANLINES;
                    
                    // compare LY=LYC
                    self.check_lyc_compare(interrupts);

                    if self.registers.ly == 0 {
                        screen.set_vblank(true);
                        self.set_mode(interrupts, PPUMode::ReadOAM);
                    }
                }
            }
        }
    }

    fn handle_dma(&mut self, bus: &PPUMemoryBus) {
        // in theory dma copy takes a while... in fact:
        // This copy needs 160 × 4 + 4 clocks to
        // complete in both double speed and single speeds modes. The copy starts after the 4 setup clocks,
        // and a new byte is copied every 4 clocks.
        if self.registers.dma_oam_active {
            self.do_dma_transfer(bus, self.registers.dma_oam_source);
            self.registers.dma_oam_active = false;
        }
    }

    fn handle_hdma(&mut self, bus: &mut PPUMemoryBus) {
        if self.registers.hdma_active && self.registers.hdma_mode == 0 {
            while self.registers.hdma_length != 0xFF {
                self.hdma_copy_block(bus);
            }
        }
    }

    fn hdma_copy_block(&mut self, bus: &mut PPUMemoryBus) {
        for _ in 0..16 {
            let b = bus.read_byte(self.registers.hdma_source);
            self.write_vram(0x8000 + self.registers.hdma_destination, self.registers.vram_bank, b);

            self.registers.hdma_source = self.registers.hdma_source.wrapping_add(1);
            self.registers.hdma_destination = self.registers.hdma_destination.wrapping_add(1);
        }

        self.registers.hdma_length = self.registers.hdma_length.wrapping_sub(1);

        if self.registers.hdma_length == 0xFF {
            self.registers.hdma_active = false;            
        }
    }

    fn set_mode(&mut self, interrupts: &mut CPUInterrupts, mode: PPUMode) {
        self.state.mode = mode;
        self.registers.stat = self.registers.stat & 0x7C | (self.state.mode as u8);

        match self.state.mode {
            PPUMode::HBlank => {
                if self.registers.hdma_active && self.registers.hdma_mode == 1 {
                    // self.hdma_copy_block(bus, registers);
                }
            },

            PPUMode::VBlank => {
                // raise the VBlank interrupt
                interrupts.raise_interrupt(Interrupts::VBlank);

                if get_flag2(self.registers.stat, STATBits::Mode1VBlankCheckEnable as u8) {
                    interrupts.raise_interrupt(Interrupts::LCDStat);
                }

                // vbl stat also triggers with oam check
                if get_flag2(self.registers.stat, STATBits::Mode2OAMCheckEnable as u8) {
                    interrupts.raise_interrupt(Interrupts::LCDStat);
                }
            },

            PPUMode::ReadOAM => {
                if get_flag2(self.registers.stat, STATBits::Mode2OAMCheckEnable as u8) {
                    interrupts.raise_interrupt(Interrupts::LCDStat);
                }
            }

            PPUMode::ReadVRAM => {

            }
        }
    }

    fn check_lyc_compare(&mut self, interrupts: &mut CPUInterrupts) {
        // update bit 2 with the comparison result
        let ly_eq_lyc = self.registers.ly == self.registers.lyc;
        set_flag2(&mut self.registers.stat, STATBits::LYCComparisonSignal as u8, ly_eq_lyc);

        if get_flag2(self.registers.stat, STATBits::LYCCheckEnable as u8) && ly_eq_lyc {
            // raise the stat interrupt
            interrupts.raise_interrupt(Interrupts::LCDStat);
        }
    }

    fn disable_lcd(&mut self) {
        self.registers.ly = 0;
        self.state.line_cycles = 0;
        self.state.mode = PPUMode::HBlank;
        self.registers.stat = self.registers.stat & 0x7C | (self.state.mode as u8);
    }

    fn enable_lcd(&mut self) {
        set_flag2(&mut self.registers.stat, STATBits::LYCComparisonSignal as u8, true);
    }

    fn pick_visible_objects(&self) -> Vec<(u8, OAMEntry)> {
        let mode_8x16 = get_flag2(self.registers.lcdc, LCDCBits::OBJSize as u8);
        let height = if mode_8x16 { 16 } else { 8 };

        let mut objs: Vec<(u8, OAMEntry)> = vec!();

        for i in 0..40 {
            let obj = self.read_oam_entry(i);

            if obj.y == 0 || obj.y > 160 {
                continue;
            }

            let y = obj.y.wrapping_sub(16);
            if self.registers.ly.wrapping_sub(y) < height {
                objs.push((i, obj));
            }
        }

        let mut visible_sprites: Vec<(u8, OAMEntry)> = objs.iter().take(10).cloned().collect();

        visible_sprites.sort_by(|&(idxa, a), &(idxb, b)| {
            match a.x.cmp(&b.x) {
                Ordering::Equal => idxb.cmp(&idxa).reverse(),
                other => other.reverse()
            }
        });

        visible_sprites
    }

    fn render_scanline(&mut self, screen: &mut Screen) {
        let bg_enabled = get_flag2(self.registers.lcdc, LCDCBits::BGWindowDisplayPriority as u8);
        let w_enabled = get_flag2(self.registers.lcdc, LCDCBits::WindowEnable as u8);
        let obj_enabled = get_flag2(self.registers.lcdc, LCDCBits::OBJDisplayEnable as u8);

        let mut bg_buffer: [u16; 160] = [0; 160];
        let mut bg_attribs: [u8; 160] = [0; 160];

        if bg_enabled {
            self.draw_background(&mut bg_buffer, &mut bg_attribs);
        }

        if w_enabled && (self.hardware_model == GameBoyModel::GBC || bg_enabled) {
            self.draw_window(&mut bg_buffer, &mut bg_attribs);
        }

        if obj_enabled {
            let mode_8x16 = get_flag2(self.registers.lcdc, LCDCBits::OBJSize as u8);
            let height = if mode_8x16 { 16 } else { 8 };
            let tile_data_base_address: u16 = 0x8000;

            let objs = self.pick_visible_objects();

            for (_idx, obj) in objs {
                if obj.x == 0 || obj.x >= 168 {
                    continue;
                }
                
                let x = obj.x.wrapping_sub(8);
                let y = obj.y.wrapping_sub(16);

                let mut row = self.registers.ly.wrapping_sub(y);
                if obj.flags.flip_y {
                    row = height - row - 1;
                }

                let obj_tile_data = self.read_tile_data(tile_data_base_address, obj.flags.bank, obj.tile, row as u8);

                for p in 0..8 {
                    if x.wrapping_add(p) >= 160 {
                        continue;
                    }
                    
                    let idx = x.wrapping_add(p) as usize;
                    if obj.flags.priority && bg_attribs[idx] != 0 {
                        continue;
                    }

                    let color_idx = match obj.flags.flip_x {
                        true => obj_tile_data[TILE_WIDTH as usize - 1 - p as usize],
                        false => obj_tile_data[p as usize]
                    };

                    if color_idx != 0 {
                        match self.hardware_model {
                            GameBoyModel::DMG => {
                                let colors: u8 = if obj.flags.palette == 1 { self.registers.obj_palette1 } else { self.registers.obj_palette0 };
                                let color = (colors & (3 << (color_idx * 2))) >> (color_idx * 2);
                                bg_buffer[idx] = color as u16;
                            }

                            GameBoyModel::GBC => {
                                let palette_color_idx = (obj.flags.cgb_palette * 8) + (color_idx * 2);
                                let palette_color: u16 = (self.registers.cgb_obj_palette_data[palette_color_idx as usize] as u16) + ((self.registers.cgb_obj_palette_data[(palette_color_idx + 1) as usize] as u16) << 8);

                                bg_buffer[idx] = palette_color;
                            }
                        }
                    }
                }
            }
        }

        screen.set_scanline(self.registers.ly as u8, &bg_buffer);
    }
    
    fn draw_background(&self, color_buffer: &mut [u16; 160], bg_attribs: &mut [u8; 160]) {
        let start_tile_row: u8 = ((self.registers.scy as u16 + self.registers.ly as u16) / (TILE_HEIGHT as u16)) as u8;
        let start_tile_col: u8 = self.registers.scx / TILE_WIDTH;
        let end_tile_col: u8 = start_tile_col + 21;
        let pixel_row = (self.registers.scy as u16 + self.registers.ly as u16) % TILE_HEIGHT as u16;
        
        let display_select = get_flag2(self.registers.lcdc, LCDCBits::BackgroundTilemapDisplaySelect as u8);
        let bg_tile_map_address: u16 = if display_select { 0x9C00 } else { 0x9800 };

        let addressing_mode = get_flag2(self.registers.lcdc, LCDCBits::TileDataSelect as u8);
        let tile_data_base_address: u16 = if addressing_mode { 0x8000 } else { 0x8800 };

        let mut pixel_idx = 0;
        let scx: u16 = self.registers.scx as u16;

        for x in start_tile_col..end_tile_col {
            // read tile number from tile map
            let tile_address = bg_tile_map_address + (((TILES_PER_ROW as u16 * (start_tile_row % TILES_PER_ROW) as u16) + (x % TILES_PER_COL) as u16) as u16);
            let tile_number: u8 = self.read_vram(tile_address, 0);

            // read tile attributes
            let tile_attribs = self.read_tile_attribs(tile_address);

            // read tile data
            let tile_index: u8 = if addressing_mode { tile_number } else { ((tile_number as i16) + 128) as u8 };
            let tile_row_data = match self.hardware_model {
                GameBoyModel::DMG => {
                    self.read_tile_data(tile_data_base_address, 0, tile_index, pixel_row as u8)
                }
                GameBoyModel::GBC => {                   
                    if tile_attribs.flip_y {
                        self.read_tile_data(tile_data_base_address, tile_attribs.bank, tile_index, 7 - pixel_row as u8)
                    }
                    else {
                        self.read_tile_data(tile_data_base_address, tile_attribs.bank, tile_index, pixel_row as u8)
                    }
                }
            };

            let mut pixel_col = x as u16 * TILE_WIDTH as u16;
            
            for i in 0..TILE_WIDTH {
                if pixel_col >= scx && pixel_col <= scx + 160 && pixel_idx < 160 {
                    let color_idx = match tile_attribs.flip_x {
                        false => tile_row_data[i as usize] & 0x03,
                        true => tile_row_data[7 - i as usize] & 0x03
                    };

                    match self.hardware_model {
                        GameBoyModel::DMG => {
                            let bg_color = (self.registers.bg_palette & (3 << (color_idx * 2))) >> (color_idx * 2);
                            color_buffer[pixel_idx] = bg_color as u16;
                            bg_attribs[pixel_idx] = color_idx;
                        }

                        GameBoyModel::GBC => {
                            let palette_color_idx = (tile_attribs.palette * 8) + (color_idx * 2);
                            let palette_color: u16 = (self.registers.cgb_bg_palette_data[palette_color_idx as usize] as u16) + ((self.registers.cgb_bg_palette_data[(palette_color_idx + 1) as usize] as u16) << 8);

                            color_buffer[pixel_idx] = palette_color;
                            bg_attribs[pixel_idx] = color_idx;
                        }
                    }

                    pixel_idx += 1;
                }
                
                pixel_col += 1;
            }

            if pixel_idx >= 160 {
                break;
            }
        }
    }

    fn draw_window(&self, color_buffer: &mut [u16; 160], bg_attribs: &mut [u8; 160]) {
        let window_select = self.registers.lcdc & (LCDCBits::WindowTilemapDisplaySelect as u8);
        let window_tile_map_address: u16 = if (window_select) != 0 { 0x9C00 } else { 0x9800 };

        let addressing_mode = self.registers.lcdc & LCDCBits::TileDataSelect as u8;
        let tile_data_base_address: u16 = if addressing_mode != 0 { 0x8000 } else { 0x8800 };

        if self.registers.ly >= self.registers.wpy {
            let relative_line = self.registers.ly - self.registers.wpy;
            let start_tile_row = relative_line / TILE_HEIGHT;
            let pixel_row = relative_line % TILE_HEIGHT;
        
            let mut pixel_col: u8 = self.registers.wpx.wrapping_sub(7);

            for x in 0..=20 {
                // read tile number from tile map
                let tile_address = window_tile_map_address + (((TILES_PER_ROW as u16 * (start_tile_row % TILES_PER_ROW) as u16) + (x % TILES_PER_COL) as u16) as u16);
                let tile_number: u8 = self.read_vram(tile_address, 0);

                // read tile attributes
                let tile_attribs = self.read_tile_attribs(tile_address);
                
                // read tile data
                let tile_index: u8 = if addressing_mode != 0 { tile_number } else { ((tile_number as i16) + 128) as u8 };
                let tile_row_data = self.read_tile_data(tile_data_base_address, tile_attribs.bank, tile_index, pixel_row as u8);

                for i in 0..TILE_WIDTH {
                    if pixel_col < 160 {
                        let color_idx = tile_row_data[i as usize] & 0x03;

                        match self.hardware_model {
                            GameBoyModel::DMG => {
                                let bg_color = (self.registers.bg_palette & (3 << (color_idx * 2))) >> (color_idx * 2);
                                color_buffer[pixel_col as usize] = bg_color as u16;
                                bg_attribs[pixel_col as usize] = color_idx;
                            }
    
                            GameBoyModel::GBC => {
                                let palette_color_idx = (tile_attribs.palette * 8) + (color_idx * 2);
    
                                let palette_color: u16 = (self.registers.cgb_bg_palette_data[palette_color_idx as usize] as u16) + ((self.registers.cgb_bg_palette_data[(palette_color_idx + 1) as usize] as u16) << 8);
    
                                color_buffer[pixel_col as usize] = palette_color;
                                bg_attribs[pixel_col as usize] = color_idx;
                            }
                        }
                    }

                    pixel_col = pixel_col.wrapping_add(1);
                }

                if pixel_col >= 160 {
                    break;
                }
            }
        }
    }

    fn read_tile_attribs(&self, tile_address: u16) -> TileAttributes {
        let tile_attribs: u8 = match self.hardware_model {
            GameBoyModel::DMG => 0,
            GameBoyModel::GBC => self.read_vram(tile_address, 1)
        };

        TileAttributes {
            palette: tile_attribs & 0x7,
            bank: get_bit(tile_attribs, 3),
            flip_x: get_bit(tile_attribs, 5) != 0,
            flip_y: get_bit(tile_attribs, 6) != 0,
            priority: get_bit(tile_attribs, 7) != 0
        }
    }

    fn read_tile_data(&self, base_address: u16, bank: u8, tile_number: u8, row: u8) -> [u8; 8] {
        let tile_address = base_address + (tile_number as u16 * TILE_SIZE as u16);

        let offset: u16 = row as u16 * 2;
        let mut tile_row: [u8; 8] = [0; 8];
        
        let lsb = self.read_vram(tile_address + offset, bank as u16);
        let msb = self.read_vram(tile_address + offset + 1, bank as u16);

        for bit in (0..TILE_WIDTH).rev() {
            let mask: u8 = 1 << bit;
            let color: u8 = (((msb & mask) >> bit) << 1) | ((lsb & mask) >> bit);

            tile_row[(7 - bit) as usize] = color;
        }

        tile_row
    }

    fn read_oam_entry(&self, idx: u8) -> OAMEntry {
        let y = self.oam[(idx as u16 * 4) as usize];
        let x = self.oam[(idx  as u16 * 4 + 1) as usize];
        let tile = self.oam[(idx as u16 * 4 + 2) as usize];
        let flags = self.oam[(idx as u16 * 4 + 3) as usize];

        OAMEntry {
            y,
            x,
            tile, 
            flags: OAMAttributes {
                priority: flags & (1 << 7) != 0,
                flip_y: flags & (1 << 6) != 0,
                flip_x: flags & (1 << 5) != 0,
                palette: get_bit(flags, 4),
                bank: get_bit(flags, 3),
                cgb_palette: flags & 0x07,
            }
        }
    }

    fn do_dma_transfer(&mut self, bus: &PPUMemoryBus, data: u8) {
        let addr: u16 = (data as u16) << 8;
        let mut data: [u8; 0xA0] = [0; 0xA0];
        
        for (i, datum) in data.iter_mut().enumerate() {
            *datum = bus.read_byte(addr + (i as u16));
        }

        for (i, datum) in data.iter().enumerate() {
            self.oam[i as usize] = *datum;
        }
    }

    fn read_vram(&self, addr: u16, bank: u16) -> u8 {
        self.vram[(addr - 0x8000 + bank * 0x2000) as usize]
    }

    fn write_vram(&mut self, addr: u16, bank: u16, data: u8) {
        self.vram[(addr - 0x8000 + bank * 0x2000) as usize] = data;
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x8000..=0x9FFF => {
                if self.state.mode != PPUMode::ReadVRAM {
                    self.read_vram(address, self.registers.vram_bank)
                }
                else {
                    println!("VRAM Invalid Read when in Mode3: VRAM{}:{:#06x}", self.registers.vram_bank, address);
                    self.read_vram(address, self.registers.vram_bank) // THIS SHOULDNT BE DONE, WE SHOULD RETURN 0xFF
                    // 0xFF
                }
            },

            0xFE00..=0xFE9F => {
                if self.state.mode != PPUMode::ReadVRAM && self.state.mode != PPUMode::ReadOAM {
                    self.oam[(address - 0xFE00) as usize]
                }
                else {
                    println!("OAM Invalid Read when in Mode3: OAM:{:#06x}", address);
                    self.oam[(address - 0xFE00) as usize]
                    //     0xFF
                }
            },

            // FF40 LCDC - LCD Control (R/W)
            0xFF40 => self.registers.lcdc,

            // FF41 STAT - LCDC Status (R/W)
            0xFF41 => {
                if !get_flag2(self.registers.lcdc, LCDCBits::LCDEnable as u8) {
                    // disable bits 0-2 if LCD is off
                    (0x80 | self.registers.stat) & !0x7
                }
                else {
                    0x80 | self.registers.stat
                }
            },

            // FF42 SCY - Scroll Y
            0xFF42 => self.registers.scy, 

            // FF43 SCX - Scroll X
            0xFF43 => self.registers.scx,

            // FF44 - LY - LCDC Y-Coordinate (R)
            0xFF44 => {
                self.registers.ly
            }

            // FF45 LYC - LY Compare (R/W)
            0xFF45 => self.registers.lyc,

            // FF46 - OAM DMA - OAM DMA Transfer and Start Address (W)
            0xFF46 => 0,

            // FF47 - BGP - BG Palette Data (R/W)
            0xFF47 => self.registers.bg_palette,

            // FF48 - OBP0 - Object Palette 0 Data (R/W)
            0xFF48 => self.registers.obj_palette0,

            // FF49 - OBP1 - Object Palette 1 Data (R/W) 
            0xFF49 => self.registers.obj_palette1,

            // FF4A WY - Window Y Position (R/W)
            0xFF4A => self.registers.wpy,

            // FF4B WX - Window X Position minus 7 (R/W)
            0xFF4B => self.registers.wpx,
            
            // FF51 HDMA1 - DMA Source, High
            // FF52 HDMA2 - DMA Source, Low
            // FF53 HDMA3 - DMA Destination, High
            // FF54 HDMA4 - DMA Destination, Low
            0xFF51 | 0xFF52 | 0xFF53 | 0xFF54 => 0xFF,

            // FF55 HDMA5 - DMA Length/Mode/Start
            // 0xFF55 => (((!self.hdma_active) as u8) << 7) | self.hdma_length & 0x7F,
            0xFF55 => self.registers.hdma_length,
            
            // FF68 BCPS/BGPI - Background Palette Index (CGB)
            0xFF68 => {
                ((self.registers.cgb_bg_palette_autoincrement as u8) << 7) | self.registers.cgb_bg_palette_index
            },

            // FF69 BCPD/BGPD - Background Palette Data (CGB)
            0xFF69 => {
                if self.state.mode != PPUMode::ReadVRAM {
                    self.registers.cgb_bg_palette_data[self.registers.cgb_bg_palette_index as usize]
                } 
                else {
                    0
                }
            },
            
            _ => panic!("Invalid read")
        }
    }

    pub fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            0x8000..=0x9FFF => {
                let lcd_enabled = get_flag2(self.registers.lcdc, LCDCBits::LCDEnable as u8);

                if !lcd_enabled || self.state.mode != PPUMode::ReadVRAM {
                    self.write_vram(address, self.registers.vram_bank, data);
                }
                else {
                    println!("VRAM Invalid Write when in Mode3: VRAM{}:{:#06x}", self.registers.vram_bank, address);
                    self.write_vram(address, self.registers.vram_bank, data); // THIS SHOULDNT BE DONE
                }
            },

            0xFE00..=0xFE9F => {
                let lcd_enabled = get_flag2(self.registers.lcdc, LCDCBits::LCDEnable as u8);

                if !lcd_enabled || (self.state.mode != PPUMode::ReadVRAM && self.state.mode != PPUMode::ReadOAM) {
                    self.oam[(address - 0xFE00) as usize] = data;
                }
                else {
                    println!("OAM Invalid Write when in Mode{}: OAM:{:#06x}", self.state.mode as u8, address);
                    self.oam[(address - 0xFE00) as usize] = data; // THIS SHOULDNT BE DONE
                }
            }

            // FF40 LCDC - LCD Control (R/W)
            0xFF40 => {
                let was_on = get_flag2(self.registers.lcdc, LCDCBits::LCDEnable as u8);
                self.registers.lcdc = data;
                let is_on = get_flag2(self.registers.lcdc, LCDCBits::LCDEnable as u8);

                if was_on && !is_on {
                    self.disable_lcd();
                }
                else if !was_on && is_on {
                    self.enable_lcd();
                }
            },

            // FF41 STAT - LCDC Status (R/W)
            0xFF41 => {
                self.registers.stat = 0x80 | (data & !0x7 | self.registers.stat & 0x7);

                if self.state.mode == PPUMode::HBlank || self.state.mode == PPUMode::VBlank {
                    self.state.trigger_stat_quirk = true;
                }
            },

            // FF42 SCY - Scroll Y (R/W)
            0xFF42 => self.registers.scy = data,  

            // FF43 SCX - Scroll X (R/W)
            0xFF43 => self.registers.scx = data,

            // FF44 - LY - LCDC Y-Coordinate (R)
            0xFF44 => {},

            // FF45 LYC - LY Compare (R/W)
            0xFF45 => {
                self.registers.lyc = data;
            },

            // FF46 - OAM DMA - OAM DMA Transfer and Start Address (W)
            0xFF46 => {
                self.registers.dma_oam_active = true;
                self.registers.dma_oam_source = data;
            },

            // FF47 - BGP - BG Palette Data (R/W)
            0xFF47 => self.registers.bg_palette = data,

            // FF48 - OBP0 - Object Palette 0 Data (R/W)
            0xFF48 => self.registers.obj_palette0 = data,

            // FF49 - OBP1 - Object Palette 1 Data (R/W) 
            0xFF49 => self.registers.obj_palette1 = data,

            // FF4A WY - Window Y Position (R/W)
            0xFF4A => self.registers.wpy = data,

            // FF4B WX - Window X Position minus 7 (R/W)
            0xFF4B => self.registers.wpx = data,

            // FF51 HDMA1 - DMA Source, High
            0xFF51 => {
                self.registers.hdma_source = (((data as u16) << 8) | (self.registers.hdma_source & 0xFF)) & !0xF;
            }

            // FF52 HDMA2 - DMA Source, Low
            0xFF52 => { 
                self.registers.hdma_source = ((self.registers.hdma_source & 0xFF00) | (data as u16)) & !0xF;
            }

            // FF53 HDMA3 - DMA Destination, High
            0xFF53 => {
                self.registers.hdma_destination = (((data as u16) << 8) | (self.registers.hdma_destination & 0xFF)) & 0x1FF0;
            }

            // FF54 HDMA4 - DMA Destination, Low
            0xFF54 => {
                self.registers.hdma_destination = ((self.registers.hdma_destination & 0xFF00) | (data as u16)) & 0x1FF0;
            }

            // FF55 HDMA5 - DMA Length/Mode/Start
            0xFF55 => {
                if self.registers.hdma_active {
                    if get_bit(data, 7) == 0 {
                        self.registers.hdma_active = false;
                        self.registers.hdma_mode = 1;
                    }
                    self.registers.hdma_length = data & 0x7F;
                }
                else {
                    self.registers.hdma_active = true;

                    self.registers.hdma_mode = get_bit(data, 7);
                    self.registers.hdma_length = data & 0x7F;
                }
            }

            // FF68 BCPS/BGPI - Background Palette Index (CGB)
            0xFF68 => {
                self.registers.cgb_bg_palette_index = data & 0x1F;
                self.registers.cgb_bg_palette_autoincrement = data & 0x80 != 0;
            },

            // FF69 BCPD/BGPD - Background Palette Data (CGB)
            0xFF69 => {
                // if self.mode != PPUMode::ReadVRAM {
                    let pidx = self.registers.cgb_bg_palette_index as usize;
                    self.registers.cgb_bg_palette_data[pidx] = data;

                    if self.registers.cgb_bg_palette_autoincrement {
                        self.registers.cgb_bg_palette_index += 1;
                        self.registers.cgb_bg_palette_index %= 64;
                    }
                // } 
                // else {
                //     println!("Palette write in VRAM mode");
                // }
            },

            // FF6A OCPS/OBPI - Object Palette Index (CGB)
            0xFF6A => {
                self.registers.cgb_obj_palette_index = data & 0x1F;
                self.registers.cgb_obj_palette_autoincrement = data & 0x80 != 0;
            },

            // FF6B OCPD/OBPD - Object Palette Data (CGB)
            0xFF6B => {
                // if self.mode != PPUMode::ReadVRAM {
                    let pidx = self.registers.cgb_obj_palette_index as usize;
                    self.registers.cgb_obj_palette_data[pidx] = data;

                    if self.registers.cgb_obj_palette_autoincrement {
                        self.registers.cgb_obj_palette_index += 1;
                    }
                // } 
                // else {
                //     println!("Palette write in VRAM mode");
                // }
            },

            _ => panic!("Invalid read")
        }
    }
}