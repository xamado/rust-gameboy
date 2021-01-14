use std::rc::Rc;
use std::cmp::Ordering;
use core::cell::RefCell;

use crate::memorybus::MemoryBus;
use crate::screen::Screen;
use crate::cpu::Interrupts;
use crate::iomapped::IOMapped;
use crate::bitutils::*;

const MAX_SCANLINES: u8 = 154;
const VBLANK_LINE: u8 = 144;
const TILE_WIDTH: u8 = 8;
const TILE_HEIGHT: u8 = 8;
const TILES_PER_ROW: u8 = 32;
const TILES_PER_COL: u8 = 32;
const TILE_SIZE: u8 = 16;

const OAM_TABLE_ADDRESS: u16 = 0xFE00;

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

#[derive(Copy, Clone)]
enum PPUMode {
    HBlank = 0,
    VBlank = 1,
    ReadOAM = 2,
    ReadVRAM = 3,
}

#[derive(Copy, Clone)]
struct OAMEntry {
    y: u8,
    x: u8,
    tile: u8,
    flags: u8
}

enum OBJAttributes {
    Priority = 1 << 7,
    YFlip = 1 << 6,
    XFlip = 1 << 5,
    Palette = 1 << 4
}

pub struct PPU {
    bus: Rc<RefCell<MemoryBus>>,
    screen: Rc<RefCell<Screen>>,
    vram: [u8; 0x2000],
    oam: [u8; 0x100],
    mode: PPUMode,
    line_cycles: u32,
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
    dma_active: bool,
    dma_source: u8,
    total_vblank_cycles: u32
}

impl PPU {
    pub fn new(bus: Rc<RefCell<MemoryBus>>, screen: Rc<RefCell<Screen>>) -> Self {
        Self {
            bus,
            screen,
            vram: [0; 0x2000],
            oam: [0; 0x100],
            mode: PPUMode::ReadOAM,
            line_cycles: 0,
            lcdc: 0x91,
            stat: 0x85,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            wpx: 0,
            wpy: 0,
            bg_palette: 0xFC,
            obj_palette0: 0xFF,
            obj_palette1: 0xFF,
            dma_active: false,
            dma_source: 0,
            total_vblank_cycles: 0,
        }
    }

    pub fn tick(&mut self) {
        // in theory dma copy takes a while... in fact:
        // This copy needs 160 × 4 + 4 clocks to
        // complete in both double speed and single speeds modes. The copy starts after the 4 setup clocks,
        // and a new byte is copied every 4 clocks.
        if self.dma_active {
            self.do_dma_transfer(self.dma_source);
            self.dma_active = false;
        }

        // If LCD is not ENABLED do nothing
        if !get_flag2(&self.lcdc, LCDCBits::LCDEnable as u8) {
            // self.ly = 0;
            return;
        }

        self.line_cycles += 1;
        self.total_vblank_cycles += 1;
        
        match self.mode {
            // OAM access mode
            PPUMode::ReadOAM => { 
                // wait for 82 cycles, then go to mode VRAM READ MODE
                if self.line_cycles >= 82 {
                    self.set_mode(PPUMode::ReadVRAM);
                }
            },

            // VRAM read mode
            PPUMode::ReadVRAM => {
                if self.line_cycles >= 252 {
                    // draw scanline
                    self.render_scanline();

                    self.set_mode(PPUMode::HBlank);
                }
            },

            // HBLANK
            PPUMode::HBlank => { 
                if self.line_cycles >= 456 {
                    self.line_cycles -= 456;
                    
                    // update LY
                    self.ly = (self.ly + 1) % MAX_SCANLINES;

                    // check if LY = LYC if enabled (bit 6)
                    self.check_lyc_compare();

                    if self.ly == VBLANK_LINE { // ly = 144 
                        self.set_mode(PPUMode::VBlank);
                    }
                    else {
                        self.set_mode(PPUMode::ReadOAM);
                    }
                }
            },

            // VBLANK
            PPUMode::VBlank => { 
                if self.line_cycles >= 456 {
                    self.line_cycles -= 456;

                    // update LY
                    self.ly = (self.ly + 1) % MAX_SCANLINES;
                    
                    // compare LY=LYC
                    self.check_lyc_compare();

                    if self.ly == 0 {
                        self.screen.borrow_mut().set_vblank(true);

                        self.total_vblank_cycles = 0;
                        self.set_mode(PPUMode::ReadOAM);
                    }
                }
            }
        }
    }

