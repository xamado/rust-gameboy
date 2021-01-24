use core::cell::RefCell;

use crate::iomapped::IOMapped;
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
    state: RefCell<APUState>,
    registers: RefCell<APURegisters>,    
    channel1: RefCell<Channel1>,
    channel2: RefCell<Channel2>,
    channel3: RefCell<Channel3>,
    channel4: RefCell<Channel4>,
    samples: RefCell<Vec<i16>>,
}

impl APU {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(APUState {
                sample_tick: (4194304_u32 / 44100_u32) as u16,
                frame_sequencer: 0,
                frame_sequencer_counter: FRAME_SEQUENCER_PERIOD,
            }),
            registers: RefCell::new(APURegisters {
                sound_enabled: false,
            }),
            channel1: RefCell::new(Channel1::new()),
            channel2: RefCell::new(Channel2::new()),
            channel3: RefCell::new(Channel3::new()),
            channel4: RefCell::new(Channel4::new()),
            samples: RefCell::new(vec!()),
        }
    }

    pub fn consume_audio_samples(&mut self) -> Vec<i16> {
        let mut samples = self.samples.borrow_mut();

        let r = samples.to_owned();
        *samples = vec!();
        
        r
    }

    pub fn tick(&self) {
        if !self.registers.borrow().sound_enabled {
            return;
        }

        let mut state = self.state.borrow_mut();

        state.frame_sequencer_counter = state.frame_sequencer_counter.wrapping_sub(1);
        if state.frame_sequencer_counter == 0 {
            state.frame_sequencer_counter = FRAME_SEQUENCER_PERIOD;
        
            self.tick_modulators(&mut state);
        }

        self.channel1.borrow_mut().tick();
        self.channel2.borrow_mut().tick();
        self.channel3.borrow_mut().tick();
        self.channel4.borrow_mut().tick();
        
        // Mix accumulated samples to fill a buffer of 44100Hz
        state.sample_tick -= 1;
        if state.sample_tick == 0 {
            self.mix_samples();
            state.sample_tick = (4194304_u32 / 44100_u32) as u16;
        }
    }

    fn mix_samples(&self) {
        let sound1: f32 = self.channel1.borrow().get_output() as f32;
        let sound2: f32 = self.channel2.borrow().get_output() as f32;
        let sound3: f32 = self.channel3.borrow().get_output() as f32;
        let sound4: f32 = self.channel4.borrow_mut().get_output();

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

        let mut samples = self.samples.borrow_mut();
        samples.push(left);
        samples.push(right);
    }

    fn tick_modulators(&self, state: &mut APUState) {
        state.frame_sequencer = state.frame_sequencer.wrapping_add(1);
        let step = state.frame_sequencer % 8;
        
        match step {
            0 | 4 => {
                self.channel1.borrow_mut().tick_length_counter();
                self.channel2.borrow_mut().tick_length_counter();
                self.channel3.borrow_mut().tick_length_counter();
                self.channel4.borrow_mut().tick_length_counter();
            },

            2 | 6 => {
                self.channel1.borrow_mut().tick_length_counter();
                self.channel2.borrow_mut().tick_length_counter();
                self.channel3.borrow_mut().tick_length_counter();
                self.channel4.borrow_mut().tick_length_counter();

                self.channel1.borrow_mut().tick_sweep_counter();
            },

            7 => {
                self.channel1.borrow_mut().tick_envelope_counter();
                self.channel2.borrow_mut().tick_envelope_counter();
                self.channel4.borrow_mut().tick_envelope_counter();
            },

            _ => {}
        }
    }
}

