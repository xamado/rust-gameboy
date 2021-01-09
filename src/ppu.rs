use std::rc::Rc;
use core::cell::RefCell;

use crate::memorybus::MemoryBus;
use crate::screen::Screen;
use crate::cpu::Interrupts;
use crate::iomapped::IOMapped;
use crate::bitutils::*;

const MAX_SCANLINES: u8 = 154;
const VBLANK_LINE: u8 = 144;
const TILE_WIDTH: u16 = 8;
const TILE_HEIGHT: u16 = 8;
const TILES_PER_ROW: u16 = 32;
const TILES_PER_COL: u16 = 32;
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

#[allow(unused)]
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

    pub fn step(&mut self, cycles: u8) {
        // in theory dma copy takes a while... in fact:
        // This copy needs 160 × 4 + 4 clocks to
        // complete in both double speed and single speeds modes. The copy starts after the 4 setup clocks,
        // and a new byte is copied every 4 clocks.
        if self.dma_active {
            self.do_dma_transfer(self.dma_source);
            self.dma_active = false;
        }

        // If LCD is not ENABLED do nothing
        if !get_flag2(&self.lcdc, 1 << 7) {
            return;
        }

        let mut mode = self.stat & 0x3;
        self.line_cycles += cycles as u32;
        self.total_vblank_cycles += cycles as u32;
        
        match mode {
            // OAM access mode
            2 => { 
                // wait for 82 cycles, then go to mode VRAM READ MODE
                if self.line_cycles >= 82 {
                    mode = 3;
                }
            },

            // VRAM read mode
            3 => {
                if self.line_cycles >= 252 {
                    // draw scanline
                    self.render_scanline();

                    mode = 0;
                    self.check_stat_interrupts();
                }
            },

            // HBLANK
            0 => { 
                if self.line_cycles >= 456 {
                    self.line_cycles -= 456;
                    
                    // update LY
                    self.ly = (self.ly + 1) % MAX_SCANLINES;
                    set_flag2(&mut self.stat, 1 << 6, self.ly == self.lyc);

                    if get_flag2(&self.stat, 1 << 6) {
                        // raise the stat interrupt
                        let iif = self.bus.borrow().read_byte(0xFF0F) | (1 << Interrupts::LCDStat as u8);
                        self.bus.borrow_mut().write_byte(0xFF0F, iif);
                    }

                    if self.ly == VBLANK_LINE { // ly = 144 
                        mode = 1;

                        // raise the VBlank interrupt
                        let iif = self.bus.borrow().read_byte(0xFF0F) | (1 << Interrupts::VBlank as u8);
                        self.bus.borrow_mut().write_byte(0xFF0F, iif);
                    }
                    else {
                        mode = 2;
                    }

                    self.check_stat_interrupts();
                }
            },

            // VBLANK
            1 => { 
                if self.line_cycles >= 456 {
                    self.line_cycles -= 456;

                    // update LY
                    self.ly = (self.ly + 1) % MAX_SCANLINES;
                    set_flag2(&mut self.stat, 1 << 6, self.ly == self.lyc);

                    if get_flag2(&self.stat, 1 << 6) {
                        // raise the stat interrupt
                        let iif = self.bus.borrow().read_byte(0xFF0F) | (1 << Interrupts::LCDStat as u8);
                        self.bus.borrow_mut().write_byte(0xFF0F, iif);
                    }

                    if self.ly == 0 {
                        self.screen.borrow_mut().set_vblank(true);
                        self.total_vblank_cycles = 0;
                        mode = 2;
                        self.check_stat_interrupts();
                    }
                }
            },

            _ => panic!("PPU Invalid mode")
        }

        self.stat = self.stat & 0x7C | mode;
    }

    fn check_stat_interrupts(&self) {
        let mask = 1 << (3 + (self.stat & 0x3));

        if self.stat & mask != 0 {
            // raise the stat interrupt
            let iif = self.bus.borrow().read_byte(0xFF0F) | (1 << Interrupts::LCDStat as u8);
            self.bus.borrow_mut().write_byte(0xFF0F, iif);
        }
    }

    fn render_scanline(&mut self) {
        //let bus = self.bus.borrow();

        //let obj1_palette: u8 = bus.read_byte(0xFF48);
        // let obj2_palette: u8 = bus.read_byte(0xFF49);

        let bg_enabled = (self.lcdc & 1) != 0;
        // let w_enabled = (reg_lcdc & (1 << 5)) != 0;
        let obj_enabled = (self.lcdc & (1 << 1)) != 0;
        
        let display_select = self.lcdc & (LCDCBits::BackgroundTilemapDisplaySelect as u8);
        let bg_tile_map_address: u16 = if (display_select) != 0 { 0x9C00 } else { 0x9800 };

        let scx: u16 = self.scx as u16;
        let scy: u16 = self.scy as u16;

        let start_tile_row: u16 = (scy + self.ly as u16) / TILE_HEIGHT;
        let start_tile_col: u16 = scx / TILE_WIDTH;
        let end_tile_col: u16 = start_tile_col + 21;

        let pixel_row = (scy + self.ly as u16) % TILE_HEIGHT;

        let mut scanline_buffer: [u8; 160] = [0; 160];
        let mut pixel_idx = 0;

        if bg_enabled {
            let addressing_mode = self.lcdc & LCDCBits::TileDataSelect as u8;
            let tile_data_base_address: u16 = if addressing_mode != 0 { 0x8000 } else { 0x8800 };

            for x in start_tile_col..end_tile_col {
                // read tile number from tile map
                let tile_address = bg_tile_map_address + (((TILES_PER_ROW * (start_tile_row as u16 % TILES_PER_ROW)) + (x as u16 % TILES_PER_COL)) as u16);
                let tile_number: u8 = self.bus.borrow().read_byte(tile_address);

                // read tile data
                let tile_index: u8 = if addressing_mode != 0 { tile_number } else { ((tile_number as i16) + 128) as u8 };
                let tile_row_data = self.read_tile_data(tile_data_base_address, tile_index, pixel_row as u8);

                let mut pixel_col = x * TILE_WIDTH;

                for i in 0..TILE_WIDTH {
                    if pixel_col >= scx && pixel_col <= scx + 160 && pixel_idx < 160 {
                        let color_idx = tile_row_data[i as usize] & 0x03;
                        let color = (self.bg_palette & (3 << (color_idx * 2))) >> (color_idx * 2);
                        scanline_buffer[pixel_idx] = color;
                        pixel_idx += 1;
                    }
                    
                    pixel_col += 1;
                }
            }
        }

        if obj_enabled {
            let tile_data_base_address: u16 = 0x8000;

            let mode_8x16 = (self.lcdc & (1 << 2)) != 0;
            let mut obj_count: u8 = 0;

            for i in 0..40 {
                if obj_count > 10 {
                    break;
                }

                let obj = self.read_oam_entry(i);
                if obj.x == 0 || obj.x >= 168 || obj.y == 0 || obj.y >= 160 {
                    continue;
                }

                let x: i16 = obj.x as i16 - 8;
                let y: i16 = obj.y as i16 - 16;
                let height = if mode_8x16 { 16 } else { 8 };

                if (self.ly as i16) < y || (self.ly as i16) >= y + (height as i16) {
                    continue;
                }
                
                let flip_x = (obj.flags & OBJAttributes::XFlip as u8) != 0;
                let flip_y = (obj.flags & OBJAttributes::YFlip as u8) != 0;
                let priority = (obj.flags & OBJAttributes::Priority as u8) != 0;

                let mut row = (self.ly as i16) - y;
                if flip_y {
                    row = height - row - 1;
                }

                let obj_tile_data = self.read_tile_data(tile_data_base_address, obj.tile, row as u8);

                for p in 0..8 {
                    if (x + (p as i16)) < 0 || (x + (p as i16)) >= 160 {
                        continue;
                    }

                    // let idx = (x as u8 + p) as usize;
                    let idx = (x as u8).wrapping_add(p) as usize;
                    let color_idx = if flip_x { obj_tile_data[TILE_WIDTH as usize - 1 - p as usize] } else { obj_tile_data[p as usize] };
                    let color = (self.obj_palette0 & (3 << (color_idx * 2))) >> (color_idx * 2);

                    if color == 0 {
                        continue;
                    }

                    if priority && scanline_buffer[idx] != 0 {
                        continue;
                    }

                    scanline_buffer[idx] = color;
                }

                obj_count += 1;
            }
        }

        self.screen.borrow_mut().set_scanline(self.ly as u8, &scanline_buffer);
    }

    fn read_tile_data(&self, base_address: u16, tile_number: u8, row: u8) -> [u8; 8] {
        let bus = self.bus.borrow();

        let tile_address = base_address + (tile_number as u16 * TILE_SIZE as u16);

        let offset: u16 = row as u16 * 2;
        let mut tile_row: [u8; 8] = [0; 8];
        
        let lsb = bus.read_byte(tile_address + offset);
        let msb = bus.read_byte(tile_address + offset + 1);

        for bit in (0..TILE_WIDTH).rev() {
            let mask: u8 = 1 << bit;
            let color: u8 = (((msb & mask) >> bit) << 1) | ((lsb & mask) >> bit);

            tile_row[(7 - bit) as usize] = color;
        }

        tile_row
    }

    fn read_oam_entry(&self, idx: u8) -> OAMEntry {
        OAMEntry {
            y: self.bus.borrow().read_byte(OAM_TABLE_ADDRESS + idx as u16 * 4),
            x: self.bus.borrow().read_byte(OAM_TABLE_ADDRESS + idx  as u16 * 4 + 1),
            tile: self.bus.borrow().read_byte(OAM_TABLE_ADDRESS + idx as u16 * 4 + 2),
            flags: self.bus.borrow().read_byte(OAM_TABLE_ADDRESS + idx as u16 * 4 + 3)
        }
    }

    fn do_dma_transfer(&self, data: u8) {
        let addr: u16 = (data as u16) << 8;
        let mut data: [u8; 0xA0] = [0; 0xA0];
        for i in 0..0xA0 {
            let bus = self.bus.borrow();
            data[i] = bus.read_byte(addr + (i as u16));
        }

        for i in 0..0xA0 {
            let mut bus = self.bus.borrow_mut();
            bus.write_byte(0xFE00 + (i as u16), data[i]);
        }
    }
}

