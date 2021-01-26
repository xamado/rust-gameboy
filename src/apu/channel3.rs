
pub struct Channel3 {
    pub enabled: bool,
    pub dac_enabled: bool,
    output_level: u8,
    length_counter: u16,
    length_counter_enabled: bool,
    waveform_timer_load: u16,
    waveform_timer: i16,
    waveform_position: u8,
    waveform_sample_buffer: u8,
    waveform_data: [u8; 16],
    frequency: u16,
    output: u8
}

impl Channel3 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
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

    pub fn get_output(&self) -> u8 {
        self.output
    }

    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            // NR30 - Channel 3 Sound on/off (RW)
            0xFF1A => 0x7F | ((self.dac_enabled as u8) << 7),

            // NR31 - Channel 3 Sound Length
            0xFF1B => 0xFF,

            // NR32 - Channel 3 Select output level
            0xFF1C => 0x9F | ((self.output_level & 0x3) << 5),

            // NR33 - Channel 3 Frequency lo
            0xFF1D => 0xFF,

            // NR34 - Channel 3 Frequency hi
            0xFF1E => 0xBF | ((self.length_counter_enabled as u8) << 6),

            _ => panic!("Invalid APU CH3 read")
        }
    }

    pub fn write_register(&mut self, addr: u16, data: u8) {
        match addr {
            // NR30 - Channel 3 Sound on/off (RW)
            0xFF1A => self.dac_enabled = (data & 0x80) != 0,

            // NR31 - Channel 3 Sound Length
            0xFF1B => self.length_counter = 256 - (data as u16),

            // NR32 - Channel 3 Select output level
            0xFF1C => self.output_level = (data & 0x60) >> 5,

            // NR33 - Channel 3 Frequency lo
            0xFF1D => self.frequency = (self.frequency & 0xFF00) | (data as u16),

            // NR34 - Channel 3 Frequency hi
            0xFF1E => {
                self.frequency = (((data as u16) & 0x07) << 8) | (self.frequency & 0x00FF);
                self.length_counter_enabled = data & 0x40 != 0;
                let trigger = (data & 0x80) != 0;

                if trigger {
                    self.trigger_channel();
                }
            },

            // FF30-FF3F - Channel 3 Wave Pattern RAM
            0xFF30..=0xFF3F => {
                let idx = (addr - 0xFF30) as usize;
                self.waveform_data[idx] = data;
            },

            _ => panic!("Invalid APU CH3 write"),
        }
    }

    fn trigger_channel(&mut self) {
        self.enabled = true;

        if self.length_counter == 0 {
            self.length_counter = 256;
        }

        self.waveform_timer_load = (2048 - self.frequency) * 2;
        self.waveform_timer = self.waveform_timer_load as i16;
        self.waveform_position = 0;
    }
}