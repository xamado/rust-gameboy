
pub struct Channel1 {
    enabled: bool,
    pub dac_enabled: bool,
    pub trigger: bool,
    
    pub length_counter: u8,
    pub length_counter_enabled: bool,
    
    pub envelope_timer: u8,
    pub envelope_direction: bool,
    pub envelope_period: u8,
    pub envelope_initial: u8,
    volume: u8,
    
    sweep_enabled: bool,
    pub sweep_period: u8,
    pub sweep_shift: u8,
    pub sweep_direction: bool,
    pub sweep_timer: i16,
    sweep_frequency_shadow: u16,
    
    waveforms: [[u8; 8]; 4],
    output_timer: i16,
    output_timer_period: u16,
    waveform_index: u8,
    
    pub frequency: u16,
    pub duty: u8,
    output: u8
}

impl Channel1 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            trigger: false,
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
            // self.sweep_enabled = false;
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

    pub fn trigger_channel(&mut self) {
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

    pub fn get_output(&self) -> u8 {
        self.output
    }
}