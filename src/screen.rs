pub struct Screen {
    framebuffer: [u8; 160 * 144],
    vblank: bool
}

impl Screen {
    pub fn new() -> Self {
        Self {
            framebuffer: [0; 160 * 144],
            vblank: false
        }
    }

    pub fn get_framebuffer(&self) -> &[u8; 160 * 144] {
        &self.framebuffer
    }

    pub fn set_scanline(&mut self, line: u8, data: &[u8; 160]) {
        let start = line as usize * 160;
        let end = start + 160;

        self.framebuffer[start..end].copy_from_slice(data);

        // println!("set line {} {} {}", line, start, end);
    }

    pub fn is_vblank(&self) -> bool {
        self.vblank
    }

    pub fn set_vblank(&mut self, v: bool) {
        self.vblank = v;
    }
}