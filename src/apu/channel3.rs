
pub struct Channel3 {
    pub enabled: bool,
    pub dac_enabled: bool,
    pub trigger: bool,
    
    pub output_level: u8,

    pub length_counter: u8,
    pub length_counter_enabled: bool,
           
    waveform_timer_load: u16,
    waveform_timer: i16,
    waveform_position: u8,
    waveform_sample_buffer: u8,
    pub waveform_data: [u8; 16],
    
    pub frequency: u16,
    output: u8
}

impl Channel3 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            trigger: false,
            output: 0,
            output_level: 0,
            length_counter: 0,
            length_counter_enabled: false,
            frequency: 0,
            waveform_sample_buffer: 0,
            waveform_position: 0,
            waveform_timer: 0,
            waveform_timer_load: 0,
            waveform_data: [0; 16],
        }
    }

    pub fn tick(&mut self) {
        self.waveform_timer -= 1;
        if self.waveform_timer <= 0 {
            self.waveform_timer = self.waveform_timer_load as i16;

            self.waveform_position = (self.waveform_position + 1) % 32;

            let idx = self.waveform_position as usize / 2;
            let b = self.waveform_data[idx];
            self.waveform_sample_buffer = (b & (0xF << ((idx % 2) * 4))) >> ((idx % 2) * 4);
        }

        self.output = if self.enabled && self.dac_enabled {
            let s = match self.output_level {
                0 => 4,
                1 => 0,
                2 => 1,
                3 => 2,
                _ => 0,
            };

            self.waveform_sample_buffer >> s 
        }
        else {
            0
        }
    }

    pub fn tick_length_counter(&mut self) {
        if self.length_counter_enabled && self.length_counter > 0 {
            self.length_counter -= 1;

            if self.length_counter == 0 {
                self.enabled = false;
            }
        }
    }

    pub fn trigger_channel(&mut self) {
        self.enabled = true;

        if self.length_counter == 0 {
            self.length_counter = 255;
        }

        self.waveform_timer_load = (2048 - self.frequency) * 2;
        self.waveform_timer = self.waveform_timer_load as i16;
        self.waveform_position = 0;
    }

    pub fn get_output(&self) -> u8 {
        self.output
    }
}