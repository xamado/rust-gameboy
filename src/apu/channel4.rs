
const DIVISORS: [u8; 8] = [ 8, 16, 32, 48, 64, 80, 96, 112 ]; 

pub struct Channel4 {
    enabled: bool,
    pub dac_enabled: bool,
    pub trigger: bool,
    output: u8,
    output_timer: i16,
    output_timer_period: u16,
    volume: u8,

    lfsr: u16,
    pub width: bool,
    pub divisor_shift: u8,
    pub divisor: u8,

    pub length: u8,
    pub length_counter: u8,
    pub length_counter_enabled: bool,
    
    pub envelope_timer: u8,
    pub envelope_direction: bool,
    pub envelope_period: u8,
    pub envelope_initial: u8
}

impl Channel4 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            trigger: false,
            output: 0,
            output_timer: 0,
            output_timer_period: 0,
            volume: 0,

            lfsr: 0xFFFF,
            width: false,
            divisor: 0,
            divisor_shift: 0,

            length: 0,
            length_counter: 0,
            length_counter_enabled: false,

            envelope_timer: 0,
            envelope_initial: 0,
            envelope_period: 0,
            envelope_direction: false
        }
    }

    pub fn tick(&mut self) {
        self.output_timer -= 1;
        if self.output_timer <= 0 {
            self.output_timer_period = (DIVISORS[self.divisor as usize] as u16) << self.divisor_shift;

            self.output_timer = self.output_timer_period as i16;

            if self.divisor_shift != 14 && self.divisor_shift != 15 {
                let b = self.lfsr & 0x1 ^ ((self.lfsr >> 1) & 0x1);
                self.lfsr >>= 1;
                self.lfsr |= b << 14;

                // 7 bit mode
                if self.width {
                    //self.lfsr = (self.lfsr & !0x4) | (b << 6);
                    self.lfsr &= !0x40;
                    self.lfsr |= b << 6;
                }
            }
        }
        
        // output volume
        self.output = if self.enabled && self.dac_enabled && (self.lfsr & 0x01) == 0 {
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
                if self.envelope_direction && self.volume < 15 {
                    self.volume = self.volume.wrapping_add(1);
                }
                else if self.volume > 0 {
                    self.volume = self.volume.wrapping_sub(1);
                }
            }
        }
    }

    pub fn trigger_channel(&mut self) {
        self.enabled = true;
        // if self.length_counter == 0 {
            self.length_counter = 64 - self.length;
        // }

        self.envelope_timer = self.envelope_period;
        self.volume = self.envelope_initial;

        self.lfsr = 0x7FFF;
        
        self.output_timer_period = (DIVISORS[self.divisor as usize] as u16) << self.divisor_shift;
        self.output_timer = self.output_timer_period as i16;
    }

    pub fn get_output(&self) -> u8 {
        self.output
    }
}