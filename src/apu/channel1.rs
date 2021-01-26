
pub struct Channel1 {
    pub enabled: bool,
    pub dac_enabled: bool,
    output_timer: i16,
    output_timer_period: u16,
    volume: u8,
    frequency: u16,
    
    length_counter: u8,
    length_counter_enabled: bool,
    
    envelope_timer: u8,
    envelope_direction: bool,
    envelope_period: u8,
    envelope_initial: u8,
    
    sweep_enabled: bool,
    sweep_period: u8,
    sweep_shift: u8,
    sweep_direction: bool,
    sweep_timer: i16,
    sweep_frequency_shadow: u16,
    
    waveforms: [[u8; 8]; 4],
    waveform_index: u8,
    
    duty: u8,
    output: u8
}

impl Channel1 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            output: 0,
            output_timer: 0,
            volume: 0,

            length_counter: 0,
            length_counter_enabled: false,
            envelope_timer: 0,
            envelope_initial: 0,
            envelope_period: 0,
            envelope_direction: false,
            sweep_enabled: false,
            sweep_period: 0,
            sweep_direction: false,
            sweep_shift: 0,
            sweep_timer: 0,
            frequency: 0,
            sweep_frequency_shadow: 0,
            output_timer_period: 0,
            waveform_index: 0,
            waveforms: [
                [ 0, 0, 0, 0, 0, 0, 0, 1 ],
                [ 1, 0, 0, 0, 0, 0, 0, 1 ],
                [ 1, 0, 0, 0, 0, 1, 1, 1 ],
                [ 0, 1, 1, 1, 1, 1, 1, 0 ]
            ],
            duty: 0
        }
    }
    
    pub fn tick(&mut self) {
        self.output_timer -= 1;
        if self.output_timer <= 0 {
            self.output_timer = self.output_timer_period as i16;

            self.waveform_index = (self.waveform_index + 1) % 8;
        }

        // output volume
        let waveform_value = self.waveforms[self.duty as usize][self.waveform_index as usize];
        
        self.output = if self.enabled && self.dac_enabled && waveform_value != 0 {
            self.volume
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

    pub fn tick_sweep_counter(&mut self) {
        self.sweep_timer -= 1;

        if self.sweep_timer <= 0 { // TODO: try to use unsigned
            self.sweep_timer = self.sweep_period as i16;

            if self.sweep_enabled && self.sweep_period != 0 {
                let frequency = self.sweep_calculate_frequency();

                if frequency < 2047 && self.sweep_shift != 0 {
                    self.frequency = frequency;
                    self.sweep_frequency_shadow = frequency;

                    self.output_timer_period = (2048 - self.frequency) * 4;
                }

                self.sweep_calculate_frequency();
            }
        }
    }

    pub fn sweep_calculate_frequency(&mut self) -> u16 {
        let mut frequency = self.sweep_frequency_shadow >> self.sweep_shift;

        if !self.sweep_direction {
            frequency = self.sweep_frequency_shadow.wrapping_add(frequency);
        }
        else {
            frequency = self.sweep_frequency_shadow.wrapping_sub(frequency);
        }

        if frequency > 2047 {
            self.enabled = false;
        }

        frequency
    }

    pub fn tick_envelope_counter(&mut self) {
        if self.envelope_timer == 0 && self.envelope_period == 0 {
            return;
        }

        self.envelope_timer -= 1;

        if self.envelope_timer == 0 {
            self.envelope_timer = self.envelope_period;

            if self.envelope_period != 0 {
                let volume;

                if self.envelope_direction {
                    volume = self.volume.wrapping_add(1);
                }
                else {
                    volume = self.volume.wrapping_sub(1);
                }

                if volume <= 15 {
                    self.volume = volume;
                }
            }
        }
    }

    pub fn get_output(&self) -> u8 {
        self.output
    }

    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            // NR10 Channel 1 Sweep Register (R/W)
            0xFF10 => {
                0x80 |
                ((self.sweep_period & 0x07) << 4) | 
                (self.sweep_direction as u8) << 3 |
                (self.sweep_shift & 0x07)
            },

            // NR11 - Channel 1 Sound length / Wave pattern duty (R/W)
            0xFF11 => 0x3F | ((self.duty & 0x3) << 6),

            // NR12 - Channel 1 Volume Envelope (R/W)
            0xFF12 => {
                (self.envelope_initial & 0x0F << 4) |
                (self.envelope_direction as u8) << 3 |
                (self.envelope_period & 0x7)
            },

            // NR13 - Channel 1 Frequency lo (W)
            0xFF13 => 0xFF,

            0xFF14 => {
                0xBF | ((self.length_counter_enabled as u8) << 6)
            },

            _ => panic!("Invalid APU CH1 read")
        }
    }

    pub fn write_register(&mut self, addr: u16, data: u8) {
        match addr {
            // NR10 Channel 1 Sweep Register (R/W)
            0xFF10 => {
                self.sweep_period = (data & 0x70) >> 4;
                self.sweep_direction = (data & 0x08) != 0;
                self.sweep_shift = data & 0x07;
            }

            // NR11 - Channel 1 Sound length / Wave pattern duty (R/W)
            0xFF11 => {
                self.length_counter = 64 - (data & 0x3F);
                self.duty = data >> 6;
            },

            // NR12 - Channel 1 Volume Envelope (R/W)
            0xFF12 => {
                self.envelope_initial = data >> 4;
                self.envelope_direction = data & 0x08 != 0;
                self.envelope_period = data & 0x07;
                self.dac_enabled = data & 0xF8 != 0;
                
                self.envelope_timer = self.envelope_period;
            }

            // NR13 - Channel 1 Frequency lo (W)
            0xFF13 => {
                self.frequency = (self.frequency & 0xFF00) | (data as u16);
            },

            // NR14 - Channel 1 Frequency hi (R/W)
            0xFF14 => {
                self.frequency = (((data as u16) & 0x07) << 8) | (self.frequency & 0x00FF);
                self.length_counter_enabled = data & 0x40 != 0;
                let trigger = (data & 0x80) != 0;

                if trigger {
                    self.trigger_channel();
                }
            },

            _ => panic!("Invalid APU CH1 write"),
        }
    }

    fn trigger_channel(&mut self) {
        self.enabled = true;

        if self.length_counter == 0 {
            self.length_counter = 64;
        }

        self.envelope_timer = self.envelope_period;
        self.volume = self.envelope_initial;

        self.output_timer_period = (2048 - self.frequency) * 4;
        self.output_timer = self.output_timer_period as i16;

        self.sweep_frequency_shadow = self.frequency;
        self.sweep_timer = self.sweep_period as i16;
        self.sweep_enabled = self.sweep_period != 0 || self.sweep_shift != 0;
        if self.sweep_shift != 0 {
            self.frequency = self.sweep_calculate_frequency();
        }
    }
}