    fn set_mode(&mut self, mode: PPUMode) {
        self.mode = mode;
        self.stat = self.stat & 0x7C | (self.mode as u8);

        let mut iif = self.bus.borrow().read_byte(0xFF0F);

        match self.mode {
            PPUMode::HBlank => {
                if self.stat & (STATBits::Mode0HBlankCheckEnable as u8) != 0 {
                    iif |= 1 << Interrupts::LCDStat as u8;
                }
            },

            PPUMode::VBlank => {
                // raise the VBlank interrupt
                iif |= 1 << Interrupts::VBlank as u8;

                if self.stat & (STATBits::Mode1VBlankCheckEnable as u8) != 0 {
                    iif |= 1 << Interrupts::LCDStat as u8;
                }

                // vbl stat also triggers with oam check
                if self.stat & (STATBits::Mode2OAMCheckEnable as u8) != 0 {
                    iif |= 1 << Interrupts::LCDStat as u8;
                }
            },

            PPUMode::ReadOAM => {
                if self.stat & (STATBits::Mode2OAMCheckEnable as u8) != 0 {
                    iif |= 1 << Interrupts::LCDStat as u8;
                }
            }

            PPUMode::ReadVRAM => {

            }
        }
        
        self.bus.borrow_mut().write_byte(0xFF0F, iif);
    }

    fn check_lyc_compare(&mut self) {
        if get_flag2(&self.stat, STATBits::LYCCheckEnable as u8) {
            // update bit 2 with the comparison result
            let ly_eq_lyc = self.ly == self.lyc;
            set_flag2(&mut self.stat, STATBits::LYCComparisonSignal as u8, ly_eq_lyc);

            if ly_eq_lyc {
                // raise the stat interrupt
                let iif = self.bus.borrow().read_byte(0xFF0F) | (1 << Interrupts::LCDStat as u8);
                self.bus.borrow_mut().write_byte(0xFF0F, iif);
            }
        }
    }

