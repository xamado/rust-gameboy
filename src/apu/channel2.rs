
pub struct Channel2 {
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
       
    waveforms: [[u8; 8]; 4],
    output_timer: i16,
    waveform_timer_load: u16,
    waveform_value: u8,
    
    pub frequency: u16,
    pub duty: u8,
    output: u8
}

impl Channel2 {
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
            frequency: 0,
            waveform_timer_load: 0,
            waveform_value: 0,
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
            self.output_timer = self.waveform_timer_load as i16;

            self.waveform_value = (self.waveform_value + 1) % 8;
        }

        // output volume
        let waveform_value = self.waveforms[self.duty as usize][self.waveform_value as usize];
        
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

        self.waveform_timer_load = (2048 - self.frequency) * 4;
        self.output_timer = self.waveform_timer_load as i16;
    }

    pub fn get_output(&self) -> u8 {
        self.output
    }
}