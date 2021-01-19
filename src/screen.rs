use crate::machine::GameBoyModel;

pub struct Screen {
    model: GameBoyModel, 
    framebuffer: Box<[u32]>,
    vblank: bool
}

const DMG_SCREEN_COLORS: [u32; 4] = [
    0xFFFFFF,
    0x7E7E7E,
    0x575757,
    0x000000,
];

impl Screen {
    pub fn new(model: GameBoyModel) -> Self {
        Self {
            model,
            framebuffer: vec!(0; 160*144).into_boxed_slice(),
            vblank: false
        }
    }

    pub fn get_framebuffer(&self) -> &[u32] {
        &self.framebuffer
    }

    pub fn set_scanline(&mut self, line: u8, data: &[u16; 160]) {
        let rng = (line as usize * 160)..(line as usize * 160 + 160);

        let colors: Vec<u32>;

        match self.model {
            GameBoyModel::DMG => {
                colors = data.iter().map(|v| -> u32 { 
                    DMG_SCREEN_COLORS[*v as usize] 
                }).collect();
            }

            GameBoyModel::GBC => {
                colors = data.iter().map(|v| {
                    let r = ((((v & 0x1F) as f32) / 31.0) * 255.0) as u32;
                    let g = ((((v >> 5) & 0x1F) as f32 / 31.0) * 255.0) as u32;
                    let b = ((((v >> 10) & 0x1F) as f32 / 31.0) * 255.0) as u32;
        
                    // r << 24 | g << 16 | b << 8
                    b << 16 | g << 8 | r
                }).collect();
            }
        }        

        self.framebuffer[rng].copy_from_slice(&colors);
    }

    pub fn is_vblank(&self) -> bool {
        self.vblank
    }

    pub fn set_vblank(&mut self, v: bool) {
        self.vblank = v;
    }
}