impl IOMapped for PPU {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            // FF40 LCDC - LCD Control (R/W)
            0xFF40 => self.lcdc,

            // FF41 STAT - LCDC Status (R/W)
            0xFF41 => 0x80 | self.stat,

            // FF42 SCY - Scroll Y
            0xFF42 => self.scy, 

            // FF43 SCX - Scroll X
            0xFF43 => self.scx,

            // FF44 - LY - LCDC Y-Coordinate (R)
            0xFF44 => self.ly,

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
            // FF40 LCDC - LCD Control (R/W)
            0xFF40 => self.lcdc = data,

            // FF41 STAT - LCDC Status (R/W)
            0xFF41 => self.stat = data & !0x3 | self.stat & 0x3,

            // FF42 SCY - Scroll Y (R/W)
            0xFF42 => self.scy = data,  

            // FF43 SCX - Scroll X (R/W)
            0xFF43 => self.scx = data,

            // FF44 - LY - LCDC Y-Coordinate (R)
            0xFF44 => {},

            // FF45 LYC - LY Compare (R/W)
            0xFF45 => self.lyc = data,

            // FF46 - DMA - DMA Transfer and Start Address (W)
            0xFF46 => {
                // self.do_dma_transfer(data);
                self.dma_active = true;
                self.dma_source = data;
            }

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