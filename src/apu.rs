use self::channel1::Channel1;
use self::channel2::Channel2;
use self::channel3::Channel3;
use self::channel4::Channel4;

mod channel1;
mod channel2;
mod channel3;
mod channel4;

const FRAME_SEQUENCER_PERIOD: u16 = 8192; // clocks

struct APURegisters {
    sound_enabled: bool,
}

struct APUState {
    sample_tick: u16,
    frame_sequencer: u16,
    frame_sequencer_counter: u16,
}

pub struct APU {
    state: APUState,
    registers: APURegisters,
    channel1: Channel1,
    channel2: Channel2,
    channel3: Channel3,
    channel4: Channel4,
    samples: Vec<i16>,
}

impl APU {
    pub fn new() -> Self {
        Self {
            state: APUState {
                sample_tick: (4194304_u32 / 44100_u32) as u16,
                frame_sequencer: 0,
                frame_sequencer_counter: FRAME_SEQUENCER_PERIOD,
            },
            registers: APURegisters {
                sound_enabled: false,
            },
            channel1: Channel1::new(),
            channel2: Channel2::new(),
            channel3: Channel3::new(),
            channel4: Channel4::new(),
            samples: vec!(),
        }
    }

    pub fn consume_audio_samples(&mut self) -> Vec<i16> {
        let r = self.samples.to_owned();
        self.samples = vec!();
        
        r
    }

    pub fn tick(&mut self) {
        if !self.registers.sound_enabled {
            return;
        }

        self.state.frame_sequencer_counter = self.state.frame_sequencer_counter.wrapping_sub(1);
        if self.state.frame_sequencer_counter == 0 {
            self.state.frame_sequencer_counter = FRAME_SEQUENCER_PERIOD;
        
            self.tick_modulators();
        }

        self.channel1.tick();
        self.channel2.tick();
        self.channel3.tick();
        self.channel4.tick();
        
        // Mix accumulated samples to fill a buffer of 44100Hz
        self.state.sample_tick -= 1;
        if self.state.sample_tick == 0 {
            self.mix_samples();
            self.state.sample_tick = (4194304_u32 / 44100_u32) as u16;
        }
    }

    fn mix_samples(&mut self) {
        let sound1: f32 = self.channel1.get_output() as f32;
        let sound2: f32 = self.channel2.get_output() as f32;
        let sound3: f32 = self.channel3.get_output() as f32;
        let sound4: f32 = self.channel4.get_output();

        // DAC
        let dac_output_ch1 = sound1 / 15.0;
        let dac_output_ch2 = sound2 / 15.0;
        let dac_output_ch3 = sound3 / 15.0;
        let dac_output_ch4 = sound4 / 15.0;

        // mixer - average the 4 DAC outputs
        let left_sample = (dac_output_ch1 + dac_output_ch2 + dac_output_ch3 + dac_output_ch4) / 4.0;
        let right_sample = (dac_output_ch1 + dac_output_ch2 + dac_output_ch3 + dac_output_ch4) / 4.0;

        // L/R volume control
        let left_volume = 1.0;
        let right_volume = 1.0;

        let left = (left_sample * left_volume * (i16::MAX as f32)) as i16;
        let right = (right_sample * right_volume * (i16::MAX as f32)) as i16;

        self.samples.push(left);
        self.samples.push(right);
    }