impl IOMapped for APU {
    fn read_byte(&self, address: u16) -> u8 {
        let channel1 = self.channel1.borrow();
        let channel2 = self.channel2.borrow();
        let channel3 = self.channel3.borrow();
        let channel4 = self.channel4.borrow();

        match address { 
            // Channel 1
            0xFF10 => {
                ((channel1.sweep_period & 0x70) << 4) | 
                (channel1.sweep_direction as u8) << 3 |
                (channel1.sweep_shift & 0x07)
            },

            0xFF11 => ((channel1.duty & 0x3) << 2) | (channel1.length_counter & 0x3F),

            0xFF12 => {
                (channel1.envelope_initial & 0x0F << 4) |
                (channel1.envelope_direction as u8) << 3 |
                (channel1.envelope_period & 0x7)
            },

            0xFF13 => {
                (channel1.frequency & 0xFF) as u8
            },

            0xFF14 => {
                ((channel1.frequency & 0x0700) >> 8) as u8 |
                (channel1.trigger as u8) << 7 |
                (channel1.length_counter_enabled as u8) << 6
            },

            // Channel 2
            0xFF16 => ((channel2.duty & 0x3) << 2) | (channel2.length_counter & 0x3F),

            0xFF17 => {
                (channel2.envelope_initial & 0x0F << 4) |
                (channel2.envelope_direction as u8) << 3 |
                (channel2.envelope_period & 0x7)
            },

            0xFF18 => {
                (channel2.frequency & 0xFF) as u8
            },

            0xFF19 => {
                ((channel2.frequency & 0x0700) >> 8) as u8 |
                (channel2.trigger as u8) << 7 |
                (channel2.length_counter_enabled as u8) << 6
            },

            // NR30 - Channel 3 Sound on/off (RW)
            0xFF1A => (channel3.dac_enabled as u8) << 7,

            // NR31 - Channel 3 Sound Length
            0xFF1B => channel3.length_counter,

            // NR32 - Channel 3 Select output level
            0xFF1C => (channel3.output_level & 0x3) << 5,

            // NR33 - Channel 3 Frequency lo
            0xFF1D => (channel3.frequency & 0xFF) as u8,

            // NR34 - Channel 3 Frequency hi
            0xFF1E => {
                ((channel3.trigger as u8) << 7) |
                ((channel3.length_counter_enabled as u8) << 6) |
                ((channel3.frequency & 0x700) >> 8) as u8
            }

            // NR41 - Channel 4 Sound Length (R/W)
            0xFF20 => channel4.length_counter & 0x1F | 0xFF,

            // NR42 - Channel 4 Volume Envelope (R/W)
            0xFF21 => {
                channel4.envelope_initial << 4 |
                (channel4.envelope_direction as u8) << 3 |
                channel4.envelope_period & 0x3 
            }

            // NR43 - Channel 4 Polynomial Counter (R/W)
            0xFF22 => {
                channel4.divisor_shift << 4 |
                (channel4.width as u8) << 3 |
                channel4.divisor & 0x3 
            }

            // NR44 - Channel 4 Counter/consecutive; Inital (R/W)
            0xFF23 => {
                (channel4.trigger as u8) << 7 |
                (channel4.length_counter_enabled as u8) << 6 |
                0xBF
            }
            
            _ => { /*println!("Invalid APU read");*/ 0 }
        }
    }