    fn pick_visible_objects(&self) -> Vec<(u8, OAMEntry)> {
        let mode_8x16 = (self.lcdc & (LCDCBits::OBJSize as u8)) != 0;
        let height = if mode_8x16 { 16 } else { 8 };

        let mut objs: Vec<(u8, OAMEntry)> = vec!();

        for i in 0..40 {
            let obj = self.read_oam_entry(i);

            if obj.y == 0 || obj.y > 160 {
                continue;
            }

            let y = obj.y.wrapping_sub(16);
            if self.ly.wrapping_sub(y) < height {
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

    fn render_scanline(&mut self) {
        let bg_enabled = (self.lcdc & (LCDCBits::BGWindowDisplayPriority as u8)) != 0;
        let w_enabled = (self.lcdc & (LCDCBits::WindowEnable as u8)) != 0;
        let obj_enabled = (self.lcdc & (LCDCBits::OBJDisplayEnable as u8)) != 0;

        let mut scanline_buffer: [u8; 160] = [0; 160];

        if bg_enabled {
            self.draw_background(&mut scanline_buffer);
        }

        if w_enabled {
            self.draw_window(&mut scanline_buffer);
        }

        if obj_enabled {
            let mode_8x16 = (self.lcdc & (LCDCBits::OBJSize as u8)) != 0;
            let height = if mode_8x16 { 16 } else { 8 };
            let tile_data_base_address: u16 = 0x8000;

            let objs = self.pick_visible_objects();

            for (_idx, obj) in objs {
                if obj.x == 0 || obj.x >= 168 {
                    continue;
                }
                
                let flip_x = (obj.flags & OBJAttributes::XFlip as u8) != 0;
                let flip_y = (obj.flags & OBJAttributes::YFlip as u8) != 0;
                let priority = (obj.flags & OBJAttributes::Priority as u8) != 0;
                let palette = (obj.flags & OBJAttributes::Palette as u8) != 0;
                let x = obj.x.wrapping_sub(8);
                let y = obj.y.wrapping_sub(16);

                let mut row = self.ly.wrapping_sub(y);
                if flip_y {
                    row = height - row - 1;
                }

                let obj_tile_data = self.read_tile_data(tile_data_base_address, obj.tile, row as u8);

                for p in 0..8 {
                    if x.wrapping_add(p) >= 160 {
                        continue;
                    }

                    let colors: u8 = if palette { self.obj_palette1 } else { self.obj_palette0 };
                    let idx = x.wrapping_add(p) as usize;
                    if priority && scanline_buffer[idx] != 0 {
                        continue;
                    }

                    let color_idx = if flip_x { obj_tile_data[TILE_WIDTH as usize - 1 - p as usize] } else { obj_tile_data[p as usize] };
                    if color_idx == 0 {
                        continue;
                    }

                    let color = (colors & (3 << (color_idx * 2))) >> (color_idx * 2);
                    scanline_buffer[idx] = color;
                }
            }
        }

        self.screen.borrow_mut().set_scanline(self.ly as u8, &scanline_buffer);
    }
    
    fn draw_background(&self, scanline_buffer: &mut [u8; 160]) {
        let start_tile_row: u8 = ((self.scy as u16 + self.ly as u16) / (TILE_HEIGHT as u16)) as u8;
        let start_tile_col: u8 = self.scx / TILE_WIDTH;
        let end_tile_col: u8 = start_tile_col + 21;
        let pixel_row = (self.scy as u16 + self.ly as u16) % TILE_HEIGHT as u16;
        
        let display_select = self.lcdc & (LCDCBits::BackgroundTilemapDisplaySelect as u8);
        let bg_tile_map_address: u16 = if (display_select) != 0 { 0x9C00 } else { 0x9800 };

        let addressing_mode = self.lcdc & LCDCBits::TileDataSelect as u8;
        let tile_data_base_address: u16 = if addressing_mode != 0 { 0x8000 } else { 0x8800 };

        let mut pixel_idx = 0;
        let scx: u16 = self.scx as u16;

        for x in start_tile_col..end_tile_col {
            // read tile number from tile map
            let tile_address = bg_tile_map_address + (((TILES_PER_ROW as u16 * (start_tile_row % TILES_PER_ROW) as u16) + (x % TILES_PER_COL) as u16) as u16);
            let tile_number: u8 = self.read_byte(tile_address);

            // read tile data
            let tile_index: u8 = if addressing_mode != 0 { tile_number } else { ((tile_number as i16) + 128) as u8 };
            let tile_row_data = self.read_tile_data(tile_data_base_address, tile_index, pixel_row as u8);

            let mut pixel_col = x as u16 * TILE_WIDTH as u16;
            
            for i in 0..TILE_WIDTH {
                if pixel_col >= scx && pixel_col <= scx + 160 && pixel_idx < 160 {
                    let color_idx = tile_row_data[i as usize] & 0x03;
                    let color = (self.bg_palette & (3 << (color_idx * 2))) >> (color_idx * 2);
                    scanline_buffer[pixel_idx] = color;
                    pixel_idx += 1;
                }
                
                pixel_col += 1;
            }

            if pixel_idx >= 160 {
                break;
            }
        }
    }

    fn draw_window(&self, scanline_buffer: &mut [u8; 160]) {
        let window_select = self.lcdc & (LCDCBits::WindowTilemapDisplaySelect as u8);
        let window_tile_map_address: u16 = if (window_select) != 0 { 0x9C00 } else { 0x9800 };

        let addressing_mode = self.lcdc & LCDCBits::TileDataSelect as u8;
        let tile_data_base_address: u16 = if addressing_mode != 0 { 0x8000 } else { 0x8800 };

        if self.ly >= self.wpy {
            let relative_line = self.ly - self.wpy;
            let start_tile_row = relative_line / TILE_HEIGHT;
            let pixel_row = relative_line % TILE_HEIGHT;
        
            let mut pixel_col: u8 = self.wpx.wrapping_sub(7);

            for x in 0..=20 {
                // read tile number from tile map
                let tile_address = window_tile_map_address + (((TILES_PER_ROW as u16 * (start_tile_row % TILES_PER_ROW) as u16) + (x % TILES_PER_COL) as u16) as u16);
                let tile_number: u8 = self.read_byte(tile_address);

                // read tile data
                let tile_index: u8 = if addressing_mode != 0 { tile_number } else { ((tile_number as i16) + 128) as u8 };
                let tile_row_data = self.read_tile_data(tile_data_base_address, tile_index, pixel_row as u8);

                for i in 0..TILE_WIDTH {
                    if pixel_col < 160 {
                        let color_idx = tile_row_data[i as usize] & 0x03;
                        let color = (self.bg_palette & (3 << (color_idx * 2))) >> (color_idx * 2);
                        scanline_buffer[pixel_col as usize] = color;
                    }

                    pixel_col = pixel_col.wrapping_add(1);
                }

                if pixel_col >= 160 {
                    break;
                }
            }
        }
    }

    fn read_tile_data(&self, base_address: u16, tile_number: u8, row: u8) -> [u8; 8] {
        let tile_address = base_address + (tile_number as u16 * TILE_SIZE as u16);

        let offset: u16 = row as u16 * 2;
        let mut tile_row: [u8; 8] = [0; 8];
        
        let lsb = self.read_byte(tile_address + offset);
        let msb = self.read_byte(tile_address + offset + 1);

        for bit in (0..TILE_WIDTH).rev() {
            let mask: u8 = 1 << bit;
            let color: u8 = (((msb & mask) >> bit) << 1) | ((lsb & mask) >> bit);

            tile_row[(7 - bit) as usize] = color;
        }

        tile_row
    }

    fn read_oam_entry(&self, idx: u8) -> OAMEntry {
        OAMEntry {
            y: self.read_byte(OAM_TABLE_ADDRESS + idx as u16 * 4),
            x: self.read_byte(OAM_TABLE_ADDRESS + idx  as u16 * 4 + 1),
            tile: self.read_byte(OAM_TABLE_ADDRESS + idx as u16 * 4 + 2),
            flags: self.read_byte(OAM_TABLE_ADDRESS + idx as u16 * 4 + 3)
        }
    }

    fn do_dma_transfer(&mut self, data: u8) {
        let addr: u16 = (data as u16) << 8;
        let mut data: [u8; 0xA0] = [0; 0xA0];
        
        let bus = self.bus.borrow();
        for (i, datum) in data.iter_mut().enumerate() {
            *datum = bus.read_byte(addr + (i as u16));
        }

        for (i, datum) in data.iter().enumerate() {
            self.oam[i as usize] = *datum;
        }
    }
}

impl IOMapped for PPU {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x8000..=0x9FFF => {
                self.vram[(address - 0x8000) as usize]
            },

            0xFE00..=0xFE9F => {
                self.oam[(address - 0xFE00) as usize]
            },

            // FF40 LCDC - LCD Control (R/W)
            0xFF40 => self.lcdc,

            // FF41 STAT - LCDC Status (R/W)
            0xFF41 => {
                if !get_flag2(&self.lcdc, LCDCBits::LCDEnable as u8) {
                    // disable bits 0-2 if LCD is off
                    (0x80 | self.stat) & !0x3
                }
                else {
                    0x80 | self.stat
                }
            },

            // FF42 SCY - Scroll Y
            0xFF42 => self.scy, 

            // FF43 SCX - Scroll X
            0xFF43 => self.scx,

            // FF44 - LY - LCDC Y-Coordinate (R)
            0xFF44 => {
                if !get_flag2(&self.lcdc, 1 << 7) {
                    0
                }
                else {
                    self.ly
                }
            }

            // FF45 LYC - LY Compare (R/W)
            0xFF45 => self.lyc,

            // FF46 - DMA - DMA Transfer and Start Address (W)
            0xFF46 => 0,

            // FF47 - BGP - BG Palette Data (R/W)
            0xFF47 => self.bg_palette,

            // FF48 - OBP0 - Object Palette 0 Data (R/W)
            0xFF48 => self.obj_palette0,

            // FF49 - OBP1 - Object Palette 1 Data (R/W) 
            0xFF49 => self.obj_palette1,

            // FF4A WY - Window Y Position (R/W)
            0xFF4A => self.wpy,

            // FF4B WX - Window X Position minus 7 (R/W)
            0xFF4B => self.wpx,
            
            _ => 0
        }
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            0x8000..=0x9FFF => {
                self.vram[(address - 0x8000) as usize] = data;
            },

            0xFE00..=0xFE9F => {
                self.oam[(address - 0xFE00) as usize] = data;
            }

            // FF40 LCDC - LCD Control (R/W)
            0xFF40 => {
                self.lcdc = data;

                // TODO: This crashes super mario land... but it should be reset..
                // if !get_flag2(&self.lcdc, LCDCBits::LCDEnable as u8) {
                //     self.ly = 0;
                // }
            },

            // FF41 STAT - LCDC Status (R/W)
            0xFF41 => self.stat = data & !0x3 | self.stat & 0x3,

            // FF42 SCY - Scroll Y (R/W)
            0xFF42 => self.scy = data,  

            // FF43 SCX - Scroll X (R/W)
            0xFF43 => self.scx = data,

            // FF44 - LY - LCDC Y-Coordinate (R)
            0xFF44 => {
                // if LCD is disabled, then we reset LY
                if !get_flag2(&self.lcdc, 1 << 7) {
                    self.ly = 0;
                }
            },

            // FF45 LYC - LY Compare (R/W)
            0xFF45 => {
                self.lyc = data;
            },

            // FF46 - DMA - DMA Transfer and Start Address (W)
            0xFF46 => {
                // self.do_dma_transfer(data);
                self.dma_active = true;
                self.dma_source = data;
            },

            // FF47 - BGP - BG Palette Data (R/W)
            0xFF47 => self.bg_palette = data,

            // FF48 - OBP0 - Object Palette 0 Data (R/W)
            0xFF48 => self.obj_palette0 = data,

            // FF49 - OBP1 - Object Palette 1 Data (R/W) 
            0xFF49 => self.obj_palette1 = data,

            // FF4A WY - Window Y Position (R/W)
            0xFF4A => self.wpy = data,

            // FF4B WX - Window X Position minus 7 (R/W)
            0xFF4B => self.wpx = data,
            
            _ => println!("PPU Invalid Write")
        }
    }
}