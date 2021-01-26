use self::channel1::Channel1;
use self::channel2::Channel2;
use self::channel3::Channel3;
use self::channel4::Channel4;
use crate::bitutils::*;
mod channel1;
mod channel2;
mod channel3;
mod channel4;

const FRAME_SEQUENCER_PERIOD: u16 = 8192; // clocks

struct APURegisters {
    sound_enabled: bool,
    enabled_terminals: u8,
    left_volume: u8,
    right_volume: u8,
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
                enabled_terminals: 0,
                left_volume: 0,
                right_volume: 0,
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
        self.samples.clear();
        
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
        let dac_output_ch1 = if self.channel1.dac_enabled { (sound1 / 15.0) * 2.0 - 1.0 } else { 0.0 };
        let dac_output_ch2 = if self.channel2.dac_enabled { (sound2 / 15.0) * 2.0 - 1.0 } else { 0.0 };
        let dac_output_ch3 = if self.channel3.dac_enabled { (sound3 / 15.0) * 2.0 - 1.0 } else { 0.0 };
        let dac_output_ch4 = if self.channel4.dac_enabled { (sound4 / 15.0) * 2.0 - 1.0 } else { 0.0 };

        // mixer - average the 4 DAC outputs
        let mut right_sample = (dac_output_ch1 * get_bit(self.registers.enabled_terminals, 0) as f32 + 
                            dac_output_ch2 * get_bit(self.registers.enabled_terminals, 1) as f32 + 
                            dac_output_ch3 * get_bit(self.registers.enabled_terminals, 2) as f32 + 
                            dac_output_ch4 * get_bit(self.registers.enabled_terminals, 3) as f32 
                        ) / 4.0;
        let mut left_sample = (dac_output_ch1 * get_bit(self.registers.enabled_terminals, 4) as f32 + 
                            dac_output_ch2 * get_bit(self.registers.enabled_terminals, 5) as f32 + 
                            dac_output_ch3 * get_bit(self.registers.enabled_terminals, 6) as f32 + 
                            dac_output_ch4 * get_bit(self.registers.enabled_terminals, 7) as f32
                        ) / 4.0;

        // L/R volume control
        right_sample = (right_sample * self.registers.left_volume as f32 + 1.0) / 8.0;
        left_sample = (left_sample * self.registers.right_volume as f32 + 1.0) / 8.0;
        
        let left = (left_sample * (i16::MAX as f32)) as i16;
        let right = (right_sample * (i16::MAX as f32)) as i16;

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
            0xFF10..=0xFF14 => self.channel1.read_register(address),

            // Channel 2
            0xFF16..=0xFF19 => self.channel2.read_register(address),

            // Channel 3
            0xFF1A..=0xFF1E => self.channel3.read_register(address),

            // NR41 - Channel 4 Sound Length (R/W)
            0xFF20..=0xFF23 => self.channel4.read_register(address),

            // NR51 - Selection of Sound output terminal (R/W)
            0xFF25 => {
                self.registers.enabled_terminals
            },

            // FF26 - NR52 - Sound on/off
            0xFF26 => {
                0x70 | 
                (self.registers.sound_enabled as u8) << 7 |
                (self.channel4.enabled as u8) << 3 |
                (self.channel3.enabled as u8) << 2 |
                (self.channel2.enabled as u8) << 1 |
                (self.channel1.enabled as u8)

            }
            
            _ => { /*println!("Invalid APU read");*/ 0 }
        }
    }

    pub fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            // Channel 1
            0xFF10..=0xFF14 => self.channel1.write_register(address, data),

            // Channel 2
            0xFF16..=0xFF19 => self.channel2.write_register(address, data),

            // Channel 3
            0xFF1A..=0xFF1E | 0xFF30..=0xFF3F => self.channel3.write_register(address, data),

            // Channel 4
            0xFF20..=0xFF23 => self.channel4.write_register(address, data),

            // NR50 - Channel control / Volume
            0xFF24 => {
                self.registers.left_volume = (data & 0x70) >> 4;
                self.registers.right_volume = data & 0x7;
            }

            // NR51 - Selection of Sound output terminal (R/W)
            0xFF25 => {
                self.registers.enabled_terminals = data;
            },

            // NR52
            0xFF26 => {
                self.registers.sound_enabled = (data & 1 << 7) != 0;
            },
            
            _ => { /*println!("Invalid APU write {:#06x} {:#04x}", address, data);*/ } 
        };
    }
}