    fn write_byte(&self, address: u16, data: u8) {
        let mut channel1 = self.channel1.borrow_mut();
        let mut channel2 = self.channel2.borrow_mut();
        let mut channel3 = self.channel3.borrow_mut();
        let mut channel4 = self.channel4.borrow_mut();

        match address {
            // NR10 Channel 1 Sweep Register (R/W)
            0xFF10 => {
                channel1.sweep_period = (data & 0x70) >> 4;
                channel1.sweep_direction = (data & 0x08) != 0;
                channel1.sweep_shift = data & 0x07;
            }

            // NR11 - Channel 1 Sound length / Wave pattern duty (R/W)
            0xFF11 => {
                channel1.length_counter = 64 - (data & 0x3F);
                channel1.duty = data >> 6;
            },

            // NR12 - Channel 1 Volume Envelope (R/W)
            0xFF12 => {
                channel1.envelope_initial = data >> 4;
                channel1.envelope_direction = data & 0x08 != 0;
                channel1.envelope_period = data & 0x07;
                channel1.dac_enabled = data & 0xF8 != 0;
                
                channel1.envelope_timer = channel1.envelope_period;
            }

            // NR13 - Channel 1 Frequency lo (W)
            0xFF13 => {
                channel1.frequency = (channel1.frequency & 0xFF00) | (data as u16);
            },

            // NR14 - Channel 1 Frequency hi (R/W)
            0xFF14 => {
                channel1.frequency = (((data as u16) & 0x07) << 8) | (channel1.frequency & 0x00FF);
                channel1.length_counter_enabled = data & 0x40 != 0;
                channel1.trigger = (data & 0x80) != 0;

                if channel1.trigger {
                    channel1.trigger_channel();
                }
            },



            // NR21 - Channel 2 Sound length / Wave pattern duty (R/W)
            0xFF16 => {
                channel2.length_counter = 64 - (data & 0x3F);
                channel2.duty = data >> 6;
            },

            // NR22 - Channel 2 Volume Envelope (R/W)
            0xFF17 => {
                channel2.envelope_initial = data >> 4;
                channel2.envelope_direction = data & 0x08 != 0;
                channel2.envelope_period = data & 0x07;
                channel2.dac_enabled = data & 0xF8 != 0;
                
                channel2.envelope_timer = channel2.envelope_period;
            }

            // NR23 - Channel 2 Frequency lo (W)
            0xFF18 => {
                channel2.frequency = (channel2.frequency & 0xFF00) | (data as u16);
            },

            // NR24 - Channel 2 Frequency hi (R/W)
            0xFF19 => {
                channel2.frequency = (((data as u16) & 0x07) << 8) | (channel2.frequency & 0x00FF);
                channel2.length_counter_enabled = data & 0x40 != 0;
                channel2.trigger = (data & 0x80) != 0;

                if channel2.trigger {
                    channel2.trigger_channel();
                }
            },
            
            // NR30 - Channel 3 Sound on/off (RW)
            0xFF1A => channel3.dac_enabled = (data & 0x80) != 0,

            // NR31 - Channel 3 Sound Length
            0xFF1B => channel3.length_counter = 255 - data,

            // NR32 - Channel 3 Select output level
            0xFF1C => channel3.output_level = (data & 0x60) >> 5,

            // NR33 - Channel 3 Frequency lo
            0xFF1D => channel3.frequency = (channel3.frequency & 0xFF00) | (data as u16),

            // NR34 - Channel 3 Frequency hi
            0xFF1E => {
                channel3.frequency = (((data as u16) & 0x07) << 8) | (channel3.frequency & 0x00FF);
                channel3.length_counter_enabled = data & 0x40 != 0;
                channel3.trigger = (data & 0x80) != 0;

                if channel3.trigger {
                    channel3.trigger_channel();
                }
            },

            // FF30-FF3F - Channel 3 Wave Pattern RAM
            0xFF30..=0xFF3F => {
                let idx = (address - 0xFF30) as usize;
                channel3.waveform_data[idx] = data;
            },

            // NR41 - Channel 4 Sound Length (R/W)
            0xFF20 => {
                channel4.length = data & 0x1F;
                channel4.length_counter = 64 - channel4.length;
            },

            // NR42 - Channel 4 Volume Envelope (R/W)
            0xFF21 => {
                channel4.envelope_initial = data >> 4;
                channel4.envelope_direction = data & 0x08 != 0;
                channel4.envelope_period = data & 0x07;
                channel4.dac_enabled = data & 0xF8 != 0;

                channel4.envelope_timer = channel4.envelope_period;
            },

            // NR43 - Channel 4 Polynomial Counter (R/W)
            0xFF22 => {
                channel4.divisor_shift = (data & 0xF0) >> 4;
                channel4.width = (data & 0x08) != 0;
                channel4.divisor = data & 0x07;
            },

            // NR44 - Channel 4 Counter/consecutive; Inital (R/W)
            0xFF23 => {
                channel4.length_counter_enabled = (data & 0x40) != 0;
                channel4.trigger = (data & 0x80) != 0;
                
                if channel4.trigger {
                    channel4.trigger_channel();
                }
            },

            // NR52
            0xFF26 => {
                self.registers.borrow_mut().sound_enabled = (data & 1 << 7) != 0;
            },
            
            _ => { /*println!("Invalid APU write {:#06x} {:#04x}", address, data);*/ } 
        };
    }
}