    fn tick_modulators(&mut self) {
        self.state.frame_sequencer = self.state.frame_sequencer.wrapping_add(1);
        let step = self.state.frame_sequencer % 8;
        
        match step {
            0 | 4 => {
                self.channel1.tick_length_counter();
                self.channel2.tick_length_counter();
                self.channel3.tick_length_counter();
                self.channel4.tick_length_counter();
            },

            2 | 6 => {
                self.channel1.tick_length_counter();
                self.channel2.tick_length_counter();
                self.channel3.tick_length_counter();
                self.channel4.tick_length_counter();

                self.channel1.tick_sweep_counter();
            },

            7 => {
                self.channel1.tick_envelope_counter();
                self.channel2.tick_envelope_counter();
                self.channel4.tick_envelope_counter();
            },

            _ => {}
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        match address { 
            // Channel 1
            0xFF10 => {
                ((self.channel1.sweep_period & 0x70) << 4) | 
                (self.channel1.sweep_direction as u8) << 3 |
                (self.channel1.sweep_shift & 0x07)
            },

            0xFF11 => ((self.channel1.duty & 0x3) << 2) | (self.channel1.length_counter & 0x3F),

            0xFF12 => {
                (self.channel1.envelope_initial & 0x0F << 4) |
                (self.channel1.envelope_direction as u8) << 3 |
                (self.channel1.envelope_period & 0x7)
            },

            0xFF13 => {
                (self.channel1.frequency & 0xFF) as u8
            },

            0xFF14 => {
                ((self.channel1.frequency & 0x0700) >> 8) as u8 |
                (self.channel1.trigger as u8) << 7 |
                (self.channel1.length_counter_enabled as u8) << 6
            },

            // Channel 2
            0xFF16 => ((self.channel2.duty & 0x3) << 2) | (self.channel2.length_counter & 0x3F),

            0xFF17 => {
                (self.channel2.envelope_initial & 0x0F << 4) |
                (self.channel2.envelope_direction as u8) << 3 |
                (self.channel2.envelope_period & 0x7)
            },

            0xFF18 => {
                (self.channel2.frequency & 0xFF) as u8
            },

            0xFF19 => {
                ((self.channel2.frequency & 0x0700) >> 8) as u8 |
                (self.channel2.trigger as u8) << 7 |
                (self.channel2.length_counter_enabled as u8) << 6
            },

            // NR30 - Channel 3 Sound on/off (RW)
            0xFF1A => (self.channel3.dac_enabled as u8) << 7,

            // NR31 - Channel 3 Sound Length
            0xFF1B => self.channel3.length_counter,

            // NR32 - Channel 3 Select output level
            0xFF1C => (self.channel3.output_level & 0x3) << 5,

            // NR33 - Channel 3 Frequency lo
            0xFF1D => (self.channel3.frequency & 0xFF) as u8,

            // NR34 - Channel 3 Frequency hi
            0xFF1E => {
                ((self.channel3.trigger as u8) << 7) |
                ((self.channel3.length_counter_enabled as u8) << 6) |
                ((self.channel3.frequency & 0x700) >> 8) as u8
            }

            // NR41 - Channel 4 Sound Length (R/W)
            0xFF20 => self.channel4.length_counter & 0x1F | 0xFF,

            // NR42 - Channel 4 Volume Envelope (R/W)
            0xFF21 => {
                self.channel4.envelope_initial << 4 |
                (self.channel4.envelope_direction as u8) << 3 |
                self.channel4.envelope_period & 0x3 
            }

            // NR43 - Channel 4 Polynomial Counter (R/W)
            0xFF22 => {
                self.channel4.divisor_shift << 4 |
                (self.channel4.width as u8) << 3 |
                self.channel4.divisor & 0x3 
            }

            // NR44 - Channel 4 Counter/consecutive; Inital (R/W)
            0xFF23 => {
                (self.channel4.trigger as u8) << 7 |
                (self.channel4.length_counter_enabled as u8) << 6 |
                0xBF
            }
            
            _ => { /*println!("Invalid APU read");*/ 0 }
        }
    }

    pub fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            // NR10 Channel 1 Sweep Register (R/W)
            0xFF10 => {
                self.channel1.sweep_period = (data & 0x70) >> 4;
                self.channel1.sweep_direction = (data & 0x08) != 0;
                self.channel1.sweep_shift = data & 0x07;
            }

            // NR11 - Channel 1 Sound length / Wave pattern duty (R/W)
            0xFF11 => {
                self.channel1.length_counter = 64 - (data & 0x3F);
                self.channel1.duty = data >> 6;
            },

            // NR12 - Channel 1 Volume Envelope (R/W)
            0xFF12 => {
                self.channel1.envelope_initial = data >> 4;
                self.channel1.envelope_direction = data & 0x08 != 0;
                self.channel1.envelope_period = data & 0x07;
                self.channel1.dac_enabled = data & 0xF8 != 0;
                
                self.channel1.envelope_timer = self.channel1.envelope_period;
            }

            // NR13 - Channel 1 Frequency lo (W)
            0xFF13 => {
                self.channel1.frequency = (self.channel1.frequency & 0xFF00) | (data as u16);
            },

            // NR14 - Channel 1 Frequency hi (R/W)
            0xFF14 => {
                self.channel1.frequency = (((data as u16) & 0x07) << 8) | (self.channel1.frequency & 0x00FF);
                self.channel1.length_counter_enabled = data & 0x40 != 0;
                self.channel1.trigger = (data & 0x80) != 0;

                if self.channel1.trigger {
                    self.channel1.trigger_channel();
                }
            },



            // NR21 - Channel 2 Sound length / Wave pattern duty (R/W)
            0xFF16 => {
                self.channel2.length_counter = 64 - (data & 0x3F);
                self.channel2.duty = data >> 6;
            },

            // NR22 - Channel 2 Volume Envelope (R/W)
            0xFF17 => {
                self.channel2.envelope_initial = data >> 4;
                self.channel2.envelope_direction = data & 0x08 != 0;
                self.channel2.envelope_period = data & 0x07;
                self.channel2.dac_enabled = data & 0xF8 != 0;
                
                self.channel2.envelope_timer = self.channel2.envelope_period;
            }

            // NR23 - Channel 2 Frequency lo (W)
            0xFF18 => {
                self.channel2.frequency = (self.channel2.frequency & 0xFF00) | (data as u16);
            },

            // NR24 - Channel 2 Frequency hi (R/W)
            0xFF19 => {
                self.channel2.frequency = (((data as u16) & 0x07) << 8) | (self.channel2.frequency & 0x00FF);
                self.channel2.length_counter_enabled = data & 0x40 != 0;
                self.channel2.trigger = (data & 0x80) != 0;

                if self.channel2.trigger {
                    self.channel2.trigger_channel();
                }
            },
            
            // NR30 - Channel 3 Sound on/off (RW)
            0xFF1A => self.channel3.dac_enabled = (data & 0x80) != 0,

            // NR31 - Channel 3 Sound Length
            0xFF1B => self.channel3.length_counter = 255 - data,

            // NR32 - Channel 3 Select output level
            0xFF1C => self.channel3.output_level = (data & 0x60) >> 5,

            // NR33 - Channel 3 Frequency lo
            0xFF1D => self.channel3.frequency = (self.channel3.frequency & 0xFF00) | (data as u16),

            // NR34 - Channel 3 Frequency hi
            0xFF1E => {
                self.channel3.frequency = (((data as u16) & 0x07) << 8) | (self.channel3.frequency & 0x00FF);
                self.channel3.length_counter_enabled = data & 0x40 != 0;
                self.channel3.trigger = (data & 0x80) != 0;

                if self.channel3.trigger {
                    self.channel3.trigger_channel();
                }
            },

            // FF30-FF3F - Channel 3 Wave Pattern RAM
            0xFF30..=0xFF3F => {
                let idx = (address - 0xFF30) as usize;
                self.channel3.waveform_data[idx] = data;
            },

            // NR41 - Channel 4 Sound Length (R/W)
            0xFF20 => {
                self.channel4.length = data & 0x1F;
                self.channel4.length_counter = 64 - self.channel4.length;
            },

            // NR42 - Channel 4 Volume Envelope (R/W)
            0xFF21 => {
                self.channel4.envelope_initial = data >> 4;
                self.channel4.envelope_direction = data & 0x08 != 0;
                self.channel4.envelope_period = data & 0x07;
                self.channel4.dac_enabled = data & 0xF8 != 0;

                self.channel4.envelope_timer = self.channel4.envelope_period;
            },

            // NR43 - Channel 4 Polynomial Counter (R/W)
            0xFF22 => {
                self.channel4.divisor_shift = (data & 0xF0) >> 4;
                self.channel4.width = (data & 0x08) != 0;
                self.channel4.divisor = data & 0x07;
            },

            // NR44 - Channel 4 Counter/consecutive; Inital (R/W)
            0xFF23 => {
                self.channel4.length_counter_enabled = (data & 0x40) != 0;
                self.channel4.trigger = (data & 0x80) != 0;
                
                if self.channel4.trigger {
                    self.channel4.trigger_channel();
                }
            },

            // NR52
            0xFF26 => {
                self.registers.sound_enabled = (data & 1 << 7) != 0;
            },
            
            _ => { /*println!("Invalid APU write {:#06x} {:#04x}", address, data);*/ } 
        };
    }
}