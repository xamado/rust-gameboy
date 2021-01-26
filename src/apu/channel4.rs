
const DIVISORS: [u8; 8] = [ 8, 16, 32, 48, 64, 80, 96, 112 ]; 

pub struct Channel4 {
    pub enabled: bool,
    pub dac_enabled: bool,
    output_timer: u16,
    output_timer_period: u16,
    volume: u8,

    lfsr: u16,
    width: bool,
    divisor_shift: u8,
    divisor: u8,

    length_counter: u8,
    length_counter_enabled: bool,

    envelope_timer: u8,
    envelope_direction: bool,
    envelope_period: u8,
    envelope_initial: u8,

    output: f32,
    output_length: u32,
}

impl Channel4 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            output_timer: 0,
            output_timer_period: 0,
            volume: 0,

            lfsr: 0xFFFF,
            width: false,
            divisor: 0,
            divisor_shift: 0,

            length_counter: 0,
            length_counter_enabled: false,

            envelope_timer: 0,
            envelope_initial: 0,
            envelope_period: 0,
            envelope_direction: false,

            output: 0.0,
            output_length: 0,
        }
    }

    pub fn tick(&mut self) {
        if self.output_timer > 0 {
            self.output_timer -= 1;
        }

        if self.output_timer == 0 {
            self.output_timer_period = (DIVISORS[self.divisor as usize] as u16) << self.divisor_shift;
            self.output_timer = self.output_timer_period;

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
        let output = if self.enabled && self.dac_enabled && (self.lfsr & 0x01) == 0 {
            self.volume
        }
        else {
            0
        };

        self.output += output as f32;
        self.output_length += 1;
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

    pub fn get_output(&mut self) -> f32 {
        let r = self.output / (self.output_length as f32);
        self.output = 0.0;
        self.output_length = 0;

        r
    }

    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            // NR41 - Channel 4 Sound Length (R/W)
            0xFF20 => 0xFF,

            // NR42 - Channel 4 Volume Envelope (R/W)
            0xFF21 => {
                self.envelope_initial << 4 |
                (self.envelope_direction as u8) << 3 |
                self.envelope_period & 0x3 
            }

            // NR43 - Channel 4 Polynomial Counter (R/W)
            0xFF22 => {
                self.divisor_shift << 4 |
                (self.width as u8) << 3 |
                self.divisor & 0x3 
            }

            // NR44 - Channel 4 Counter/consecutive; Inital (R/W)
            0xFF23 => 0xBF | ((self.length_counter_enabled as u8) << 6),

            _ => panic!("Invalid APU CH4 read")
        }
    }

    pub fn write_register(&mut self, addr: u16, data: u8) {
        match addr {
            // NR41 - Channel 4 Sound Length (R/W)
            0xFF20 => {
                self.length_counter = 64 - (data & 0x3F);
            },

            // NR42 - Channel 4 Volume Envelope (R/W)
            0xFF21 => {
                self.envelope_initial = data >> 4;
                self.envelope_direction = data & 0x08 != 0;
                self.envelope_period = data & 0x07;
                self.dac_enabled = data & 0xF8 != 0;

                self.envelope_timer = self.envelope_period;
            },

            // NR43 - Channel 4 Polynomial Counter (R/W)
            0xFF22 => {
                self.divisor_shift = (data & 0xF0) >> 4;
                self.width = (data & 0x08) != 0;
                self.divisor = data & 0x07;
            },

            // NR44 - Channel 4 Counter/consecutive; Inital (R/W)
            0xFF23 => {
                self.length_counter_enabled = (data & 0x40) != 0;
                let trigger = (data & 0x80) != 0;
                
                if trigger {
                    self.trigger_channel();
                }
            },

            _ => panic!("Invalid APU CH4 write"),
        }
    }

    fn trigger_channel(&mut self) {
        self.enabled = true;

        if self.length_counter == 0 {
            self.length_counter = 64;
        }

        self.envelope_timer = self.envelope_period;
        self.volume = self.envelope_initial;

        self.lfsr = 0x7FFF;
        
        self.output_timer_period = (DIVISORS[self.divisor as usize] as u16) << self.divisor_shift;
        self.output_timer = self.output_timer_period;
    }
}