use crate::memorybus::MemoryBus;
use crate::bitutils::*;

use std::collections::HashMap;
use std::rc::Rc;
use core::cell::RefCell;

const FLAG_Z: u8 = 1 << 7;
const FLAG_N: u8 = 1 << 6;
const FLAG_H: u8 = 1 << 5;
const FLAG_C: u8 = 1 << 4;

#[derive(PartialEq)]
enum CPUState {
    Normal,
    Halt,
    Stop
}

#[derive(Clone)]
struct Instruction {
    dissassembly: String,
    bytes: u16,
    func: fn(&mut CPU) -> u8
}

struct Registers {
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16
}

#[derive(Copy, Clone)]
pub enum Interrupts {
    VBlank = 0,
    LCDStat,
    Timer,
    Serial,
    Joypad
}

const INTERRUPT_ADDRESS : [u16; 5] = [
    0x0040,
    0x0048,
    0x0050,
    0x0058,
    0x0060
];

pub struct CPU {
    state: CPUState,
    registers: Registers,
    instructions: HashMap<u16, Instruction>,
    bus: Rc<RefCell<MemoryBus>>,
    interrupts_enabled: bool,
    interrupts_enable_request: bool,
    debug: bool
}

impl CPU {
    pub fn new(bus: Rc<RefCell<MemoryBus>>) -> Self {
        let instruction_table : HashMap<u16, Instruction> = [
            (0x0000_u16, Instruction { dissassembly: String::from("NOP"), bytes: 1, func: CPU::op_nop }),
            (0x0010_u16, Instruction { dissassembly: String::from("STOP"), bytes: 2, func: CPU::op_stop }),
            (0x0076_u16, Instruction { dissassembly: String::from("HALT"), bytes: 1, func: CPU::op_halt }),

            (0x003C_u16, Instruction { dissassembly: String::from("INC A"), bytes: 1, func: CPU::op_inc_a }),
            (0x0004_u16, Instruction { dissassembly: String::from("INC B"), bytes: 1, func: CPU::op_inc_b }),
            (0x000C_u16, Instruction { dissassembly: String::from("INC C"), bytes: 1, func: CPU::op_inc_c }),
            (0x0014_u16, Instruction { dissassembly: String::from("INC D"), bytes: 1, func: CPU::op_inc_d }),
            (0x001C_u16, Instruction { dissassembly: String::from("INC E"), bytes: 1, func: CPU::op_inc_e }),
            (0x0024_u16, Instruction { dissassembly: String::from("INC H"), bytes: 1, func: CPU::op_inc_h }),
            (0x002C_u16, Instruction { dissassembly: String::from("INC L"), bytes: 1, func: CPU::op_inc_l }),
            (0x0003_u16, Instruction { dissassembly: String::from("INC BC"), bytes: 1, func: CPU::op_inc_bc }),
            (0x0013_u16, Instruction { dissassembly: String::from("INC DE"), bytes: 1, func: CPU::op_inc_de }),
            (0x0023_u16, Instruction { dissassembly: String::from("INC HL"), bytes: 1, func: CPU::op_inc_hl }),
            (0x0033_u16, Instruction { dissassembly: String::from("INC SP"), bytes: 1, func: CPU::op_inc_sp }),
            (0x0034_u16, Instruction { dissassembly: String::from("INC (HL)"), bytes: 1, func: CPU::op_inc_mem_hl }),
            (0x003D_u16, Instruction { dissassembly: String::from("DEC A"), bytes: 1, func: CPU::op_dec_a }),
            (0x0005_u16, Instruction { dissassembly: String::from("DEC B"), bytes: 1, func: CPU::op_dec_b }),
            (0x000D_u16, Instruction { dissassembly: String::from("DEC C"), bytes: 1, func: CPU::op_dec_c }),
            (0x0015_u16, Instruction { dissassembly: String::from("DEC D"), bytes: 1, func: CPU::op_dec_d }),
            (0x001D_u16, Instruction { dissassembly: String::from("DEC E"), bytes: 1, func: CPU::op_dec_e }),
            (0x0025_u16, Instruction { dissassembly: String::from("DEC H"), bytes: 1, func: CPU::op_dec_h }),
            (0x002D_u16, Instruction { dissassembly: String::from("DEC L"), bytes: 1, func: CPU::op_dec_l }),
            (0x000B_u16, Instruction { dissassembly: String::from("DEC BC"), bytes: 1, func: CPU::op_dec_bc }),
            (0x001B_u16, Instruction { dissassembly: String::from("DEC DE"), bytes: 1, func: CPU::op_dec_de }),
            (0x002B_u16, Instruction { dissassembly: String::from("DEC HL"), bytes: 1, func: CPU::op_dec_hl }),
            (0x003B_u16, Instruction { dissassembly: String::from("DEC SP"), bytes: 1, func: CPU::op_dec_sp }),
            (0x0035_u16, Instruction { dissassembly: String::from("DEC (HL)"), bytes: 1, func: CPU::op_dec_mem_hl }),
            (0x0087_u16, Instruction { dissassembly: String::from("ADD A,A"), bytes: 1, func: CPU::op_add_a_a }),
            (0x0080_u16, Instruction { dissassembly: String::from("ADD A,B"), bytes: 1, func: CPU::op_add_a_b }),
            (0x0081_u16, Instruction { dissassembly: String::from("ADD A,C"), bytes: 1, func: CPU::op_add_a_c }),
            (0x0082_u16, Instruction { dissassembly: String::from("ADD A,D"), bytes: 1, func: CPU::op_add_a_d }),
            (0x0083_u16, Instruction { dissassembly: String::from("ADD A,E"), bytes: 1, func: CPU::op_add_a_e }),
            (0x0084_u16, Instruction { dissassembly: String::from("ADD A,H"), bytes: 1, func: CPU::op_add_a_h }),
            (0x0085_u16, Instruction { dissassembly: String::from("ADD A,L"), bytes: 1, func: CPU::op_add_a_l }),
            (0x00C6_u16, Instruction { dissassembly: String::from("ADD A,d8"), bytes: 2, func: CPU::op_add_a_d8 }),
            (0x0086_u16, Instruction { dissassembly: String::from("ADD A,(HL)"), bytes: 1, func: CPU::op_add_a_mem_hl }),
            (0x0009_u16, Instruction { dissassembly: String::from("ADD HL,BC"), bytes: 1, func: CPU::op_add_hl_bc }),
            (0x0019_u16, Instruction { dissassembly: String::from("ADD HL,DE"), bytes: 1, func: CPU::op_add_hl_de }),
            (0x0029_u16, Instruction { dissassembly: String::from("ADD HL,HL"), bytes: 1, func: CPU::op_add_hl_hl }),
            (0x0039_u16, Instruction { dissassembly: String::from("ADD HL,SP"), bytes: 1, func: CPU::op_add_hl_sp }),
            (0x00E8_u16, Instruction { dissassembly: String::from("ADD SP,s8"), bytes: 2, func: CPU::op_add_sp_s8 }),
            (0x0097_u16, Instruction { dissassembly: String::from("SUB A"), bytes: 1, func: CPU::op_sub_a }),
            (0x0090_u16, Instruction { dissassembly: String::from("SUB B"), bytes: 1, func: CPU::op_sub_b }),
            (0x0091_u16, Instruction { dissassembly: String::from("SUB C"), bytes: 1, func: CPU::op_sub_c }),
            (0x0092_u16, Instruction { dissassembly: String::from("SUB D"), bytes: 1, func: CPU::op_sub_d }),
            (0x0093_u16, Instruction { dissassembly: String::from("SUB E"), bytes: 1, func: CPU::op_sub_e }),
            (0x0094_u16, Instruction { dissassembly: String::from("SUB H"), bytes: 1, func: CPU::op_sub_h }),
            (0x0095_u16, Instruction { dissassembly: String::from("SUB L"), bytes: 1, func: CPU::op_sub_l }),
            (0x00D6_u16, Instruction { dissassembly: String::from("SUB d8"), bytes: 2, func: CPU::op_sub_d8 }),
            (0x0096_u16, Instruction { dissassembly: String::from("SUB (HL)"), bytes: 1, func: CPU::op_sub_mem_hl }),
            (0x008F_u16, Instruction { dissassembly: String::from("ADC A,A"), bytes: 1, func: CPU::op_adc_a_a }),
            (0x0088_u16, Instruction { dissassembly: String::from("ADC A,B"), bytes: 1, func: CPU::op_adc_a_b }),
            (0x0089_u16, Instruction { dissassembly: String::from("ADC A,C"), bytes: 1, func: CPU::op_adc_a_c }),
            (0x008A_u16, Instruction { dissassembly: String::from("ADC A,D"), bytes: 1, func: CPU::op_adc_a_d }),
            (0x008B_u16, Instruction { dissassembly: String::from("ADC A,E"), bytes: 1, func: CPU::op_adc_a_e }),
            (0x008C_u16, Instruction { dissassembly: String::from("ADC A,H"), bytes: 1, func: CPU::op_adc_a_h }),
            (0x008D_u16, Instruction { dissassembly: String::from("ADC A,L"), bytes: 1, func: CPU::op_adc_a_l }),
            (0x00CE_u16, Instruction { dissassembly: String::from("ADC A,d8"), bytes: 2, func: CPU::op_adc_a_d8 }),
            (0x008E_u16, Instruction { dissassembly: String::from("ADC A,(HL)"), bytes: 1, func: CPU::op_adc_a_mem_hl }),
            (0x009F_u16, Instruction { dissassembly: String::from("SBC A,A"), bytes: 1, func: CPU::op_sbc_a_a }),
            (0x0098_u16, Instruction { dissassembly: String::from("SBC A,B"), bytes: 1, func: CPU::op_sbc_a_b }),
            (0x0099_u16, Instruction { dissassembly: String::from("SBC A,C"), bytes: 1, func: CPU::op_sbc_a_c }),
            (0x009A_u16, Instruction { dissassembly: String::from("SBC A,D"), bytes: 1, func: CPU::op_sbc_a_d }),
            (0x009B_u16, Instruction { dissassembly: String::from("SBC A,E"), bytes: 1, func: CPU::op_sbc_a_e }),
            (0x009C_u16, Instruction { dissassembly: String::from("SBC A,H"), bytes: 1, func: CPU::op_sbc_a_h }),
            (0x009D_u16, Instruction { dissassembly: String::from("SBC A,L"), bytes: 1, func: CPU::op_sbc_a_l }),
            (0x00DE_u16, Instruction { dissassembly: String::from("SBC A,d8"), bytes: 2, func: CPU::op_sbc_a_d8 }),
            (0x009E_u16, Instruction { dissassembly: String::from("SBC A,(HL)"), bytes: 2, func: CPU::op_sbc_a_mem_hl }),

            (0x0027_u16, Instruction { dissassembly: String::from("DAA"), bytes: 1, func: CPU::op_daa }),
            (0x0037_u16, Instruction { dissassembly: String::from("SCF"), bytes: 1, func: CPU::op_scf }),
            (0x003F_u16, Instruction { dissassembly: String::from("CCF"), bytes: 1, func: CPU::op_ccf }),

            (0x00BF_u16, Instruction { dissassembly: String::from("CP A"), bytes: 1, func: CPU::op_cp_a }),
            (0x00B8_u16, Instruction { dissassembly: String::from("CP B"), bytes: 1, func: CPU::op_cp_b }),
            (0x00B9_u16, Instruction { dissassembly: String::from("CP C"), bytes: 1, func: CPU::op_cp_c }),
            (0x00BA_u16, Instruction { dissassembly: String::from("CP D"), bytes: 1, func: CPU::op_cp_d }),
            (0x00BB_u16, Instruction { dissassembly: String::from("CP E"), bytes: 1, func: CPU::op_cp_e }),
            (0x00BC_u16, Instruction { dissassembly: String::from("CP H"), bytes: 1, func: CPU::op_cp_h }),
            (0x00BD_u16, Instruction { dissassembly: String::from("CP L"), bytes: 1, func: CPU::op_cp_l }),
            (0x00FE_u16, Instruction { dissassembly: String::from("CP d8"), bytes: 1, func: CPU::op_cp_d8 }),
            (0x00BE_u16, Instruction { dissassembly: String::from("CP (HL)"), bytes: 1, func: CPU::op_cp_mem_hl }),

            // LOAD instructions
            (0x007F_u16, Instruction { dissassembly: String::from("LD A,A"), bytes: 1, func: CPU::op_ld_a_a }),
            (0x0078_u16, Instruction { dissassembly: String::from("LD A,B"), bytes: 1, func: CPU::op_ld_a_b }),
            (0x0079_u16, Instruction { dissassembly: String::from("LD A,C"), bytes: 1, func: CPU::op_ld_a_c }),
            (0x007A_u16, Instruction { dissassembly: String::from("LD A,D"), bytes: 1, func: CPU::op_ld_a_d }),
            (0x007B_u16, Instruction { dissassembly: String::from("LD A,E"), bytes: 1, func: CPU::op_ld_a_e }),
            (0x007C_u16, Instruction { dissassembly: String::from("LD A,H"), bytes: 1, func: CPU::op_ld_a_h }),
            (0x007D_u16, Instruction { dissassembly: String::from("LD A,L"), bytes: 1, func: CPU::op_ld_a_l }),
            (0x0047_u16, Instruction { dissassembly: String::from("LD B,A"), bytes: 1, func: CPU::op_ld_b_a }),
            (0x0040_u16, Instruction { dissassembly: String::from("LD B,B"), bytes: 1, func: CPU::op_ld_b_b }),
            (0x0041_u16, Instruction { dissassembly: String::from("LD B,C"), bytes: 1, func: CPU::op_ld_b_c }),
            (0x0042_u16, Instruction { dissassembly: String::from("LD B,D"), bytes: 1, func: CPU::op_ld_b_d }),
            (0x0043_u16, Instruction { dissassembly: String::from("LD B,E"), bytes: 1, func: CPU::op_ld_b_e }),
            (0x0044_u16, Instruction { dissassembly: String::from("LD B,H"), bytes: 1, func: CPU::op_ld_b_h }),
            (0x0045_u16, Instruction { dissassembly: String::from("LD B,L"), bytes: 1, func: CPU::op_ld_b_l }),
            (0x004F_u16, Instruction { dissassembly: String::from("LD C,A"), bytes: 1, func: CPU::op_ld_c_a }),
            (0x0048_u16, Instruction { dissassembly: String::from("LD C,B"), bytes: 1, func: CPU::op_ld_c_b }),
            (0x0049_u16, Instruction { dissassembly: String::from("LD C,C"), bytes: 1, func: CPU::op_ld_c_c }),
            (0x004A_u16, Instruction { dissassembly: String::from("LD C,D"), bytes: 1, func: CPU::op_ld_c_d }),
            (0x004B_u16, Instruction { dissassembly: String::from("LD C,E"), bytes: 1, func: CPU::op_ld_c_e }),
            (0x004C_u16, Instruction { dissassembly: String::from("LD C,H"), bytes: 1, func: CPU::op_ld_c_h }),
            (0x004D_u16, Instruction { dissassembly: String::from("LD C,L"), bytes: 1, func: CPU::op_ld_c_l }),
            (0x0057_u16, Instruction { dissassembly: String::from("LD D,A"), bytes: 1, func: CPU::op_ld_d_a }),
            (0x0050_u16, Instruction { dissassembly: String::from("LD D,B"), bytes: 1, func: CPU::op_ld_d_b }),
            (0x0051_u16, Instruction { dissassembly: String::from("LD D,C"), bytes: 1, func: CPU::op_ld_d_c }),
            (0x0052_u16, Instruction { dissassembly: String::from("LD D,D"), bytes: 1, func: CPU::op_ld_d_d }),
            (0x0053_u16, Instruction { dissassembly: String::from("LD D,E"), bytes: 1, func: CPU::op_ld_d_e }),
            (0x0054_u16, Instruction { dissassembly: String::from("LD D,H"), bytes: 1, func: CPU::op_ld_d_h }),
            (0x0055_u16, Instruction { dissassembly: String::from("LD D,L"), bytes: 1, func: CPU::op_ld_d_l }),
            (0x005F_u16, Instruction { dissassembly: String::from("LD E,A"), bytes: 1, func: CPU::op_ld_e_a }),
            (0x0058_u16, Instruction { dissassembly: String::from("LD E,B"), bytes: 1, func: CPU::op_ld_e_b }),
            (0x0059_u16, Instruction { dissassembly: String::from("LD E,C"), bytes: 1, func: CPU::op_ld_e_c }),
            (0x005A_u16, Instruction { dissassembly: String::from("LD E,D"), bytes: 1, func: CPU::op_ld_e_d }),
            (0x005B_u16, Instruction { dissassembly: String::from("LD E,E"), bytes: 1, func: CPU::op_ld_e_e }),
            (0x005C_u16, Instruction { dissassembly: String::from("LD E,H"), bytes: 1, func: CPU::op_ld_e_h }),
            (0x005D_u16, Instruction { dissassembly: String::from("LD E,L"), bytes: 1, func: CPU::op_ld_e_l }),
            (0x0067_u16, Instruction { dissassembly: String::from("LD H,A"), bytes: 1, func: CPU::op_ld_h_a }),
            (0x0060_u16, Instruction { dissassembly: String::from("LD H,B"), bytes: 1, func: CPU::op_ld_h_b }),
            (0x0061_u16, Instruction { dissassembly: String::from("LD H,C"), bytes: 1, func: CPU::op_ld_h_c }),
            (0x0062_u16, Instruction { dissassembly: String::from("LD H,D"), bytes: 1, func: CPU::op_ld_h_d }),
            (0x0063_u16, Instruction { dissassembly: String::from("LD H,E"), bytes: 1, func: CPU::op_ld_h_e }),
            (0x0064_u16, Instruction { dissassembly: String::from("LD H,H"), bytes: 1, func: CPU::op_ld_h_h }),
            (0x0065_u16, Instruction { dissassembly: String::from("LD H,L"), bytes: 1, func: CPU::op_ld_h_l }),
            (0x0066_u16, Instruction { dissassembly: String::from("LD H,(HL)"), bytes: 1, func: CPU::op_ld_h_mem_hl }),
            (0x006F_u16, Instruction { dissassembly: String::from("LD L,A"), bytes: 1, func: CPU::op_ld_l_a }),
            (0x0068_u16, Instruction { dissassembly: String::from("LD L,B"), bytes: 1, func: CPU::op_ld_l_b }),
            (0x0069_u16, Instruction { dissassembly: String::from("LD L,C"), bytes: 1, func: CPU::op_ld_l_c }),
            (0x006A_u16, Instruction { dissassembly: String::from("LD L,D"), bytes: 1, func: CPU::op_ld_l_d }),
            (0x006B_u16, Instruction { dissassembly: String::from("LD L,E"), bytes: 1, func: CPU::op_ld_l_e }),
            (0x006C_u16, Instruction { dissassembly: String::from("LD L,H"), bytes: 1, func: CPU::op_ld_l_h }),
            (0x006D_u16, Instruction { dissassembly: String::from("LD L,L"), bytes: 1, func: CPU::op_ld_l_l }),
            (0x006E_u16, Instruction { dissassembly: String::from("LD L,(HL)"), bytes: 1, func: CPU::op_ld_l_mem_hl }),
            (0x003E_u16, Instruction { dissassembly: String::from("LD A,d8"), bytes: 2, func: CPU::op_ld_a_d8 }),
            (0x0006_u16, Instruction { dissassembly: String::from("LD B,d8"), bytes: 2, func: CPU::op_ld_b_d8 }),
            (0x000E_u16, Instruction { dissassembly: String::from("LD C,d8"), bytes: 2, func: CPU::op_ld_c_d8 }),
            (0x0016_u16, Instruction { dissassembly: String::from("LD D,d8"), bytes: 2, func: CPU::op_ld_d_d8 }),
            (0x001E_u16, Instruction { dissassembly: String::from("LD E,d8"), bytes: 2, func: CPU::op_ld_e_d8 }),
            (0x002E_u16, Instruction { dissassembly: String::from("LD L,d8"), bytes: 2, func: CPU::op_ld_l_d8 }),
            (0x0026_u16, Instruction { dissassembly: String::from("LD H,d8"), bytes: 2, func: CPU::op_ld_h_d8 }),
            (0x0001_u16, Instruction { dissassembly: String::from("LD BC,d16"), bytes: 3, func: CPU::op_ld_bc_d16 }),
            (0x0011_u16, Instruction { dissassembly: String::from("LD DE,d16"), bytes: 3, func: CPU::op_ld_de_d16 }),
            (0x0021_u16, Instruction { dissassembly: String::from("LD HL,d16"), bytes: 3, func: CPU::op_ld_hl_d16 }),
            (0x0031_u16, Instruction { dissassembly: String::from("LD SP,d16"), bytes: 3, func: CPU::op_ld_sp_d16 }),
            (0x00F9_u16, Instruction { dissassembly: String::from("LD SP,HL"), bytes: 1, func: CPU::op_ld_sp_hl }),
            (0x00F8_u16, Instruction { dissassembly: String::from("LD HL,SP+s8"), bytes: 2, func: CPU::op_ld_hl_sp_add_s8 }),
            (0x00F2_u16, Instruction { dissassembly: String::from("LD A,(C)"), bytes: 1, func: CPU::op_ld_a_mem_c }),
            (0x000A_u16, Instruction { dissassembly: String::from("LD A,(BC)"), bytes: 1, func: CPU::op_ld_a_mem_bc }),
            (0x001A_u16, Instruction { dissassembly: String::from("LD A,(DE)"), bytes: 1, func: CPU::op_ld_a_mem_de }),
            (0x007E_u16, Instruction { dissassembly: String::from("LD A,(HL)"), bytes: 1, func: CPU::op_ld_a_mem_hl }),
            (0x0046_u16, Instruction { dissassembly: String::from("LD B,(HL)"), bytes: 1, func: CPU::op_ld_b_mem_hl }),
            (0x004E_u16, Instruction { dissassembly: String::from("LD C,(HL)"), bytes: 1, func: CPU::op_ld_c_mem_hl }),
            (0x0056_u16, Instruction { dissassembly: String::from("LD D,(HL)"), bytes: 1, func: CPU::op_ld_d_mem_hl }),
            (0x005E_u16, Instruction { dissassembly: String::from("LD E,(HL)"), bytes: 1, func: CPU::op_ld_e_mem_hl }),
            (0x00F0_u16, Instruction { dissassembly: String::from("LD A,(d8)"), bytes: 2, func: CPU::op_ld_a_mem_d8 }),
            (0x00FA_u16, Instruction { dissassembly: String::from("LD A,(a16)"), bytes: 3, func: CPU::op_ld_a_mem_a16 }),
            (0x002A_u16, Instruction { dissassembly: String::from("LD A,(HL+)"), bytes: 1, func: CPU::op_ld_a_mem_hl_inc }),
            (0x003A_u16, Instruction { dissassembly: String::from("LD A,(HL-)"), bytes: 1, func: CPU::op_ld_a_mem_hl_dec }),
            (0x00E2_u16, Instruction { dissassembly: String::from("LD (C),A"), bytes: 1, func: CPU::op_ld_mem_c_a }),
            (0x0002_u16, Instruction { dissassembly: String::from("LD (BC),A"), bytes: 1, func: CPU::op_ld_mem_bc_a }),
            (0x0012_u16, Instruction { dissassembly: String::from("LD (DE),A"), bytes: 1, func: CPU::op_ld_mem_de_a }),
            (0x0077_u16, Instruction { dissassembly: String::from("LD (HL),A"), bytes: 1, func: CPU::op_ld_mem_hl_a }),
            (0x0070_u16, Instruction { dissassembly: String::from("LD (HL),B"), bytes: 1, func: CPU::op_ld_mem_hl_b }),
            (0x0071_u16, Instruction { dissassembly: String::from("LD (HL),C"), bytes: 1, func: CPU::op_ld_mem_hl_c }),
            (0x0072_u16, Instruction { dissassembly: String::from("LD (HL),D"), bytes: 1, func: CPU::op_ld_mem_hl_d }),
            (0x0073_u16, Instruction { dissassembly: String::from("LD (HL),E"), bytes: 1, func: CPU::op_ld_mem_hl_e }),
            (0x0074_u16, Instruction { dissassembly: String::from("LD (HL),H"), bytes: 2, func: CPU::op_ld_mem_hl_h }),
            (0x0075_u16, Instruction { dissassembly: String::from("LD (HL),L"), bytes: 2, func: CPU::op_ld_mem_hl_l }),
            (0x0032_u16, Instruction { dissassembly: String::from("LD (HL-),A"), bytes: 1, func: CPU::op_ld_mem_hl_dec_a }),
            (0x0022_u16, Instruction { dissassembly: String::from("LD (HL+),A"), bytes: 1, func: CPU::op_ld_mem_hl_inc_a }),
            (0x0036_u16, Instruction { dissassembly: String::from("LD (HL),d8"), bytes: 1, func: CPU::op_ld_mem_hl_d8 }),
            (0x00E0_u16, Instruction { dissassembly: String::from("LD (a8),A"), bytes: 2, func: CPU::op_ld_mem_a8_a }),
            (0x00EA_u16, Instruction { dissassembly: String::from("LD (a16),A"), bytes: 3, func: CPU::op_ld_mem_a16_a }),
            (0x0008_u16, Instruction { dissassembly: String::from("LD (a16),SP"), bytes: 3, func: CPU::op_ld_mem_a16_sp }),

            (0x00A7_u16, Instruction { dissassembly: String::from("AND A"), bytes: 1, func: CPU::op_and_a }),
            (0x00A0_u16, Instruction { dissassembly: String::from("AND B"), bytes: 1, func: CPU::op_and_b }),
            (0x00A1_u16, Instruction { dissassembly: String::from("AND C"), bytes: 1, func: CPU::op_and_c }),
            (0x00A2_u16, Instruction { dissassembly: String::from("AND D"), bytes: 1, func: CPU::op_and_d }),
            (0x00A3_u16, Instruction { dissassembly: String::from("AND E"), bytes: 1, func: CPU::op_and_e }),
            (0x00A4_u16, Instruction { dissassembly: String::from("AND H"), bytes: 1, func: CPU::op_and_h }),
            (0x00A5_u16, Instruction { dissassembly: String::from("AND L"), bytes: 1, func: CPU::op_and_l }),
            (0x00E6_u16, Instruction { dissassembly: String::from("AND d8"), bytes: 2, func: CPU::op_and_d8 }),
            (0x00A6_u16, Instruction { dissassembly: String::from("AND (HL)"), bytes: 1, func: CPU::op_and_mem_hl }),

            (0x00B7_u16, Instruction { dissassembly: String::from("OR A"), bytes: 1, func: CPU::op_or_a }),
            (0x00B0_u16, Instruction { dissassembly: String::from("OR B"), bytes: 1, func: CPU::op_or_b}),
            (0x00B1_u16, Instruction { dissassembly: String::from("OR C"), bytes: 1, func: CPU::op_or_c }),
            (0x00B2_u16, Instruction { dissassembly: String::from("OR D"), bytes: 1, func: CPU::op_or_d }),
            (0x00B3_u16, Instruction { dissassembly: String::from("OR E"), bytes: 1, func: CPU::op_or_e }),
            (0x00B4_u16, Instruction { dissassembly: String::from("OR H"), bytes: 1, func: CPU::op_or_h }),
            (0x00B5_u16, Instruction { dissassembly: String::from("OR L"), bytes: 1, func: CPU::op_or_l }),
            (0x00F6_u16, Instruction { dissassembly: String::from("OR d8"), bytes: 2, func: CPU::op_or_d8 }),
            (0x00B6_u16, Instruction { dissassembly: String::from("OR (HL)"), bytes: 1, func: CPU::op_or_mem_hl }),

            (0x00AF_u16, Instruction { dissassembly: String::from("XOR A"), bytes: 1, func: CPU::op_xor_a }),
            (0x00A8_u16, Instruction { dissassembly: String::from("XOR B"), bytes: 1, func: CPU::op_xor_b }),
            (0x00A9_u16, Instruction { dissassembly: String::from("XOR C"), bytes: 1, func: CPU::op_xor_c }),
            (0x00AA_u16, Instruction { dissassembly: String::from("XOR D"), bytes: 1, func: CPU::op_xor_d }),
            (0x00AB_u16, Instruction { dissassembly: String::from("XOR E"), bytes: 1, func: CPU::op_xor_e }),
            (0x00AC_u16, Instruction { dissassembly: String::from("XOR H"), bytes: 1, func: CPU::op_xor_h }),
            (0x00AD_u16, Instruction { dissassembly: String::from("XOR L"), bytes: 1, func: CPU::op_xor_l }),
            (0x00EE_u16, Instruction { dissassembly: String::from("XOR d8"), bytes: 2, func: CPU::op_xor_d8 }),
            (0x00AE_u16, Instruction { dissassembly: String::from("XOR (HL)"), bytes: 1, func: CPU::op_xor_mem_hl }),
            
            (0x002F_u16, Instruction { dissassembly: String::from("CPL"), bytes: 1, func: CPU::op_cpl }),
            
            (0x00E9_u16, Instruction { dissassembly: String::from("JP HL"), bytes: 1, func: CPU::op_jp_hl }),
            (0x00C3_u16, Instruction { dissassembly: String::from("JP a16"), bytes: 3, func: CPU::op_jp_a16 }),
            (0x00C2_u16, Instruction { dissassembly: String::from("JP NZ,a16"), bytes: 3, func: CPU::op_jp_nz_a16 }),
            (0x00CA_u16, Instruction { dissassembly: String::from("JP Z,a16"), bytes: 3, func: CPU::op_jp_z_a16 }),
            (0x00D2_u16, Instruction { dissassembly: String::from("JP NC,a16"), bytes: 3, func: CPU::op_jp_nc_a16 }),
            (0x00DA_u16, Instruction { dissassembly: String::from("JP C,a16"), bytes: 3, func: CPU::op_jp_c_a16 }),
            (0x0018_u16, Instruction { dissassembly: String::from("JR s8"), bytes: 2, func: CPU::op_jr_s8 }),
            (0x0020_u16, Instruction { dissassembly: String::from("JR NZ,s8"), bytes: 2, func: CPU::op_jr_nz_s8 }),
            (0x0028_u16, Instruction { dissassembly: String::from("JR Z,s8"), bytes: 2, func: CPU::op_jr_z_s8 }),
            (0x0030_u16, Instruction { dissassembly: String::from("JR NC,s8"), bytes: 2, func: CPU::op_jr_nc_s8 }),
            (0x0038_u16, Instruction { dissassembly: String::from("JR C,s8"), bytes: 2, func: CPU::op_jr_c_s8 }),
            (0x00CD_u16, Instruction { dissassembly: String::from("CALL a16"), bytes: 3, func: CPU::op_call_a16 }),
            (0x00C4_u16, Instruction { dissassembly: String::from("CALL NZ,a16"), bytes: 3, func: CPU::op_call_nz_a16 }),
            (0x00CC_u16, Instruction { dissassembly: String::from("CALL Z,a16"), bytes: 3, func: CPU::op_call_z_a16 }),
            (0x00D4_u16, Instruction { dissassembly: String::from("CALL NC,a16"), bytes: 3, func: CPU::op_call_nc_a16 }),
            (0x00DC_u16, Instruction { dissassembly: String::from("CALL C,a16"), bytes: 3, func: CPU::op_call_c_a16 }),
            (0x00C0_u16, Instruction { dissassembly: String::from("RET NZ"), bytes: 1, func: CPU::op_ret_nz }),
            (0x00C9_u16, Instruction { dissassembly: String::from("RET"), bytes: 1, func: CPU::op_ret }),
            (0x00C8_u16, Instruction { dissassembly: String::from("RET Z"), bytes: 1, func: CPU::op_ret_z }),
            (0x00D0_u16, Instruction { dissassembly: String::from("RET NC"), bytes: 1, func: CPU::op_ret_nc }),
            (0x00D8_u16, Instruction { dissassembly: String::from("RET C"), bytes: 1, func: CPU::op_ret_c }),
            (0x00D9_u16, Instruction { dissassembly: String::from("RETI"), bytes: 1, func: CPU::op_reti }),

            (0x00F5_u16, Instruction { dissassembly: String::from("PUSH AF"), bytes: 1, func: CPU::op_push_af }),
            (0x00C5_u16, Instruction { dissassembly: String::from("PUSH BC"), bytes: 1, func: CPU::op_push_bc }),
            (0x00D5_u16, Instruction { dissassembly: String::from("PUSH DE"), bytes: 1, func: CPU::op_push_de }),
            (0x00E5_u16, Instruction { dissassembly: String::from("PUSH HL"), bytes: 1, func: CPU::op_push_hl }),
            (0x00F1_u16, Instruction { dissassembly: String::from("POP AF"), bytes: 1, func: CPU::op_pop_af }),
            (0x00C1_u16, Instruction { dissassembly: String::from("POP BC"), bytes: 1, func: CPU::op_pop_bc }),
            (0x00D1_u16, Instruction { dissassembly: String::from("POP DE"), bytes: 1, func: CPU::op_pop_de }),
            (0x00E1_u16, Instruction { dissassembly: String::from("POP HL"), bytes: 1, func: CPU::op_pop_hl }),

            (0x0017_u16, Instruction { dissassembly: String::from("RLA"), bytes: 1, func: CPU::op_rla }),
            (0x001F_u16, Instruction { dissassembly: String::from("RRA"), bytes: 1, func: CPU::op_rra }),
            (0x0007_u16, Instruction { dissassembly: String::from("RLCA"), bytes: 1, func: CPU::op_rlca }),
            (0x000F_u16, Instruction { dissassembly: String::from("RRCA"), bytes: 1, func: CPU::op_rrca }),
            
            (0x00C7_u16, Instruction { dissassembly: String::from("RST 0"), bytes: 1, func: CPU::op_rst_0 }),
            (0x00CF_u16, Instruction { dissassembly: String::from("RST 1"), bytes: 1, func: CPU::op_rst_1 }),
            (0x00D7_u16, Instruction { dissassembly: String::from("RST 2"), bytes: 1, func: CPU::op_rst_2 }),
            (0x00DF_u16, Instruction { dissassembly: String::from("RST 3"), bytes: 1, func: CPU::op_rst_3 }),
            (0x00E7_u16, Instruction { dissassembly: String::from("RST 4"), bytes: 1, func: CPU::op_rst_4 }),
            (0x00EF_u16, Instruction { dissassembly: String::from("RST 5"), bytes: 1, func: CPU::op_rst_5 }),
            (0x00F7_u16, Instruction { dissassembly: String::from("RST 6"), bytes: 1, func: CPU::op_rst_6 }),
            (0x00FF_u16, Instruction { dissassembly: String::from("RST 7"), bytes: 1, func: CPU::op_rst_7 }),

            (0x00F3_u16, Instruction { dissassembly: String::from("DI"), bytes: 1, func: CPU::op_di }),
            (0x00FB_u16, Instruction { dissassembly: String::from("EI"), bytes: 1, func: CPU::op_ei }),
            
            // 16 bit opcodes
            (0xCB07_u16, Instruction { dissassembly: String::from("RLC A"), bytes: 2, func: CPU::op_rlc_a }),
            (0xCB00_u16, Instruction { dissassembly: String::from("RLC B"), bytes: 2, func: CPU::op_rlc_b }),
            (0xCB01_u16, Instruction { dissassembly: String::from("RLC C"), bytes: 2, func: CPU::op_rlc_c }),
            (0xCB02_u16, Instruction { dissassembly: String::from("RLC D"), bytes: 2, func: CPU::op_rlc_d }),
            (0xCB03_u16, Instruction { dissassembly: String::from("RLC E"), bytes: 2, func: CPU::op_rlc_e }),
            (0xCB04_u16, Instruction { dissassembly: String::from("RLC H"), bytes: 2, func: CPU::op_rlc_h }),
            (0xCB05_u16, Instruction { dissassembly: String::from("RLC L"), bytes: 2, func: CPU::op_rlc_l }),
            (0xCB06_u16, Instruction { dissassembly: String::from("RLC (HL)"), bytes: 2, func: CPU::op_rlc_mem_hl }),
            (0xCB0F_u16, Instruction { dissassembly: String::from("RRC A"), bytes: 2, func: CPU::op_rrc_a }),
            (0xCB08_u16, Instruction { dissassembly: String::from("RRC B"), bytes: 2, func: CPU::op_rrc_b }),
            (0xCB09_u16, Instruction { dissassembly: String::from("RRC C"), bytes: 2, func: CPU::op_rrc_c }),
            (0xCB0A_u16, Instruction { dissassembly: String::from("RRC D"), bytes: 2, func: CPU::op_rrc_d }),
            (0xCB0B_u16, Instruction { dissassembly: String::from("RRC E"), bytes: 2, func: CPU::op_rrc_e }),
            (0xCB0C_u16, Instruction { dissassembly: String::from("RRC H"), bytes: 2, func: CPU::op_rrc_h }),
            (0xCB0D_u16, Instruction { dissassembly: String::from("RRC L"), bytes: 2, func: CPU::op_rrc_l }),
            (0xCB0E_u16, Instruction { dissassembly: String::from("RRC (HL)"), bytes: 2, func: CPU::op_rrc_mem_hl }),
            (0xCB17_u16, Instruction { dissassembly: String::from("RL A"), bytes: 2, func: CPU::op_rl_a }),
            (0xCB10_u16, Instruction { dissassembly: String::from("RL B"), bytes: 2, func: CPU::op_rl_b }),
            (0xCB11_u16, Instruction { dissassembly: String::from("RL C"), bytes: 2, func: CPU::op_rl_c }),
            (0xCB12_u16, Instruction { dissassembly: String::from("RL D"), bytes: 2, func: CPU::op_rl_d }),
            (0xCB13_u16, Instruction { dissassembly: String::from("RL E"), bytes: 2, func: CPU::op_rl_e }),
            (0xCB14_u16, Instruction { dissassembly: String::from("RL H"), bytes: 2, func: CPU::op_rl_h }),
            (0xCB15_u16, Instruction { dissassembly: String::from("RL L"), bytes: 2, func: CPU::op_rl_l }),
            (0xCB16_u16, Instruction { dissassembly: String::from("RL (HL)"), bytes: 2, func: CPU::op_rl_mem_hl }),
            (0xCB1F_u16, Instruction { dissassembly: String::from("RR A"), bytes: 2, func: CPU::op_rr_a }),
            (0xCB18_u16, Instruction { dissassembly: String::from("RR B"), bytes: 2, func: CPU::op_rr_b }),
            (0xCB19_u16, Instruction { dissassembly: String::from("RR C"), bytes: 2, func: CPU::op_rr_c }),
            (0xCB1A_u16, Instruction { dissassembly: String::from("RR D"), bytes: 2, func: CPU::op_rr_d }),
            (0xCB1B_u16, Instruction { dissassembly: String::from("RR E"), bytes: 2, func: CPU::op_rr_e }),
            (0xCB1C_u16, Instruction { dissassembly: String::from("RR H"), bytes: 2, func: CPU::op_rr_h }),
            (0xCB1D_u16, Instruction { dissassembly: String::from("RR L"), bytes: 2, func: CPU::op_rr_l }),
            (0xCB1E_u16, Instruction { dissassembly: String::from("RR (HL)"), bytes: 2, func: CPU::op_rr_mem_hl }),
            (0xCB27_u16, Instruction { dissassembly: String::from("SLA A"), bytes: 2, func: CPU::op_sla_a }),
            (0xCB20_u16, Instruction { dissassembly: String::from("SLA B"), bytes: 2, func: CPU::op_sla_b }),
            (0xCB21_u16, Instruction { dissassembly: String::from("SLA C"), bytes: 2, func: CPU::op_sla_c }),
            (0xCB22_u16, Instruction { dissassembly: String::from("SLA D"), bytes: 2, func: CPU::op_sla_d }),
            (0xCB23_u16, Instruction { dissassembly: String::from("SLA E"), bytes: 2, func: CPU::op_sla_e }),
            (0xCB24_u16, Instruction { dissassembly: String::from("SLA H"), bytes: 2, func: CPU::op_sla_h }),
            (0xCB25_u16, Instruction { dissassembly: String::from("SLA L"), bytes: 2, func: CPU::op_sla_l }),
            (0xCB26_u16, Instruction { dissassembly: String::from("SLA (HL)"), bytes: 2, func: CPU::op_sla_mem_hl }),
            (0xCB3F_u16, Instruction { dissassembly: String::from("SRL A"), bytes: 2, func: CPU::op_srl_a}),
            (0xCB38_u16, Instruction { dissassembly: String::from("SRL B"), bytes: 2, func: CPU::op_srl_b }),
            (0xCB39_u16, Instruction { dissassembly: String::from("SRL C"), bytes: 2, func: CPU::op_srl_c }),
            (0xCB3A_u16, Instruction { dissassembly: String::from("SRL D"), bytes: 2, func: CPU::op_srl_d }),
            (0xCB3B_u16, Instruction { dissassembly: String::from("SRL E"), bytes: 2, func: CPU::op_srl_e }),
            (0xCB3C_u16, Instruction { dissassembly: String::from("SRL H"), bytes: 2, func: CPU::op_srl_h }),
            (0xCB3D_u16, Instruction { dissassembly: String::from("SRL L"), bytes: 2, func: CPU::op_srl_l }),
            (0xCB3E_u16, Instruction { dissassembly: String::from("SRL (HL)"), bytes: 2, func: CPU::op_srl_mem_hl }),
            (0xCB2F_u16, Instruction { dissassembly: String::from("SRA A"), bytes: 2, func: CPU::op_sra_a}),
            (0xCB28_u16, Instruction { dissassembly: String::from("SRA B"), bytes: 2, func: CPU::op_sra_b }),
            (0xCB29_u16, Instruction { dissassembly: String::from("SRA C"), bytes: 2, func: CPU::op_sra_c }),
            (0xCB2A_u16, Instruction { dissassembly: String::from("SRA D"), bytes: 2, func: CPU::op_sra_d }),
            (0xCB2B_u16, Instruction { dissassembly: String::from("SRA E"), bytes: 2, func: CPU::op_sra_e }),
            (0xCB2C_u16, Instruction { dissassembly: String::from("SRA H"), bytes: 2, func: CPU::op_sra_h }),
            (0xCB2D_u16, Instruction { dissassembly: String::from("SRA L"), bytes: 2, func: CPU::op_sra_l }),
            (0xCB2E_u16, Instruction { dissassembly: String::from("SRA (HL)"), bytes: 2, func: CPU::op_sra_mem_hl }),
            
            (0xCB37_u16, Instruction { dissassembly: String::from("SWAP A"), bytes: 2, func: CPU::op_swap_a }),
            (0xCB30_u16, Instruction { dissassembly: String::from("SWAP B"), bytes: 2, func: CPU::op_swap_b }),
            (0xCB31_u16, Instruction { dissassembly: String::from("SWAP C"), bytes: 2, func: CPU::op_swap_c }),
            (0xCB32_u16, Instruction { dissassembly: String::from("SWAP D"), bytes: 2, func: CPU::op_swap_d }),
            (0xCB33_u16, Instruction { dissassembly: String::from("SWAP E"), bytes: 2, func: CPU::op_swap_e }),
            (0xCB34_u16, Instruction { dissassembly: String::from("SWAP H"), bytes: 2, func: CPU::op_swap_h }),
            (0xCB35_u16, Instruction { dissassembly: String::from("SWAP L"), bytes: 2, func: CPU::op_swap_l }),
            (0xCB36_u16, Instruction { dissassembly: String::from("SWAP (HL)"), bytes: 2, func: CPU::op_swap_mem_hl }),

            (0xCB47_u16, Instruction { dissassembly: String::from("BIT 0,A"), bytes: 2, func: CPU::op_bit0_a }),
            (0xCB40_u16, Instruction { dissassembly: String::from("BIT 0,B"), bytes: 2, func: CPU::op_bit0_b }),
            (0xCB41_u16, Instruction { dissassembly: String::from("BIT 0,C"), bytes: 2, func: CPU::op_bit0_c }),
            (0xCB42_u16, Instruction { dissassembly: String::from("BIT 0,D"), bytes: 2, func: CPU::op_bit0_d }),
            (0xCB43_u16, Instruction { dissassembly: String::from("BIT 0,E"), bytes: 2, func: CPU::op_bit0_e }),
            (0xCB44_u16, Instruction { dissassembly: String::from("BIT 0,H"), bytes: 2, func: CPU::op_bit0_h }),
            (0xCB45_u16, Instruction { dissassembly: String::from("BIT 0,L"), bytes: 2, func: CPU::op_bit0_l }),
            (0xCB4F_u16, Instruction { dissassembly: String::from("BIT 1,A"), bytes: 2, func: CPU::op_bit1_a }),
            (0xCB48_u16, Instruction { dissassembly: String::from("BIT 1,B"), bytes: 2, func: CPU::op_bit1_b }),
            (0xCB49_u16, Instruction { dissassembly: String::from("BIT 1,C"), bytes: 2, func: CPU::op_bit1_c }),
            (0xCB4A_u16, Instruction { dissassembly: String::from("BIT 1,D"), bytes: 2, func: CPU::op_bit1_d }),
            (0xCB4B_u16, Instruction { dissassembly: String::from("BIT 1,E"), bytes: 2, func: CPU::op_bit1_e }),
            (0xCB4C_u16, Instruction { dissassembly: String::from("BIT 1,H"), bytes: 2, func: CPU::op_bit1_h }),
            (0xCB4D_u16, Instruction { dissassembly: String::from("BIT 1,L"), bytes: 2, func: CPU::op_bit1_l }),
            (0xCB57_u16, Instruction { dissassembly: String::from("BIT 2,A"), bytes: 2, func: CPU::op_bit2_a }),
            (0xCB50_u16, Instruction { dissassembly: String::from("BIT 2,B"), bytes: 2, func: CPU::op_bit2_b }),
            (0xCB51_u16, Instruction { dissassembly: String::from("BIT 2,C"), bytes: 2, func: CPU::op_bit2_c }),
            (0xCB52_u16, Instruction { dissassembly: String::from("BIT 2,D"), bytes: 2, func: CPU::op_bit2_d }),
            (0xCB53_u16, Instruction { dissassembly: String::from("BIT 2,E"), bytes: 2, func: CPU::op_bit2_e }),
            (0xCB54_u16, Instruction { dissassembly: String::from("BIT 2,H"), bytes: 2, func: CPU::op_bit2_h }),
            (0xCB55_u16, Instruction { dissassembly: String::from("BIT 2,L"), bytes: 2, func: CPU::op_bit2_l }),
            (0xCB5F_u16, Instruction { dissassembly: String::from("BIT 3,A"), bytes: 2, func: CPU::op_bit3_a }),
            (0xCB58_u16, Instruction { dissassembly: String::from("BIT 3,B"), bytes: 2, func: CPU::op_bit3_b }),
            (0xCB59_u16, Instruction { dissassembly: String::from("BIT 3,C"), bytes: 2, func: CPU::op_bit3_c }),
            (0xCB5A_u16, Instruction { dissassembly: String::from("BIT 3,D"), bytes: 2, func: CPU::op_bit3_d }),
            (0xCB5B_u16, Instruction { dissassembly: String::from("BIT 3,E"), bytes: 2, func: CPU::op_bit3_e }),
            (0xCB5C_u16, Instruction { dissassembly: String::from("BIT 3,H"), bytes: 2, func: CPU::op_bit3_h }),
            (0xCB5D_u16, Instruction { dissassembly: String::from("BIT 3,L"), bytes: 2, func: CPU::op_bit3_l }),
            (0xCB67_u16, Instruction { dissassembly: String::from("BIT 4,A"), bytes: 2, func: CPU::op_bit4_a }),
            (0xCB60_u16, Instruction { dissassembly: String::from("BIT 4,B"), bytes: 2, func: CPU::op_bit4_b }),
            (0xCB61_u16, Instruction { dissassembly: String::from("BIT 4,C"), bytes: 2, func: CPU::op_bit4_c }),
            (0xCB62_u16, Instruction { dissassembly: String::from("BIT 4,D"), bytes: 2, func: CPU::op_bit4_d }),
            (0xCB63_u16, Instruction { dissassembly: String::from("BIT 4,E"), bytes: 2, func: CPU::op_bit4_e }),
            (0xCB64_u16, Instruction { dissassembly: String::from("BIT 4,H"), bytes: 2, func: CPU::op_bit4_h }),
            (0xCB65_u16, Instruction { dissassembly: String::from("BIT 4,L"), bytes: 2, func: CPU::op_bit4_l }),
            (0xCB6F_u16, Instruction { dissassembly: String::from("BIT 5,A"), bytes: 2, func: CPU::op_bit5_a }),
            (0xCB68_u16, Instruction { dissassembly: String::from("BIT 5,B"), bytes: 2, func: CPU::op_bit5_b }),
            (0xCB69_u16, Instruction { dissassembly: String::from("BIT 5,C"), bytes: 2, func: CPU::op_bit5_c }),
            (0xCB6A_u16, Instruction { dissassembly: String::from("BIT 5,D"), bytes: 2, func: CPU::op_bit5_d }),
            (0xCB6B_u16, Instruction { dissassembly: String::from("BIT 5,E"), bytes: 2, func: CPU::op_bit5_e }),
            (0xCB6C_u16, Instruction { dissassembly: String::from("BIT 5,H"), bytes: 2, func: CPU::op_bit5_h }),
            (0xCB6D_u16, Instruction { dissassembly: String::from("BIT 5,L"), bytes: 2, func: CPU::op_bit5_l }),
            (0xCB77_u16, Instruction { dissassembly: String::from("BIT 6,A"), bytes: 2, func: CPU::op_bit6_a }),
            (0xCB70_u16, Instruction { dissassembly: String::from("BIT 6,B"), bytes: 2, func: CPU::op_bit6_b }),
            (0xCB71_u16, Instruction { dissassembly: String::from("BIT 6,C"), bytes: 2, func: CPU::op_bit6_c }),
            (0xCB72_u16, Instruction { dissassembly: String::from("BIT 6,D"), bytes: 2, func: CPU::op_bit6_d }),
            (0xCB73_u16, Instruction { dissassembly: String::from("BIT 6,E"), bytes: 2, func: CPU::op_bit6_e }),
            (0xCB74_u16, Instruction { dissassembly: String::from("BIT 6,H"), bytes: 2, func: CPU::op_bit6_h }),
            (0xCB75_u16, Instruction { dissassembly: String::from("BIT 6,L"), bytes: 2, func: CPU::op_bit6_l }),
            (0xCB7F_u16, Instruction { dissassembly: String::from("BIT 7,A"), bytes: 2, func: CPU::op_bit7_a }),
            (0xCB78_u16, Instruction { dissassembly: String::from("BIT 7,B"), bytes: 2, func: CPU::op_bit7_b }),
            (0xCB79_u16, Instruction { dissassembly: String::from("BIT 7,C"), bytes: 2, func: CPU::op_bit7_c }),
            (0xCB7A_u16, Instruction { dissassembly: String::from("BIT 7,D"), bytes: 2, func: CPU::op_bit7_d }),
            (0xCB7B_u16, Instruction { dissassembly: String::from("BIT 7,E"), bytes: 2, func: CPU::op_bit7_e }),
            (0xCB7C_u16, Instruction { dissassembly: String::from("BIT 7,H"), bytes: 2, func: CPU::op_bit7_h }),
            (0xCB7D_u16, Instruction { dissassembly: String::from("BIT 7,L"), bytes: 2, func: CPU::op_bit7_l }),
            (0xCB46_u16, Instruction { dissassembly: String::from("BIT 0,(HL)"), bytes: 2, func: CPU::op_bit0_mem_hl }),
            (0xCB4E_u16, Instruction { dissassembly: String::from("BIT 1,(HL)"), bytes: 2, func: CPU::op_bit1_mem_hl }),
            (0xCB56_u16, Instruction { dissassembly: String::from("BIT 2,(HL)"), bytes: 2, func: CPU::op_bit2_mem_hl }),
            (0xCB5E_u16, Instruction { dissassembly: String::from("BIT 3,(HL)"), bytes: 2, func: CPU::op_bit3_mem_hl }),
            (0xCB66_u16, Instruction { dissassembly: String::from("BIT 4,(HL)"), bytes: 2, func: CPU::op_bit4_mem_hl }),
            (0xCB6E_u16, Instruction { dissassembly: String::from("BIT 5,(HL)"), bytes: 2, func: CPU::op_bit5_mem_hl }),
            (0xCB76_u16, Instruction { dissassembly: String::from("BIT 6,(HL)"), bytes: 2, func: CPU::op_bit6_mem_hl }),
            (0xCB7E_u16, Instruction { dissassembly: String::from("BIT 7,(HL)"), bytes: 2, func: CPU::op_bit7_mem_hl }),

            (0xCBC7_u16, Instruction { dissassembly: String::from("SET 0,A"), bytes: 2, func: CPU::op_set0_a }),
            (0xCBC0_u16, Instruction { dissassembly: String::from("SET 0,B"), bytes: 2, func: CPU::op_set0_b }),
            (0xCBC1_u16, Instruction { dissassembly: String::from("SET 0,C"), bytes: 2, func: CPU::op_set0_c }),
            (0xCBC2_u16, Instruction { dissassembly: String::from("SET 0,D"), bytes: 2, func: CPU::op_set0_d }),
            (0xCBC3_u16, Instruction { dissassembly: String::from("SET 0,E"), bytes: 2, func: CPU::op_set0_e }),
            (0xCBC4_u16, Instruction { dissassembly: String::from("SET 0,H"), bytes: 2, func: CPU::op_set0_h }),
            (0xCBC5_u16, Instruction { dissassembly: String::from("SET 0,L"), bytes: 2, func: CPU::op_set0_l }),
            (0xCBCF_u16, Instruction { dissassembly: String::from("SET 1,A"), bytes: 2, func: CPU::op_set1_a }),
            (0xCBC8_u16, Instruction { dissassembly: String::from("SET 1,B"), bytes: 2, func: CPU::op_set1_b }),
            (0xCBC9_u16, Instruction { dissassembly: String::from("SET 1,C"), bytes: 2, func: CPU::op_set1_c }),
            (0xCBCA_u16, Instruction { dissassembly: String::from("SET 1,D"), bytes: 2, func: CPU::op_set1_d }),
            (0xCBCB_u16, Instruction { dissassembly: String::from("SET 1,E"), bytes: 2, func: CPU::op_set1_e }),
            (0xCBCC_u16, Instruction { dissassembly: String::from("SET 1,H"), bytes: 2, func: CPU::op_set1_h }),
            (0xCBCD_u16, Instruction { dissassembly: String::from("SET 1,L"), bytes: 2, func: CPU::op_set1_l }),
            (0xCBD7_u16, Instruction { dissassembly: String::from("SET 2,A"), bytes: 2, func: CPU::op_set2_a }),
            (0xCBD0_u16, Instruction { dissassembly: String::from("SET 2,B"), bytes: 2, func: CPU::op_set2_b }),
            (0xCBD1_u16, Instruction { dissassembly: String::from("SET 2,C"), bytes: 2, func: CPU::op_set2_c }),
            (0xCBD2_u16, Instruction { dissassembly: String::from("SET 2,D"), bytes: 2, func: CPU::op_set2_d }),
            (0xCBD3_u16, Instruction { dissassembly: String::from("SET 2,E"), bytes: 2, func: CPU::op_set2_e }),
            (0xCBD4_u16, Instruction { dissassembly: String::from("SET 2,H"), bytes: 2, func: CPU::op_set2_h }),
            (0xCBD5_u16, Instruction { dissassembly: String::from("SET 2,L"), bytes: 2, func: CPU::op_set2_l }),
            (0xCBDF_u16, Instruction { dissassembly: String::from("SET 3,A"), bytes: 2, func: CPU::op_set3_a }),
            (0xCBD8_u16, Instruction { dissassembly: String::from("SET 3,B"), bytes: 2, func: CPU::op_set3_b }),
            (0xCBD9_u16, Instruction { dissassembly: String::from("SET 3,C"), bytes: 2, func: CPU::op_set3_c }),
            (0xCBDA_u16, Instruction { dissassembly: String::from("SET 3,D"), bytes: 2, func: CPU::op_set3_d }),
            (0xCBDB_u16, Instruction { dissassembly: String::from("SET 3,E"), bytes: 2, func: CPU::op_set3_e }),
            (0xCBDC_u16, Instruction { dissassembly: String::from("SET 3,H"), bytes: 2, func: CPU::op_set3_h }),
            (0xCBDD_u16, Instruction { dissassembly: String::from("SET 3,L"), bytes: 2, func: CPU::op_set3_l }),
            (0xCBE7_u16, Instruction { dissassembly: String::from("SET 4,A"), bytes: 2, func: CPU::op_set4_a }),
            (0xCBE0_u16, Instruction { dissassembly: String::from("SET 4,B"), bytes: 2, func: CPU::op_set4_b }),
            (0xCBE1_u16, Instruction { dissassembly: String::from("SET 4,C"), bytes: 2, func: CPU::op_set4_c }),
            (0xCBE2_u16, Instruction { dissassembly: String::from("SET 4,D"), bytes: 2, func: CPU::op_set4_d }),
            (0xCBE3_u16, Instruction { dissassembly: String::from("SET 4,E"), bytes: 2, func: CPU::op_set4_e }),
            (0xCBE4_u16, Instruction { dissassembly: String::from("SET 4,H"), bytes: 2, func: CPU::op_set4_h }),
            (0xCBE5_u16, Instruction { dissassembly: String::from("SET 4,L"), bytes: 2, func: CPU::op_set4_l }),
            (0xCBEF_u16, Instruction { dissassembly: String::from("SET 5,A"), bytes: 2, func: CPU::op_set5_a }),
            (0xCBE8_u16, Instruction { dissassembly: String::from("SET 5,B"), bytes: 2, func: CPU::op_set5_b }),
            (0xCBE9_u16, Instruction { dissassembly: String::from("SET 5,C"), bytes: 2, func: CPU::op_set5_c }),
            (0xCBEA_u16, Instruction { dissassembly: String::from("SET 5,D"), bytes: 2, func: CPU::op_set5_d }),
            (0xCBEB_u16, Instruction { dissassembly: String::from("SET 5,E"), bytes: 2, func: CPU::op_set5_e }),
            (0xCBEC_u16, Instruction { dissassembly: String::from("SET 5,H"), bytes: 2, func: CPU::op_set5_h }),
            (0xCBED_u16, Instruction { dissassembly: String::from("SET 5,L"), bytes: 2, func: CPU::op_set5_l }),
            (0xCBF7_u16, Instruction { dissassembly: String::from("SET 6,A"), bytes: 2, func: CPU::op_set6_a }),
            (0xCBF0_u16, Instruction { dissassembly: String::from("SET 6,B"), bytes: 2, func: CPU::op_set6_b }),
            (0xCBF1_u16, Instruction { dissassembly: String::from("SET 6,C"), bytes: 2, func: CPU::op_set6_c }),
            (0xCBF2_u16, Instruction { dissassembly: String::from("SET 6,D"), bytes: 2, func: CPU::op_set6_d }),
            (0xCBF3_u16, Instruction { dissassembly: String::from("SET 6,E"), bytes: 2, func: CPU::op_set6_e }),
            (0xCBF4_u16, Instruction { dissassembly: String::from("SET 6,H"), bytes: 2, func: CPU::op_set6_h }),
            (0xCBF5_u16, Instruction { dissassembly: String::from("SET 6,L"), bytes: 2, func: CPU::op_set6_l }),
            (0xCBFF_u16, Instruction { dissassembly: String::from("SET 7,A"), bytes: 2, func: CPU::op_set7_a }),
            (0xCBF8_u16, Instruction { dissassembly: String::from("SET 7,B"), bytes: 2, func: CPU::op_set7_b }),
            (0xCBF9_u16, Instruction { dissassembly: String::from("SET 7,C"), bytes: 2, func: CPU::op_set7_c }),
            (0xCBFA_u16, Instruction { dissassembly: String::from("SET 7,D"), bytes: 2, func: CPU::op_set7_d }),
            (0xCBFB_u16, Instruction { dissassembly: String::from("SET 7,E"), bytes: 2, func: CPU::op_set7_e }),
            (0xCBFC_u16, Instruction { dissassembly: String::from("SET 7,H"), bytes: 2, func: CPU::op_set7_h }),
            (0xCBFD_u16, Instruction { dissassembly: String::from("SET 7,L"), bytes: 2, func: CPU::op_set7_l }),

            (0xCB87_u16, Instruction { dissassembly: String::from("RES 0,A"), bytes: 2, func: CPU::op_res0_a }),
            (0xCB80_u16, Instruction { dissassembly: String::from("RES 0,B"), bytes: 2, func: CPU::op_res0_b }),
            (0xCB81_u16, Instruction { dissassembly: String::from("RES 0,C"), bytes: 2, func: CPU::op_res0_c }),
            (0xCB82_u16, Instruction { dissassembly: String::from("RES 0,D"), bytes: 2, func: CPU::op_res0_d }),
            (0xCB83_u16, Instruction { dissassembly: String::from("RES 0,E"), bytes: 2, func: CPU::op_res0_e }),
            (0xCB84_u16, Instruction { dissassembly: String::from("RES 0,H"), bytes: 2, func: CPU::op_res0_h }),
            (0xCB85_u16, Instruction { dissassembly: String::from("RES 0,L"), bytes: 2, func: CPU::op_res0_l }),
            (0xCB8F_u16, Instruction { dissassembly: String::from("RES 1,A"), bytes: 2, func: CPU::op_res1_a }),
            (0xCB88_u16, Instruction { dissassembly: String::from("RES 1,B"), bytes: 2, func: CPU::op_res1_b }),
            (0xCB89_u16, Instruction { dissassembly: String::from("RES 1,C"), bytes: 2, func: CPU::op_res1_c }),
            (0xCB8A_u16, Instruction { dissassembly: String::from("RES 1,D"), bytes: 2, func: CPU::op_res1_d }),
            (0xCB8B_u16, Instruction { dissassembly: String::from("RES 1,E"), bytes: 2, func: CPU::op_res1_e }),
            (0xCB8C_u16, Instruction { dissassembly: String::from("RES 1,H"), bytes: 2, func: CPU::op_res1_h }),
            (0xCB8D_u16, Instruction { dissassembly: String::from("RES 1,L"), bytes: 2, func: CPU::op_res1_l }),
            (0xCB97_u16, Instruction { dissassembly: String::from("RES 2,A"), bytes: 2, func: CPU::op_res2_a }),
            (0xCB90_u16, Instruction { dissassembly: String::from("RES 2,B"), bytes: 2, func: CPU::op_res2_b }),
            (0xCB91_u16, Instruction { dissassembly: String::from("RES 2,C"), bytes: 2, func: CPU::op_res2_c }),
            (0xCB92_u16, Instruction { dissassembly: String::from("RES 2,D"), bytes: 2, func: CPU::op_res2_d }),
            (0xCB93_u16, Instruction { dissassembly: String::from("RES 2,E"), bytes: 2, func: CPU::op_res2_e }),
            (0xCB94_u16, Instruction { dissassembly: String::from("RES 2,H"), bytes: 2, func: CPU::op_res2_h }),
            (0xCB95_u16, Instruction { dissassembly: String::from("RES 2,L"), bytes: 2, func: CPU::op_res2_l }),
            (0xCB9F_u16, Instruction { dissassembly: String::from("RES 3,A"), bytes: 2, func: CPU::op_res3_a }),
            (0xCB98_u16, Instruction { dissassembly: String::from("RES 3,B"), bytes: 2, func: CPU::op_res3_b }),
            (0xCB99_u16, Instruction { dissassembly: String::from("RES 3,C"), bytes: 2, func: CPU::op_res3_c }),
            (0xCB9A_u16, Instruction { dissassembly: String::from("RES 3,D"), bytes: 2, func: CPU::op_res3_d }),
            (0xCB9B_u16, Instruction { dissassembly: String::from("RES 3,E"), bytes: 2, func: CPU::op_res3_e }),
            (0xCB9C_u16, Instruction { dissassembly: String::from("RES 3,H"), bytes: 2, func: CPU::op_res3_h }),
            (0xCB9D_u16, Instruction { dissassembly: String::from("RES 3,L"), bytes: 2, func: CPU::op_res3_l }),
            (0xCBA7_u16, Instruction { dissassembly: String::from("RES 4,A"), bytes: 2, func: CPU::op_res4_a }),
            (0xCBA0_u16, Instruction { dissassembly: String::from("RES 4,B"), bytes: 2, func: CPU::op_res4_b }),
            (0xCBA1_u16, Instruction { dissassembly: String::from("RES 4,C"), bytes: 2, func: CPU::op_res4_c }),
            (0xCBA2_u16, Instruction { dissassembly: String::from("RES 4,D"), bytes: 2, func: CPU::op_res4_d }),
            (0xCBA3_u16, Instruction { dissassembly: String::from("RES 4,E"), bytes: 2, func: CPU::op_res4_e }),
            (0xCBA4_u16, Instruction { dissassembly: String::from("RES 4,H"), bytes: 2, func: CPU::op_res4_h }),
            (0xCBA5_u16, Instruction { dissassembly: String::from("RES 4,L"), bytes: 2, func: CPU::op_res4_l }),
            (0xCBAF_u16, Instruction { dissassembly: String::from("RES 5,A"), bytes: 2, func: CPU::op_res5_a }),
            (0xCBA8_u16, Instruction { dissassembly: String::from("RES 5,B"), bytes: 2, func: CPU::op_res5_b }),
            (0xCBA9_u16, Instruction { dissassembly: String::from("RES 5,C"), bytes: 2, func: CPU::op_res5_c }),
            (0xCBAA_u16, Instruction { dissassembly: String::from("RES 5,D"), bytes: 2, func: CPU::op_res5_d }),
            (0xCBAB_u16, Instruction { dissassembly: String::from("RES 5,E"), bytes: 2, func: CPU::op_res5_e }),
            (0xCBAC_u16, Instruction { dissassembly: String::from("RES 5,H"), bytes: 2, func: CPU::op_res5_h }),
            (0xCBAD_u16, Instruction { dissassembly: String::from("RES 5,L"), bytes: 2, func: CPU::op_res5_l }),
            (0xCBB7_u16, Instruction { dissassembly: String::from("RES 6,A"), bytes: 2, func: CPU::op_res6_a }),
            (0xCBB0_u16, Instruction { dissassembly: String::from("RES 6,B"), bytes: 2, func: CPU::op_res6_b }),
            (0xCBB1_u16, Instruction { dissassembly: String::from("RES 6,C"), bytes: 2, func: CPU::op_res6_c }),
            (0xCBB2_u16, Instruction { dissassembly: String::from("RES 6,D"), bytes: 2, func: CPU::op_res6_d }),
            (0xCBB3_u16, Instruction { dissassembly: String::from("RES 6,E"), bytes: 2, func: CPU::op_res6_e }),
            (0xCBB4_u16, Instruction { dissassembly: String::from("RES 6,H"), bytes: 2, func: CPU::op_res6_h }),
            (0xCBB5_u16, Instruction { dissassembly: String::from("RES 6,L"), bytes: 2, func: CPU::op_res6_l }),
            (0xCBBF_u16, Instruction { dissassembly: String::from("RES 7,A"), bytes: 2, func: CPU::op_res7_a }),
            (0xCBB8_u16, Instruction { dissassembly: String::from("RES 7,B"), bytes: 2, func: CPU::op_res7_b }),
            (0xCBB9_u16, Instruction { dissassembly: String::from("RES 7,C"), bytes: 2, func: CPU::op_res7_c }),
            (0xCBBA_u16, Instruction { dissassembly: String::from("RES 7,D"), bytes: 2, func: CPU::op_res7_d }),
            (0xCBBB_u16, Instruction { dissassembly: String::from("RES 7,E"), bytes: 2, func: CPU::op_res7_e }),
            (0xCBBC_u16, Instruction { dissassembly: String::from("RES 7,H"), bytes: 2, func: CPU::op_res7_h }),
            (0xCBBD_u16, Instruction { dissassembly: String::from("RES 7,L"), bytes: 2, func: CPU::op_res7_l }),
            (0xCB86_u16, Instruction { dissassembly: String::from("RES 0,(HL)"), bytes: 2, func: CPU::op_res0_mem_hl }),
            (0xCB8E_u16, Instruction { dissassembly: String::from("RES 1,(HL)"), bytes: 2, func: CPU::op_res1_mem_hl }),
            (0xCB96_u16, Instruction { dissassembly: String::from("RES 2,(HL)"), bytes: 2, func: CPU::op_res2_mem_hl }),
            (0xCB9E_u16, Instruction { dissassembly: String::from("RES 3,(HL)"), bytes: 2, func: CPU::op_res3_mem_hl }),
            (0xCBA6_u16, Instruction { dissassembly: String::from("RES 4,(HL)"), bytes: 2, func: CPU::op_res4_mem_hl }),
            (0xCBAE_u16, Instruction { dissassembly: String::from("RES 5,(HL)"), bytes: 2, func: CPU::op_res5_mem_hl }),
            (0xCBB6_u16, Instruction { dissassembly: String::from("RES 6,(HL)"), bytes: 2, func: CPU::op_res6_mem_hl }),
            (0xCBBE_u16, Instruction { dissassembly: String::from("RES 7,(HL"), bytes: 2, func: CPU::op_res7_mem_hl }),

            (0xCBC6_u16, Instruction { dissassembly: String::from("SET 0,(HL)"), bytes: 2, func: CPU::op_set0_mem_hl }),
            (0xCBCE_u16, Instruction { dissassembly: String::from("SET 1,(HL)"), bytes: 2, func: CPU::op_set1_mem_hl }),
            (0xCBD6_u16, Instruction { dissassembly: String::from("SET 2,(HL)"), bytes: 2, func: CPU::op_set2_mem_hl }),
            (0xCBDE_u16, Instruction { dissassembly: String::from("SET 3,(HL)"), bytes: 2, func: CPU::op_set3_mem_hl }),
            (0xCBE6_u16, Instruction { dissassembly: String::from("SET 4,(HL)"), bytes: 2, func: CPU::op_set4_mem_hl }),
            (0xCBEE_u16, Instruction { dissassembly: String::from("SET 5,(HL)"), bytes: 2, func: CPU::op_set5_mem_hl }),
            (0xCBF6_u16, Instruction { dissassembly: String::from("SET 6,(HL)"), bytes: 2, func: CPU::op_set6_mem_hl }),
            (0xCBFE_u16, Instruction { dissassembly: String::from("SET 7,(HL)"), bytes: 2, func: CPU::op_set7_mem_hl })

        ].iter().cloned().collect();

        Self {
            bus,
            instructions: instruction_table,
            registers: Registers { 
                a: 0x01, f: 0x00,
                b: 0xFF, c: 0x13,
                d: 0x00, e: 0xC1,
                h: 0x84, l: 0x03,
                sp: 0xFFFE,
                pc: 0x0000
            },
            state: CPUState::Normal,
            interrupts_enabled: false,
            interrupts_enable_request: false,
            debug: false,
        }
    }

    pub fn set_start_pc(&mut self, pc: u16) {
        self.registers.pc = pc;
    }

    pub fn step(&mut self) -> u8 {
        let pc = self.registers.pc;
        let mut cycles = 0;

        cycles += self.dispatch_interrupts();

        if self.state == CPUState::Normal {
            let op : u16;
            let b1: u8 = self.read_byte_from_pc();
            if b1 != 0xCB {
                op = b1 as u16;
            }
            else {
                let b2: u8 = self.read_byte_from_pc();
                op = (b1 as u16) << 8 | (b2 as u16);
            }

            if !self.instructions.contains_key(&op) {
                panic!("Undefined instruction: @{:#06x} {:#04x}", pc, op);
            }

            let inst : &Instruction = &(self.instructions[&op]);        
            let func = inst.func;
            let dis = inst.dissassembly.clone();

            //  if pc == 0x0200 {
            // self.debug = true;
            //  }

            if self.debug {
                let af = ((self.registers.a as u16) << 8) | (self.registers.f as u16);
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);

                println!("@{:#06X} op: {:#04X} ({}) AF: {:#06X} | BC: {:#06X} | DE: {:#06X} | HL: {:#06X}", pc, op, dis, af, bc, de, hl);
            }
            
            // call the instruction
            cycles += func(self);
        }

        if cycles == 0 { 1 } else { cycles } 
    }

    fn dispatch_interrupts(&mut self) -> u8 {
        let mut cycles = 0;

        let bus = self.bus.borrow_mut();
        let iie = bus.read_byte(0xFFFF);
        let iif = bus.read_byte(0xFF0F);
        drop(bus);

        let masked_interrupts = iie & iif & 0x1F;

        // if halted and an interrupt is triggered, exit halt even if IME=0 (4 cycles)
        if self.state == CPUState::Halt && masked_interrupts != 0 {
            self.state = CPUState::Normal;
            cycles += 4;
        }

        // if IME=1 and IF and IE are enabled, do the interrupt dispatch (20 cycles)
        if self.interrupts_enabled && masked_interrupts != 0 {
            if (1 << Interrupts::VBlank as u8) & masked_interrupts != 0 {
                self.execute_interrupt(Interrupts::VBlank);
            }
            else if(1 << Interrupts::LCDStat as u8) & masked_interrupts != 0 {
                self.execute_interrupt(Interrupts::LCDStat);
            }
            else if(1 << Interrupts::Timer as u8) & masked_interrupts != 0 {
                self.execute_interrupt(Interrupts::Timer);
            }
            else if(1 << Interrupts::Serial as u8) & masked_interrupts != 0 {
                self.execute_interrupt(Interrupts::Serial);
            }
            else if(1 << Interrupts::Joypad as u8) & masked_interrupts != 0 {
                self.execute_interrupt(Interrupts::Joypad);
            }

            cycles += 20;
        }

        // when EI is called, we don't enable interrupts, instead we do this here, after checking
        // and interrupts will be enabled after the next cycle
        if self.interrupts_enable_request {
            self.interrupts_enable_request = false;
            self.interrupts_enabled = true;
        }

        cycles
    }

    fn execute_interrupt(&mut self, interrupt : Interrupts) {
        self.interrupts_enabled = false;

        self.push(self.registers.pc);

        self.registers.pc = INTERRUPT_ADDRESS[interrupt as usize];

        let mut bus = self.bus.borrow_mut();
        let iif = bus.read_byte(0xFF0F) & !(1 << interrupt as u8);
        bus.write_byte(0xFF0F, iif);
    }

    fn read_byte_from_pc(&mut self) -> u8 {
        let b = self.read_memory(self.registers.pc);
        self.registers.pc += 1;
        
        b
    }

    fn read_memory(&self, address: u16) -> u8 {
        self.bus.borrow().read_byte(address)
    }

    fn read_word_from_pc(&mut self) -> u16 {
        self.read_byte_from_pc() as u16 | ((self.read_byte_from_pc() as u16) << 8)
    }

    fn write_memory(&self, address: u16, data: u8) {
        self.bus.borrow_mut().write_byte(address, data);
    }

    fn write_word(&self, address: u16, data: u16) {
        self.write_memory(address, (data & 0xFF) as u8);
        self.write_memory(address + 1, ((data & 0xFF00) >> 8) as u8);
    }

    // INSTRUCTIONS

    fn op_nop(_cpu: &mut CPU) -> u8 {
        1
    }

    fn op_stop(cpu: &mut CPU) -> u8 {
        let iie = cpu.read_memory(0xFFFF);
        // TODO: P10-P13 should be LOW
        if iie == 0 {
            cpu.state = CPUState::Stop;
        }

        1
    }

    fn op_halt(cpu: &mut CPU) -> u8 {
        let iie = cpu.read_memory(0xFFFF);
        let iif = cpu.read_memory(0xFF0F);
        let masked_interrupts = iie & iif & 0x1f;

        if masked_interrupts == 0 {
            cpu.state = CPUState::Halt;
        }

        1
    }

    fn op_inc_a(cpu: &mut CPU) -> u8 {
        inc(&mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_inc_b(cpu: &mut CPU) -> u8  {
        inc(&mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_inc_c(cpu: &mut CPU) -> u8 {
        inc(&mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_inc_d(cpu: &mut CPU) -> u8 {
        inc(&mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_inc_e(cpu: &mut CPU) -> u8 {
        inc(&mut cpu.registers.e, &mut cpu.registers.f)
    }
    
    fn op_inc_h(cpu: &mut CPU) -> u8 {
        inc(&mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_inc_l(cpu: &mut CPU) -> u8 {
        inc(&mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_inc_bc(cpu: &mut CPU) -> u8 {
        let mut bc: u16 = (cpu.registers.b as u16) << 8 | (cpu.registers.c as u16);
        bc = bc.wrapping_add(1);

        cpu.registers.b = (bc >> 8) as u8;
        cpu.registers.c = bc as u8;

        2
    }

    fn op_inc_de(cpu: &mut CPU) -> u8 {
        let mut de: u16 = (cpu.registers.d as u16) << 8 | (cpu.registers.e as u16);
        de = de.wrapping_add(1);

        cpu.registers.d = (de >> 8) as u8;
        cpu.registers.e = de as u8;

        2
    }

    fn op_inc_hl(cpu: &mut CPU) -> u8 {
        let mut hl: u16 = (cpu.registers.h as u16) << 8 | (cpu.registers.l as u16);
        hl = hl.wrapping_add(1);

        cpu.registers.h = (hl >> 8) as u8;
        cpu.registers.l = hl as u8;

        2
    }

    fn op_inc_sp(cpu: &mut CPU) -> u8 {
        cpu.registers.sp = cpu.registers.sp.wrapping_add(1);

        2
    }

    fn op_inc_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl);

        let is_half_carry = is_half_carry(&v, &1);
        let r = v.wrapping_add(1);
        cpu.write_memory(hl, r);

        set_flag2(&mut cpu.registers.f, FLAG_Z, r == 0);
        set_flag2(&mut cpu.registers.f, FLAG_N, false);
        set_flag2(&mut cpu.registers.f, FLAG_H, is_half_carry);

        3
    }

    fn op_dec_a(cpu: &mut CPU) -> u8 {
        dec(&mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_dec_b(cpu: &mut CPU) -> u8 {
        dec(&mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_dec_c(cpu: &mut CPU) -> u8 {
        dec(&mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_dec_d(cpu: &mut CPU) -> u8 {
        dec(&mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_dec_e(cpu: &mut CPU) -> u8 {
        dec(&mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_dec_h(cpu: &mut CPU) -> u8 {
        dec(&mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_dec_l(cpu: &mut CPU) -> u8 {
        dec(&mut cpu.registers.l, &mut cpu.registers.f)
    }    

    fn op_dec_bc(cpu: &mut CPU) -> u8 {
        let mut bc: u16 = (cpu.registers.b as u16) << 8 | (cpu.registers.c as u16);
        bc = bc.wrapping_sub(1);

        cpu.registers.b = (bc >> 8) as u8;
        cpu.registers.c = bc as u8;

        2
    }

    fn op_dec_de(cpu: &mut CPU) -> u8 {
        let mut de: u16 = (cpu.registers.d as u16) << 8 | (cpu.registers.e as u16);
        de = de.wrapping_sub(1);

        cpu.registers.d = (de >> 8) as u8;
        cpu.registers.e = de as u8;

        2
    }

    fn op_dec_hl(cpu: &mut CPU) -> u8 {
        let mut hl: u16 = (cpu.registers.h as u16) << 8 | (cpu.registers.l as u16);
        hl = hl.wrapping_sub(1);

        cpu.registers.h = (hl >> 8) as u8;
        cpu.registers.l = hl as u8;

        2
    }

    fn op_dec_sp(cpu: &mut CPU) -> u8 {
        cpu.registers.sp = cpu.registers.sp.wrapping_sub(1);

        2
    }
    
    fn op_dec_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl).wrapping_sub(1);
        cpu.write_memory(hl, v);

        cpu.set_flag(FLAG_Z, v == 0);
        cpu.set_flag(FLAG_N, true);
        cpu.set_flag(FLAG_H, v & 0x0F == 0x0F);

        3
    }

    fn op_add_a_a(cpu: &mut CPU) -> u8 {
        let a = cpu.registers.a;
        add_reg(&mut cpu.registers.a, a, &mut cpu.registers.f)
    }

    fn op_add_a_b(cpu: &mut CPU) -> u8 {
        add_reg(&mut cpu.registers.a, cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_add_a_c(cpu: &mut CPU) -> u8 {
        add_reg(&mut cpu.registers.a, cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_add_a_d(cpu: &mut CPU) -> u8 {
        add_reg(&mut cpu.registers.a, cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_add_a_e(cpu: &mut CPU) -> u8 {
        add_reg(&mut cpu.registers.a, cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_add_a_h(cpu: &mut CPU) -> u8 {
        add_reg(&mut cpu.registers.a, cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_add_a_l(cpu: &mut CPU) -> u8 {
        add_reg(&mut cpu.registers.a, cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_add_a_d8(cpu: &mut CPU) -> u8 {
        let d8 = cpu.read_byte_from_pc();
        add_reg(&mut cpu.registers.a, d8, &mut cpu.registers.f);

        2
    }

    fn op_add_a_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl);

        add_reg(&mut cpu.registers.a, v, &mut cpu.registers.f);

        2
    }

    fn op_add_hl_bc(cpu: &mut CPU) -> u8 {
        let mut hl = ((cpu.registers.h as u16) << 8) | cpu.registers.l as u16;
        let bc = ((cpu.registers.b as u16) << 8) | cpu.registers.c as u16;
        add_reg16(&mut hl, bc, &mut cpu.registers.f);

        cpu.registers.h = (hl >> 8) as u8;
        cpu.registers.l = hl as u8;

        2
    }

    fn op_add_hl_de(cpu: &mut CPU) -> u8 {
        let mut hl = ((cpu.registers.h as u16) << 8) | cpu.registers.l as u16;
        let de = ((cpu.registers.d as u16) << 8) | cpu.registers.e as u16;
        add_reg16(&mut hl, de, &mut cpu.registers.f);

        cpu.registers.h = (hl >> 8) as u8;
        cpu.registers.l = hl as u8;

        2
    }

    fn op_add_hl_hl(cpu: &mut CPU) -> u8 {
        let mut hl = ((cpu.registers.h as u16) << 8) | cpu.registers.l as u16;
        let hl2 = hl;
        add_reg16(&mut hl, hl2, &mut cpu.registers.f);

        cpu.set_hl(hl);

        2
    }

    fn op_add_hl_sp(cpu: &mut CPU) -> u8 {
        let mut hl = cpu.get_hl();
        add_reg16(&mut hl, cpu.registers.sp, &mut cpu.registers.f);
        cpu.set_hl(hl);

        2
    }

    fn op_add_sp_s8(cpu: &mut CPU) -> u8 {
        let s8: i8 = cpu.read_byte_from_pc() as i8;
        
        let is_half_carry = is_half_carry(&(cpu.registers.sp as u8), &(s8 as u8));
        let is_full_carry = is_full_carry(&(cpu.registers.sp as u8), &(s8 as u8));

        cpu.registers.sp = (cpu.registers.sp as i32).wrapping_add(s8 as i32) as u16;

        set_flag2(&mut cpu.registers.f, FLAG_Z, false);
        set_flag2(&mut cpu.registers.f, FLAG_N, false);
        set_flag2(&mut cpu.registers.f, FLAG_H, is_half_carry);
        set_flag2(&mut cpu.registers.f, FLAG_C, is_full_carry);

        4
    }

    fn op_sub_a(cpu: &mut CPU) -> u8 {
        let a = cpu.registers.a;
        sub(&mut cpu.registers.a, a, &mut cpu.registers.f)
    }

    fn op_sub_b(cpu: &mut CPU) -> u8 {
        sub(&mut cpu.registers.a, cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_sub_c(cpu: &mut CPU) -> u8 {
        sub(&mut cpu.registers.a, cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_sub_d(cpu: &mut CPU) -> u8 {
        sub(&mut cpu.registers.a, cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_sub_e(cpu: &mut CPU) -> u8 {
        sub(&mut cpu.registers.a, cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_sub_h(cpu: &mut CPU) -> u8 {
        sub(&mut cpu.registers.a, cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_sub_l(cpu: &mut CPU) -> u8 {
        sub(&mut cpu.registers.a, cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_sub_d8(cpu: &mut CPU) -> u8 {
        let d8 = cpu.read_byte_from_pc();
        sub(&mut cpu.registers.a, d8, &mut cpu.registers.f) + 1
    }

    fn op_sub_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let v = cpu.read_memory(hl);
        sub(&mut cpu.registers.a, v, &mut cpu.registers.f) + 1
    }

    fn op_adc_a_a(cpu: &mut CPU) -> u8 {
        let a = cpu.registers.a;
        adc_reg(&mut cpu.registers.a, a, &mut cpu.registers.f)
    }

    fn op_adc_a_b(cpu: &mut CPU) -> u8 {
        adc_reg(&mut cpu.registers.a, cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_adc_a_c(cpu: &mut CPU) -> u8 {
        adc_reg(&mut cpu.registers.a, cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_adc_a_d(cpu: &mut CPU) -> u8 {
        adc_reg(&mut cpu.registers.a, cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_adc_a_e(cpu: &mut CPU) -> u8 {
        adc_reg(&mut cpu.registers.a, cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_adc_a_h(cpu: &mut CPU) -> u8 {
        adc_reg(&mut cpu.registers.a, cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_adc_a_l(cpu: &mut CPU) -> u8 {
        adc_reg(&mut cpu.registers.a, cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_adc_a_d8(cpu: &mut CPU) -> u8 {
        let d8 = cpu.read_byte_from_pc();
        adc_reg(&mut cpu.registers.a, d8, &mut cpu.registers.f) + 1
    }

    fn op_adc_a_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let v = cpu.read_memory(hl);
        adc_reg(&mut cpu.registers.a, v, &mut cpu.registers.f) + 1
    }

    fn op_sbc_a_a(cpu: &mut CPU) -> u8 {
        let a = cpu.registers.a;
        sbc_reg(&mut cpu.registers.a, a, &mut cpu.registers.f)
    }

    fn op_sbc_a_b(cpu: &mut CPU) -> u8 {
        sbc_reg(&mut cpu.registers.a, cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_sbc_a_c(cpu: &mut CPU) -> u8 {
        sbc_reg(&mut cpu.registers.a, cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_sbc_a_d(cpu: &mut CPU) -> u8 {
        sbc_reg(&mut cpu.registers.a, cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_sbc_a_e(cpu: &mut CPU) -> u8 {
        sbc_reg(&mut cpu.registers.a, cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_sbc_a_h(cpu: &mut CPU) -> u8 {
        sbc_reg(&mut cpu.registers.a, cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_sbc_a_l(cpu: &mut CPU) -> u8 {
        sbc_reg(&mut cpu.registers.a, cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_sbc_a_d8(cpu: &mut CPU) -> u8 {
        let d8 = cpu.read_byte_from_pc();
        sbc_reg(&mut cpu.registers.a, d8, &mut cpu.registers.f) + 1
    }

    fn op_sbc_a_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl);
        sbc_reg(&mut cpu.registers.a, v, &mut cpu.registers.f) + 1
    }

    fn op_daa(cpu: &mut CPU) -> u8 {
        // https://forums.nesdev.com/viewtopic.php?t=15944
        // note: assumes a is a uint8_t and wraps from 0xff to 0
        let c = cpu.get_flag(FLAG_C);
        let h = cpu.get_flag(FLAG_H);

        if !cpu.get_flag(FLAG_N) {  // after an addition, adjust if (half-)carry occurred or if result is out of bounds
            if c || cpu.registers.a > 0x99 { 
                cpu.registers.a = cpu.registers.a.wrapping_add(0x60);
                cpu.set_flag(FLAG_C, true);
            }
            if h || (cpu.registers.a & 0x0f) > 0x09 {
                cpu.registers.a = cpu.registers.a.wrapping_add(0x6);
            }
        } else {  // after a subtraction, only adjust if (half-)carry occurred
            if c { 
                cpu.registers.a = cpu.registers.a.wrapping_sub(0x60);
            }
            if h { 
                cpu.registers.a = cpu.registers.a.wrapping_sub(0x6);
            }
        }
        
        cpu.set_flag(FLAG_Z, cpu.registers.a == 0);
        cpu.set_flag(FLAG_H, false);

        1
    }

    fn op_scf(cpu: &mut CPU) -> u8 {
        set_flag2(&mut cpu.registers.f, FLAG_N, false);
        set_flag2(&mut cpu.registers.f, FLAG_H, false);
        set_flag2(&mut cpu.registers.f, FLAG_C, true);

        1
    }

    fn op_ccf(cpu: &mut CPU) -> u8 {
        let cy = get_flag2(&cpu.registers.f, FLAG_C);

        set_flag2(&mut cpu.registers.f, FLAG_N, false);
        set_flag2(&mut cpu.registers.f, FLAG_H, false);
        set_flag2(&mut cpu.registers.f, FLAG_C, !cy);

        1
    }

    fn op_cp_a(cpu: &mut CPU) -> u8 {
        cp_reg(cpu.registers.a, cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_cp_b(cpu: &mut CPU) -> u8 {
        cp_reg(cpu.registers.a, cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_cp_c(cpu: &mut CPU) -> u8 {
        cp_reg(cpu.registers.a, cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_cp_d(cpu: &mut CPU) -> u8 {
        cp_reg(cpu.registers.a, cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_cp_e(cpu: &mut CPU) -> u8 {
        cp_reg(cpu.registers.a, cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_cp_h(cpu: &mut CPU) -> u8 {
        cp_reg(cpu.registers.a, cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_cp_l(cpu: &mut CPU) -> u8 {
        cp_reg(cpu.registers.a, cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_cp_d8(cpu: &mut CPU) -> u8 {
        let d8 = cpu.read_byte_from_pc();
        cp_reg(cpu.registers.a, d8, &mut cpu.registers.f) + 1
    }
    
    fn op_cp_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let v = cpu.read_memory(hl);

        let r = cpu.registers.a.wrapping_sub(v);

        cpu.set_flag(FLAG_Z, r == 0);
        cpu.set_flag(FLAG_N, true);

        let hc = (cpu.registers.a as i8 & 0xF) - (v as i8 & 0xF);
        cpu.set_flag(FLAG_H, hc < 0);
        cpu.set_flag(FLAG_C, cpu.registers.a < v);

        2
    }

    fn op_ld_a_a(_cpu: &mut CPU) -> u8 {
        1
    }

    fn op_ld_a_b(cpu: &mut CPU) -> u8 {
        cpu.registers.a = cpu.registers.b;

        1
    }

    fn op_ld_a_c(cpu: &mut CPU) -> u8 {
        cpu.registers.a = cpu.registers.c;

        1
    }

    fn op_ld_a_d(cpu: &mut CPU) -> u8 {
        cpu.registers.a = cpu.registers.d;

        1
    }

    fn op_ld_a_e(cpu: &mut CPU) -> u8 {
        cpu.registers.a = cpu.registers.e;

        1
    }

    fn op_ld_a_h(cpu: &mut CPU) -> u8 {
        cpu.registers.a = cpu.registers.h;

        1
    }

    fn op_ld_a_l(cpu: &mut CPU) -> u8 {
        cpu.registers.a = cpu.registers.l;

        1
    }

    fn op_ld_b_a(cpu: &mut CPU) -> u8 {
        cpu.registers.b = cpu.registers.a;

        1
    }

    fn op_ld_b_b(_cpu: &mut CPU) -> u8 {
        1
    }

    fn op_ld_b_c(cpu: &mut CPU) -> u8 {
        cpu.registers.b = cpu.registers.c;

        1
    }

    fn op_ld_b_d(cpu: &mut CPU) -> u8 {
        cpu.registers.b = cpu.registers.d;

        1
    }

    fn op_ld_b_e(cpu: &mut CPU) -> u8 {
        cpu.registers.b = cpu.registers.e;

        1
    }

    fn op_ld_b_h(cpu: &mut CPU) -> u8 {
        cpu.registers.b = cpu.registers.h;

        1
    }

    fn op_ld_b_l(cpu: &mut CPU) -> u8 {
        cpu.registers.b = cpu.registers.l;

        1
    }

    fn op_ld_c_a(cpu: &mut CPU) -> u8 {
        cpu.registers.c = cpu.registers.a;

        1
    }

    fn op_ld_c_b(cpu: &mut CPU) -> u8 {
        cpu.registers.c = cpu.registers.b;

        1
    }

    fn op_ld_c_c(_cpu: &mut CPU) -> u8 {
        1
    }

    fn op_ld_c_d(cpu: &mut CPU) -> u8 {
        cpu.registers.c = cpu.registers.d;

        1
    }

    fn op_ld_c_e(cpu: &mut CPU) -> u8 {
        cpu.registers.c = cpu.registers.e;

        1
    }

    fn op_ld_c_h(cpu: &mut CPU) -> u8 {
        cpu.registers.c = cpu.registers.h;

        1
    }

    fn op_ld_c_l(cpu: &mut CPU) -> u8 {
        cpu.registers.c = cpu.registers.l;

        1
    }

    fn op_ld_d_a(cpu: &mut CPU) -> u8 {
        cpu.registers.d = cpu.registers.a;

        1
    }

    fn op_ld_d_b(cpu: &mut CPU) -> u8 {
        cpu.registers.d = cpu.registers.b;

        1
    }

    fn op_ld_d_c(cpu: &mut CPU) -> u8 {
        cpu.registers.d = cpu.registers.c;

        1
    }

    fn op_ld_d_d(_cpu: &mut CPU) -> u8 {
        1
    }

    fn op_ld_d_e(cpu: &mut CPU) -> u8 {
        cpu.registers.d = cpu.registers.e;

        1
    }

    fn op_ld_d_h(cpu: &mut CPU) -> u8 {
        cpu.registers.d = cpu.registers.h;

        1
    }
    
    fn op_ld_d_l(cpu: &mut CPU) -> u8 {
        cpu.registers.d = cpu.registers.l;

        1
    }

    fn op_ld_e_a(cpu: &mut CPU) -> u8 {
        cpu.registers.e = cpu.registers.a;

        1
    }

    fn op_ld_e_b(cpu: &mut CPU) -> u8 {
        cpu.registers.e = cpu.registers.b;

        1
    }

    fn op_ld_e_c(cpu: &mut CPU) -> u8 {
        cpu.registers.e = cpu.registers.c;

        1
    }

    fn op_ld_e_d(cpu: &mut CPU) -> u8 {
        cpu.registers.e = cpu.registers.d;

        1
    }

    fn op_ld_e_e(_cpu: &mut CPU) -> u8 {
        1
    }

    fn op_ld_e_h(cpu: &mut CPU) -> u8 {
        cpu.registers.e = cpu.registers.h;

        1
    }
    
    fn op_ld_e_l(cpu: &mut CPU) -> u8 {
        cpu.registers.e = cpu.registers.l;

        1
    }

    fn op_ld_h_a(cpu: &mut CPU) -> u8 {
        cpu.registers.h = cpu.registers.a;

        1
    }

    fn op_ld_h_b(cpu: &mut CPU) -> u8 {
        cpu.registers.h = cpu.registers.b;

        1
    }

    fn op_ld_h_c(cpu: &mut CPU) -> u8 {
        cpu.registers.h = cpu.registers.c;

        1
    }

    fn op_ld_h_d(cpu: &mut CPU) -> u8 {
        cpu.registers.h = cpu.registers.d;

        1
    }

    fn op_ld_h_e(cpu: &mut CPU) -> u8 {
        cpu.registers.h = cpu.registers.e;

        1
    }

    fn op_ld_h_h(_cpu: &mut CPU) -> u8 {
        1
    }

    fn op_ld_h_l(cpu: &mut CPU) -> u8 {
        cpu.registers.h = cpu.registers.l;

        1
    }

    fn op_ld_h_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let m8 = cpu.read_memory(hl);
        cpu.registers.h = m8;

        2
    }

    fn op_ld_l_a(cpu: &mut CPU) -> u8 {
        cpu.registers.l = cpu.registers.a;

        1
    }

    fn op_ld_l_b(cpu: &mut CPU) -> u8 {
        cpu.registers.l = cpu.registers.b;

        1
    }

    fn op_ld_l_c(cpu: &mut CPU) -> u8 {
        cpu.registers.l = cpu.registers.c;

        1
    }

    fn op_ld_l_d(cpu: &mut CPU) -> u8 {
        cpu.registers.l = cpu.registers.d;

        1
    }

    fn op_ld_l_e(cpu: &mut CPU) -> u8 {
        cpu.registers.l = cpu.registers.e;

        1
    }

    fn op_ld_l_h(cpu: &mut CPU) -> u8 {
        cpu.registers.l = cpu.registers.h;

        1
    }

    fn op_ld_l_l(_cpu: &mut CPU) -> u8 {
        1
    }
    
    fn op_ld_l_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl);
        cpu.registers.l = v;

        2
    }

    fn op_ld_a_d8(cpu: &mut CPU) -> u8 {
        cpu.registers.a = cpu.read_byte_from_pc();

        2
    }

    fn op_ld_b_d8(cpu: &mut CPU) -> u8 {
        cpu.registers.b = cpu.read_byte_from_pc();

        2
    }    

    fn op_ld_c_d8(cpu: &mut CPU) -> u8 {
        cpu.registers.c = cpu.read_byte_from_pc();

        2
    }

    fn op_ld_d_d8(cpu: &mut CPU) -> u8 {
        cpu.registers.d = cpu.read_byte_from_pc();

        2
    }

    fn op_ld_e_d8(cpu: &mut CPU) -> u8 {
        cpu.registers.e = cpu.read_byte_from_pc();

        2
    }

    fn op_ld_l_d8(cpu: &mut CPU) -> u8 {
        cpu.registers.l = cpu.read_byte_from_pc();

        2
    }

    fn op_ld_h_d8(cpu: &mut CPU) -> u8 {
        cpu.registers.h = cpu.read_byte_from_pc();

        2
    }

    fn op_ld_bc_d16(cpu: &mut CPU) -> u8 {
        cpu.registers.c = cpu.read_byte_from_pc();
        cpu.registers.b = cpu.read_byte_from_pc();

        3
    }

    fn op_ld_de_d16(cpu: &mut CPU) -> u8 {
        cpu.registers.e = cpu.read_byte_from_pc();
        cpu.registers.d = cpu.read_byte_from_pc();

        3
    }

    fn op_ld_hl_d16(cpu: &mut CPU) -> u8 {
        cpu.registers.l = cpu.read_byte_from_pc();
        cpu.registers.h = cpu.read_byte_from_pc();

        3
    }

    fn op_ld_sp_d16(cpu: &mut CPU) -> u8 {
        cpu.registers.sp = cpu.read_word_from_pc();

        3
    }

    fn op_ld_sp_hl(cpu: &mut CPU) -> u8 {
        cpu.registers.sp = cpu.get_hl();

        2
    }

    fn op_ld_hl_sp_add_s8(cpu: &mut CPU) -> u8 {
        let imm8 = cpu.read_byte_from_pc() as i8;
        
        let v = imm8 as u8;
        let lb = cpu.registers.sp as u8;

        let is_half_carry = is_half_carry(&lb, &v);
        let is_full_carry = is_full_carry(&lb, &v);

        let hl = (cpu.registers.sp as i32).wrapping_add(imm8 as i32) as u16;
        cpu.set_hl(hl);

        set_flag2(&mut cpu.registers.f, FLAG_Z, false);
        set_flag2(&mut cpu.registers.f, FLAG_N, false);
        set_flag2(&mut cpu.registers.f, FLAG_H, is_half_carry);
        set_flag2(&mut cpu.registers.f, FLAG_C, is_full_carry);

        3
    }

    fn op_ld_a_mem_c(cpu: &mut CPU) -> u8 {
        cpu.registers.a = cpu.read_memory(0xFF00 | cpu.registers.c as u16);

        2
    }

    fn op_ld_a_mem_bc(cpu: &mut CPU) -> u8 {
        let bc = ((cpu.registers.b as u16) << 8) | (cpu.registers.c as u16); 
        cpu.registers.a = cpu.read_memory(bc);

        2
    }

    fn op_ld_a_mem_de(cpu: &mut CPU) -> u8 {
        let de = ((cpu.registers.d as u16) << 8) | (cpu.registers.e as u16);
        cpu.registers.a = cpu.read_memory(de);

        2
    }

    fn op_ld_a_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        cpu.registers.a = cpu.read_memory(hl);

        2
    }

    fn op_ld_b_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        cpu.registers.b = cpu.read_memory(hl);

        2
    }

    fn op_ld_c_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        cpu.registers.c = cpu.read_memory(hl);

        2
    }

    fn op_ld_d_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        cpu.registers.d = cpu.read_memory(hl);

        2
    }

    fn op_ld_e_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        cpu.registers.e = cpu.read_memory(hl);

        2
    }
    
    fn op_ld_a_mem_d8(cpu: &mut CPU) -> u8 {
        let a8 = cpu.read_byte_from_pc();
        cpu.registers.a = cpu.read_memory(0xFF00 | (a8 as u16));

        3
    }

    fn op_ld_a_mem_a16(cpu: &mut CPU) -> u8 {
        let a16 = cpu.read_word_from_pc();
        cpu.registers.a = cpu.read_memory(a16);

        4
    }

    fn op_ld_a_mem_hl_inc(cpu: &mut CPU) -> u8 {
        let hl: u16 = (cpu.registers.h as u16) << 8 | (cpu.registers.l as u16);
        cpu.registers.a = cpu.read_memory(hl);

        let d_hl = hl.wrapping_add(1);
        cpu.registers.h = (d_hl >> 8) as u8;
        cpu.registers.l = d_hl as u8;

        2
    }

    fn op_ld_a_mem_hl_dec(cpu: &mut CPU) -> u8 {
        let hl: u16 = (cpu.registers.h as u16) << 8 | (cpu.registers.l as u16);
        cpu.registers.a = cpu.read_memory(hl);

        let d_hl = hl.wrapping_sub(1);
        cpu.registers.h = (d_hl >> 8) as u8;
        cpu.registers.l = d_hl as u8;

        2
    }

    fn op_ld_mem_c_a(cpu: &mut CPU) -> u8 {
        let address = 0xFF00 | (cpu.registers.c as u16);
        cpu.write_memory(address, cpu.registers.a);

        2
    }

    fn op_ld_mem_bc_a(cpu: &mut CPU) -> u8 {
        let address = (cpu.registers.b as u16) << 8 | cpu.registers.c as u16;
        cpu.write_memory(address, cpu.registers.a);

        2
    }

    fn op_ld_mem_de_a(cpu: &mut CPU) -> u8 {
        let address = (cpu.registers.d as u16) << 8 | cpu.registers.e as u16;
        cpu.write_memory(address, cpu.registers.a);

        2
    }

    fn op_ld_mem_hl_a(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        cpu.write_memory(hl, cpu.registers.a);

        2
    }

    fn op_ld_mem_hl_b(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        cpu.write_memory(hl, cpu.registers.b);

        2
    }

    fn op_ld_mem_hl_c(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        cpu.write_memory(hl, cpu.registers.c);

        2
    }

    fn op_ld_mem_hl_d(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        cpu.write_memory(hl, cpu.registers.d);

        2
    }

    fn op_ld_mem_hl_e(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        cpu.write_memory(hl, cpu.registers.e);

        2
    }

    fn op_ld_mem_hl_h(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        cpu.write_memory(hl, cpu.registers.h);

        2
    }

    fn op_ld_mem_hl_l(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        cpu.write_memory(hl, cpu.registers.l);

        2
    }

    fn op_ld_mem_hl_dec_a(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        cpu.write_memory(hl, cpu.registers.a);

        let d_hl = hl.wrapping_sub(1);
        cpu.registers.h = (d_hl >> 8) as u8;
        cpu.registers.l = d_hl as u8;

        2
    }

    fn op_ld_mem_hl_inc_a(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        cpu.write_memory(hl, cpu.registers.a);

        let d_hl = hl + 1;
        cpu.registers.h = (d_hl >> 8) as u8;
        cpu.registers.l = d_hl as u8;

        2
    }

    fn op_ld_mem_hl_d8(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let d8 = cpu.read_byte_from_pc();
        cpu.write_memory(hl, d8);

        3
    }

    fn op_ld_mem_a8_a(cpu: &mut CPU) -> u8 {
        let address: u16 = 0xFF00 | (cpu.read_byte_from_pc() as u16);
        cpu.write_memory(address, cpu.registers.a);

        3
    }

    fn op_ld_mem_a16_a(cpu: &mut CPU) -> u8 {
        let a16 = cpu.read_word_from_pc();
        cpu.write_memory(a16, cpu.registers.a);

        4
    }

    fn op_ld_mem_a16_sp(cpu: &mut CPU) -> u8 {
        let a16 = cpu.read_word_from_pc();
        cpu.write_word(a16, cpu.registers.sp);

        5
    }

    fn op_and_a(cpu: &mut CPU) -> u8 {
        let a = cpu.registers.a;
        and(&mut cpu.registers.a, a, &mut cpu.registers.f)
    }

    fn op_and_b(cpu: &mut CPU) -> u8 {
        and(&mut cpu.registers.a, cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_and_c(cpu: &mut CPU) -> u8 {
        and(&mut cpu.registers.a, cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_and_d(cpu: &mut CPU) -> u8 {
        and(&mut cpu.registers.a, cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_and_e(cpu: &mut CPU) -> u8 {
        and(&mut cpu.registers.a, cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_and_h(cpu: &mut CPU) -> u8 {
        and(&mut cpu.registers.a, cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_and_l(cpu: &mut CPU) -> u8 {
        and(&mut cpu.registers.a, cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_and_d8(cpu: &mut CPU) -> u8 {
        let d8 = cpu.read_byte_from_pc();
        and(&mut cpu.registers.a, d8, &mut cpu.registers.f) + 1
    }

    fn op_and_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl);
        and(&mut cpu.registers.a, v, &mut cpu.registers.f) + 1
    }

    fn op_or_a(cpu: &mut CPU) -> u8 {
        let a = cpu.registers.a;
        or(&mut cpu.registers.a, a, &mut cpu.registers.f)
    }

    fn op_or_b(cpu: &mut CPU) -> u8 {
        or(&mut cpu.registers.a, cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_or_c(cpu: &mut CPU) -> u8 {
        or(&mut cpu.registers.a, cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_or_d(cpu: &mut CPU) -> u8 {
        or(&mut cpu.registers.a, cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_or_e(cpu: &mut CPU) -> u8 {
        or(&mut cpu.registers.a, cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_or_h(cpu: &mut CPU) -> u8 {
        or(&mut cpu.registers.a, cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_or_l(cpu: &mut CPU) -> u8 {
        or(&mut cpu.registers.a, cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_or_d8(cpu: &mut CPU) -> u8 {
        let d8 = cpu.read_byte_from_pc();
        or(&mut cpu.registers.a, d8, &mut cpu.registers.f) + 1
    }

    fn op_or_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl);
        or(&mut cpu.registers.a, v, &mut cpu.registers.f) + 1
    }

    fn op_xor_a(cpu: &mut CPU) -> u8 {
        xor(cpu.registers.a, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_xor_b(cpu: &mut CPU) -> u8 {
        xor(cpu.registers.b, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_xor_c(cpu: &mut CPU) -> u8 {
        xor(cpu.registers.c, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_xor_d(cpu: &mut CPU) -> u8 {
        xor(cpu.registers.d, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_xor_e(cpu: &mut CPU) -> u8 {
        xor(cpu.registers.e, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_xor_h(cpu: &mut CPU) -> u8 {
        xor(cpu.registers.h, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_xor_l(cpu: &mut CPU) -> u8 {
        xor(cpu.registers.l, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_xor_d8(cpu: &mut CPU) -> u8 {
        let d8 = cpu.read_byte_from_pc();
        xor(d8, &mut cpu.registers.a, &mut cpu.registers.f) + 1
    }

    fn op_xor_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl);

        xor(v, &mut cpu.registers.a, &mut cpu.registers.f) + 1
    }

    fn op_cpl(cpu: &mut CPU) -> u8 {
        cpu.registers.a = !cpu.registers.a;

        cpu.set_flag(FLAG_N, true);
        cpu.set_flag(FLAG_H, true);

        1
    }

    fn op_jp_hl(cpu: &mut CPU) -> u8 {
        cpu.registers.pc = cpu.get_hl();

        1
    }

    fn op_jp_a16(cpu: &mut CPU) -> u8 {
        let a16 = cpu.read_word_from_pc();
        cpu.registers.pc = a16;

        4
    }

    fn op_jp_nz_a16(cpu: &mut CPU) -> u8 {
        let a16 = cpu.read_word_from_pc();

        if !cpu.get_flag(FLAG_Z) {
            cpu.registers.pc = a16;

            4
        }
        else {
            3
        }
    }

    fn op_jp_z_a16(cpu: &mut CPU) -> u8 {
        let a16 = cpu.read_word_from_pc();

        if cpu.get_flag(FLAG_Z) {
            cpu.registers.pc = a16;

            4
        }
        else {
            3
        }
    }

    fn op_jp_nc_a16(cpu: &mut CPU) -> u8 {
        let a16 = cpu.read_word_from_pc();

        if !cpu.get_flag(FLAG_C) {
            cpu.registers.pc = a16;

            4
        }
        else {
            3
        }
    }

    fn op_jp_c_a16(cpu: &mut CPU) -> u8 {
        let a16 = cpu.read_word_from_pc();

        if cpu.get_flag(FLAG_C) {
            cpu.registers.pc = a16;

            4
        }
        else {
            3
        }
    }

    fn op_jr_s8(cpu: &mut CPU) -> u8 {
        let offset = cpu.read_byte_from_pc() as i8;
        cpu.registers.pc = (cpu.registers.pc as i32 + offset as i32) as u16;

        3
    }

    fn op_jr_nz_s8(cpu: &mut CPU) -> u8 {
        let offset = cpu.read_byte_from_pc() as i8;

        if !cpu.get_flag(FLAG_Z) {
            cpu.registers.pc = (cpu.registers.pc as i32 + offset as i32) as u16;

            3
        }
        else {
            2
        }
    }

    fn op_jr_z_s8(cpu: &mut CPU) -> u8 {
        let offset = cpu.read_byte_from_pc() as i8;

        if cpu.get_flag(FLAG_Z) {
            cpu.registers.pc = (cpu.registers.pc as i32 + offset as i32) as u16;

            3
        }
        else {
            2
        }
    }

    fn op_jr_nc_s8(cpu: &mut CPU) -> u8 {
        let offset = cpu.read_byte_from_pc() as i8;

        if !cpu.get_flag(FLAG_C) {
            cpu.registers.pc = (cpu.registers.pc as i32 + offset as i32) as u16;

            3
        }
        else {
            2
        }
    }

    fn op_jr_c_s8(cpu: &mut CPU) -> u8 {
        let offset = cpu.read_byte_from_pc() as i8;

        if cpu.get_flag(FLAG_C) {
            cpu.registers.pc = (cpu.registers.pc as i32 + offset as i32) as u16;

            3
        }
        else {
            2
        }
    }

    fn op_call_a16(cpu: &mut CPU) -> u8 {
        let a16 = cpu.read_word_from_pc();

        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, ((cpu.registers.pc & 0xFF00) >> 8) as u8);
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, (cpu.registers.pc & 0x00FF) as u8);

        cpu.registers.pc = a16;

        6
    }

    fn op_call_nz_a16(cpu: &mut CPU) -> u8 {
        let a16 = cpu.read_word_from_pc();

        if !cpu.get_flag(FLAG_Z) {
            cpu.registers.sp -= 1;
            cpu.write_memory(cpu.registers.sp, ((cpu.registers.pc & 0xFF00) >> 8) as u8);
            cpu.registers.sp -= 1;
            cpu.write_memory(cpu.registers.sp, (cpu.registers.pc & 0x00FF) as u8);
            
            cpu.registers.pc = a16;

            6
        }
        else {
            3
        }
    }

    fn op_call_z_a16(cpu: &mut CPU) -> u8 {
        let a16 = cpu.read_word_from_pc();

        if cpu.get_flag(FLAG_Z) {
            cpu.registers.sp -= 1;
            cpu.write_memory(cpu.registers.sp, ((cpu.registers.pc & 0xFF00) >> 8) as u8);
            cpu.registers.sp -= 1;
            cpu.write_memory(cpu.registers.sp, (cpu.registers.pc & 0x00FF) as u8);
            
            cpu.registers.pc = a16;

            6
        }
        else {
            3
        }
    }

    fn op_call_nc_a16(cpu: &mut CPU) -> u8 {
        let a16 = cpu.read_word_from_pc();

        if !cpu.get_flag(FLAG_C) {
            cpu.registers.sp -= 1;
            cpu.write_memory(cpu.registers.sp, ((cpu.registers.pc & 0xFF00) >> 8) as u8);
            cpu.registers.sp -= 1;
            cpu.write_memory(cpu.registers.sp, (cpu.registers.pc & 0x00FF) as u8);
            
            cpu.registers.pc = a16;

            6
        }
        else {
            3
        }
    }

    fn op_call_c_a16(cpu: &mut CPU) -> u8 {
        let a16 = cpu.read_word_from_pc();

        if cpu.get_flag(FLAG_C) {
            cpu.registers.sp -= 1;
            cpu.write_memory(cpu.registers.sp, ((cpu.registers.pc & 0xFF00) >> 8) as u8);
            cpu.registers.sp -= 1;
            cpu.write_memory(cpu.registers.sp, (cpu.registers.pc & 0x00FF) as u8);
            
            cpu.registers.pc = a16;

            6
        }
        else {
            3
        }
    }

    fn op_ret_nz(cpu: &mut CPU) -> u8 {
        if !cpu.get_flag(FLAG_Z) {
            let l = cpu.read_memory(cpu.registers.sp) as u16;
            cpu.registers.sp += 1;
            let h = cpu.read_memory(cpu.registers.sp) as u16;
            cpu.registers.sp += 1;

            cpu.registers.pc = h << 8 | l;

            5
        }
        else {
            2
        }
    }

    fn op_ret(cpu: &mut CPU) -> u8 {
        let mut pc: u16;

        pc = cpu.read_memory(cpu.registers.sp) as u16;
        cpu.registers.sp += 1;
        pc |= (cpu.read_memory(cpu.registers.sp) as u16) << 8;
        cpu.registers.sp += 1;

        cpu.registers.pc = pc;

        4
    }

    fn op_ret_z(cpu: &mut CPU) -> u8 {
        if cpu.get_flag(FLAG_Z) {
            let l = cpu.read_memory(cpu.registers.sp) as u16;
            cpu.registers.sp += 1;
            let h = cpu.read_memory(cpu.registers.sp) as u16;
            cpu.registers.sp += 1;

            cpu.registers.pc = h << 8 | l;

            5
        }
        else {
            2
        }
    }

    fn op_ret_nc(cpu: &mut CPU) -> u8 {
        if !cpu.get_flag(FLAG_C) {
            let l = cpu.read_memory(cpu.registers.sp) as u16;
            cpu.registers.sp += 1;
            let h = cpu.read_memory(cpu.registers.sp) as u16;
            cpu.registers.sp += 1;

            cpu.registers.pc = h << 8 | l;

            5
        }
        else {
            2
        }
    }

    fn op_ret_c(cpu: &mut CPU) -> u8 {
        if cpu.get_flag(FLAG_C) {
            let l = cpu.read_memory(cpu.registers.sp) as u16;
            cpu.registers.sp += 1;
            let h = cpu.read_memory(cpu.registers.sp) as u16;
            cpu.registers.sp += 1;

            cpu.registers.pc = h << 8 | l;

            5
        }
        else {
            2
        }
    }

    fn op_reti(cpu: &mut CPU) -> u8 {
        let mut pc: u16;

        pc = cpu.read_memory(cpu.registers.sp) as u16;
        cpu.registers.sp += 1;
        pc |= (cpu.read_memory(cpu.registers.sp) as u16) << 8;
        cpu.registers.sp += 1;

        cpu.registers.pc = pc;

        cpu.interrupts_enabled = true;

        4
    }

    fn op_push_af(cpu: &mut CPU) -> u8 {
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.a);
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.f);

        4
    }

    fn op_push_bc(cpu: &mut CPU) -> u8 {
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.b);
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.c);

        4
    }

    fn op_push_de(cpu: &mut CPU) -> u8 {
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.d);
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.e);

        4
    }

    fn op_push_hl(cpu: &mut CPU) -> u8 {
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.h);
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.l);

        4
    }

    fn op_pop_af(cpu: &mut CPU) -> u8 {
        cpu.registers.f = cpu.read_memory(cpu.registers.sp);
        cpu.registers.sp += 1;
        cpu.registers.a = cpu.read_memory(cpu.registers.sp);
        cpu.registers.sp += 1;

        // only the higher 4 bits are used for flags
        cpu.registers.f &= 0xF0; 

        3
    }

    fn op_pop_bc(cpu: &mut CPU) -> u8 {
        cpu.registers.c = cpu.read_memory(cpu.registers.sp);
        cpu.registers.sp += 1;
        cpu.registers.b = cpu.read_memory(cpu.registers.sp);
        cpu.registers.sp += 1;

        3
    }
    
    fn op_pop_de(cpu: &mut CPU) -> u8 {
        cpu.registers.e = cpu.read_memory(cpu.registers.sp);
        cpu.registers.sp += 1;
        cpu.registers.d = cpu.read_memory(cpu.registers.sp);
        cpu.registers.sp += 1;

        3
    }

    fn op_pop_hl(cpu: &mut CPU) -> u8 {
        cpu.registers.l = cpu.read_memory(cpu.registers.sp);
        cpu.registers.sp += 1;
        cpu.registers.h = cpu.read_memory(cpu.registers.sp);
        cpu.registers.sp += 1;

        3
    }

    fn op_rla(cpu: &mut CPU) -> u8 {
        let prev_carry: u8 = cpu.get_flag(FLAG_C) as u8;
        
        let carry = cpu.registers.a & (1 << 7);
        cpu.registers.a = (cpu.registers.a << 1) | prev_carry;

        cpu.set_flag(FLAG_Z, false);
        cpu.set_flag(FLAG_N, false);
        cpu.set_flag(FLAG_H, false);
        cpu.set_flag(FLAG_C, carry != 0);

        1
    }

    fn op_rra(cpu: &mut CPU) -> u8 {
        let prev_carry: u8 = cpu.get_flag(FLAG_C) as u8;
        
        let carry = cpu.registers.a & 0x1;
        cpu.registers.a = (cpu.registers.a >> 1) | (prev_carry << 7);

        cpu.set_flag(FLAG_Z, false);
        cpu.set_flag(FLAG_N, false);
        cpu.set_flag(FLAG_H, false);
        cpu.set_flag(FLAG_C, carry != 0);

        1
    }

    fn op_rlca(cpu: &mut CPU) -> u8 {
        let carry = cpu.registers.a & (1 << 7);
        cpu.registers.a = (cpu.registers.a << 1) | (carry >> 7);
    
        set_flag2(&mut cpu.registers.f, FLAG_Z, false);
        set_flag2(&mut cpu.registers.f, FLAG_N, false);
        set_flag2(&mut cpu.registers.f, FLAG_H, false);
        set_flag2(&mut cpu.registers.f, FLAG_C, carry != 0);
    
        1
    }

    fn op_rrca(cpu: &mut CPU) -> u8 {
        let carry = cpu.registers.a & 0x1;
        cpu.registers.a = (cpu.registers. a >> 1) | (carry << 7);
    
        set_flag2(&mut cpu.registers.f, FLAG_Z, false);
        set_flag2(&mut cpu.registers.f, FLAG_N, false);
        set_flag2(&mut cpu.registers.f, FLAG_H, false);
        set_flag2(&mut cpu.registers.f, FLAG_C, carry != 0);
    
        1
    }

    fn op_rst_0(cpu: &mut CPU) -> u8 {
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, (cpu.registers.pc >> 8) as u8);
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.pc as u8);

        cpu.registers.pc = 0x0000;

        4
    }

    fn op_rst_1(cpu: &mut CPU) -> u8 {
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, (cpu.registers.pc >> 8) as u8);
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.pc as u8);

        cpu.registers.pc = 0x0008;

        4
    }

    fn op_rst_2(cpu: &mut CPU) -> u8 {
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, (cpu.registers.pc >> 8) as u8);
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.pc as u8);

        cpu.registers.pc = 0x0010;

        4
    }

    fn op_rst_3(cpu: &mut CPU) -> u8 {
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, (cpu.registers.pc >> 8) as u8);
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.pc as u8);

        cpu.registers.pc = 0x0018;

        4
    }

    fn op_rst_4(cpu: &mut CPU) -> u8 {
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, (cpu.registers.pc >> 8) as u8);
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.pc as u8);

        cpu.registers.pc = 0x0020;

        4
    }

    fn op_rst_5(cpu: &mut CPU) -> u8 {
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, (cpu.registers.pc >> 8) as u8);
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.pc as u8);

        cpu.registers.pc = 0x0028;

        4
    }

    fn op_rst_6(cpu: &mut CPU) -> u8 {
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, (cpu.registers.pc >> 8) as u8);
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.pc as u8);

        cpu.registers.pc = 0x0030;

        4
    }

    fn op_rst_7(cpu: &mut CPU) -> u8 {
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, (cpu.registers.pc >> 8) as u8);
        cpu.registers.sp -= 1;
        cpu.write_memory(cpu.registers.sp, cpu.registers.pc as u8);

        cpu.registers.pc = 0x0038;

        4
    }

    fn op_rlc_a(cpu: &mut CPU) -> u8 {
        rlc(&mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_rlc_b(cpu: &mut CPU) -> u8 {
        rlc(&mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_rlc_c(cpu: &mut CPU) -> u8 {
        rlc(&mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_rlc_d(cpu: &mut CPU) -> u8 {
        rlc(&mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_rlc_e(cpu: &mut CPU) -> u8 {
        rlc(&mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_rlc_h(cpu: &mut CPU) -> u8 {
        rlc(&mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_rlc_l(cpu: &mut CPU) -> u8 {
        rlc(&mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_rlc_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let mut v = cpu.read_memory(hl);
        rlc(&mut v, &mut cpu.registers.f);
        cpu.write_memory(hl, v);

        4
    }

    fn op_rrc_a(cpu: &mut CPU) -> u8 {
        rrc(&mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_rrc_b(cpu: &mut CPU) -> u8 {
        rrc(&mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_rrc_c(cpu: &mut CPU) -> u8 {
        rrc(&mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_rrc_d(cpu: &mut CPU) -> u8 {
        rrc(&mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_rrc_e(cpu: &mut CPU) -> u8 {
        rrc(&mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_rrc_h(cpu: &mut CPU) -> u8 {
        rrc(&mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_rrc_l(cpu: &mut CPU) -> u8 {
        rrc(&mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_rrc_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let mut v = cpu.read_memory(hl);
        rrc(&mut v, &mut cpu.registers.f);
        cpu.write_memory(hl, v);

        4
    }

    fn op_rl_a(cpu: &mut CPU) -> u8 {
        rl(&mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_rl_b(cpu: &mut CPU) -> u8 {
        rl(&mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_rl_c(cpu: &mut CPU) -> u8 {
        rl(&mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_rl_d(cpu: &mut CPU) -> u8 {
        rl(&mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_rl_e(cpu: &mut CPU) -> u8 {
        rl(&mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_rl_h(cpu: &mut CPU) -> u8 {
        rl(&mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_rl_l(cpu: &mut CPU) -> u8 {
        rl(&mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_rl_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let mut v = cpu.read_memory(hl);
        rl(&mut v, &mut cpu.registers.f);
        cpu.write_memory(hl, v);

        4
    }

    fn op_rr_a(cpu: &mut CPU) -> u8 {
        rr(&mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_rr_b(cpu: &mut CPU) -> u8 {
        rr(&mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_rr_c(cpu: &mut CPU) -> u8 {
        rr(&mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_rr_d(cpu: &mut CPU) -> u8 {
        rr(&mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_rr_e(cpu: &mut CPU) -> u8 {
        rr(&mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_rr_h(cpu: &mut CPU) -> u8 {
        rr(&mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_rr_l(cpu: &mut CPU) -> u8 {
        rr(&mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_rr_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let mut v = cpu.read_memory(hl);
        rr(&mut v, &mut cpu.registers.f);
        cpu.write_memory(hl, v);

        4
    }

    fn op_sla_a(cpu: &mut CPU) -> u8 {
        sla_reg(&mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_sla_b(cpu: &mut CPU) -> u8 {
        sla_reg(&mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_sla_c(cpu: &mut CPU) -> u8 {
        sla_reg(&mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_sla_d(cpu: &mut CPU) -> u8 {
        sla_reg(&mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_sla_e(cpu: &mut CPU) -> u8 {
        sla_reg(&mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_sla_h(cpu: &mut CPU) -> u8 {
        sla_reg(&mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_sla_l(cpu: &mut CPU) -> u8 {
        sla_reg(&mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_sla_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let mut v = cpu.read_memory(hl);
        sla_reg(&mut v, &mut cpu.registers.f);
        cpu.write_memory(hl, v);

        4
    }

    fn op_srl_a(cpu: &mut CPU) -> u8 {
        srl_reg(&mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_srl_b(cpu: &mut CPU) -> u8 {
        srl_reg(&mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_srl_c(cpu: &mut CPU) -> u8 {
        srl_reg(&mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_srl_d(cpu: &mut CPU) -> u8 {
        srl_reg(&mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_srl_e(cpu: &mut CPU) -> u8 {
        srl_reg(&mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_srl_h(cpu: &mut CPU) -> u8 {
        srl_reg(&mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_srl_l(cpu: &mut CPU) -> u8 {
        srl_reg(&mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_srl_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let mut v = cpu.read_memory(hl);
        srl_reg(&mut v, &mut cpu.registers.f);
        cpu.write_memory(hl, v);

        4
    }

    fn op_sra_a(cpu: &mut CPU) -> u8 {
        sra_reg(&mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_sra_b(cpu: &mut CPU) -> u8 {
        sra_reg(&mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_sra_c(cpu: &mut CPU) -> u8 {
        sra_reg(&mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_sra_d(cpu: &mut CPU) -> u8 {
        sra_reg(&mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_sra_e(cpu: &mut CPU) -> u8 {
        sra_reg(&mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_sra_h(cpu: &mut CPU) -> u8 {
        sra_reg(&mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_sra_l(cpu: &mut CPU) -> u8 {
        sra_reg(&mut cpu.registers.l, &mut cpu.registers.f)
    }
    
    fn op_sra_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let mut v = cpu.read_memory(hl);
        sra_reg(&mut v, &mut cpu.registers.f);
        cpu.write_memory(hl, v);

        4
    }

    fn op_di(cpu: &mut CPU) -> u8 {
        cpu.interrupts_enabled = false;
        cpu.interrupts_enable_request = false;

        1
    }

    fn op_ei(cpu: &mut CPU) -> u8 {
        cpu.interrupts_enable_request = true;

        1
    }

    fn op_swap_a(cpu: &mut CPU) -> u8 {
        swap_reg(&mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_swap_b(cpu: &mut CPU) -> u8 {
        swap_reg(&mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_swap_c(cpu: &mut CPU) -> u8 {
        swap_reg(&mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_swap_d(cpu: &mut CPU) -> u8 {
        swap_reg(&mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_swap_e(cpu: &mut CPU) -> u8 {
        swap_reg(&mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_swap_h(cpu: &mut CPU) -> u8 {
        swap_reg(&mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_swap_l(cpu: &mut CPU) -> u8 {
        swap_reg(&mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_swap_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let mut v = cpu.read_memory(hl);
        swap_reg(&mut v, &mut cpu.registers.f);
        cpu.write_memory(hl, v);

        4
    }

    fn op_bit0_a(cpu: &mut CPU) -> u8 {
        bit_n_reg(0, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_bit0_b(cpu: &mut CPU) -> u8 {
        bit_n_reg(0, &mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_bit0_c(cpu: &mut CPU) -> u8 {
        bit_n_reg(0, &mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_bit0_d(cpu: &mut CPU) -> u8 {
        bit_n_reg(0, &mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_bit0_e(cpu: &mut CPU) -> u8 {
        bit_n_reg(0, &mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_bit0_h(cpu: &mut CPU) -> u8 {
        bit_n_reg(0, &mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_bit0_l(cpu: &mut CPU) -> u8 {
        bit_n_reg(0, &mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_bit1_a(cpu: &mut CPU) -> u8 {
        bit_n_reg(1, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_bit1_b(cpu: &mut CPU) -> u8 {
        bit_n_reg(1, &mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_bit1_c(cpu: &mut CPU) -> u8 {
        bit_n_reg(1, &mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_bit1_d(cpu: &mut CPU) -> u8 {
        bit_n_reg(1, &mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_bit1_e(cpu: &mut CPU) -> u8 {
        bit_n_reg(1, &mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_bit1_h(cpu: &mut CPU) -> u8 {
        bit_n_reg(1, &mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_bit1_l(cpu: &mut CPU) -> u8 {
        bit_n_reg(1, &mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_bit2_a(cpu: &mut CPU) -> u8 {
        bit_n_reg(2, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_bit2_b(cpu: &mut CPU) -> u8 {
        bit_n_reg(2, &mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_bit2_c(cpu: &mut CPU) -> u8 {
        bit_n_reg(2, &mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_bit2_d(cpu: &mut CPU) -> u8 {
        bit_n_reg(2, &mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_bit2_e(cpu: &mut CPU) -> u8 {
        bit_n_reg(2, &mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_bit2_h(cpu: &mut CPU) -> u8 {
        bit_n_reg(2, &mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_bit2_l(cpu: &mut CPU) -> u8 {
        bit_n_reg(2, &mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_bit3_a(cpu: &mut CPU) -> u8 {
        bit_n_reg(3, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_bit3_b(cpu: &mut CPU) -> u8 {
        bit_n_reg(3, &mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_bit3_c(cpu: &mut CPU) -> u8 {
        bit_n_reg(3, &mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_bit3_d(cpu: &mut CPU) -> u8 {
        bit_n_reg(3, &mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_bit3_e(cpu: &mut CPU) -> u8 {
        bit_n_reg(3, &mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_bit3_h(cpu: &mut CPU) -> u8 {
        bit_n_reg(3, &mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_bit3_l(cpu: &mut CPU) -> u8 {
        bit_n_reg(3, &mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_bit4_a(cpu: &mut CPU) -> u8 {
        bit_n_reg(4, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_bit4_b(cpu: &mut CPU) -> u8 {
        bit_n_reg(4, &mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_bit4_c(cpu: &mut CPU) -> u8 {
        bit_n_reg(4, &mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_bit4_d(cpu: &mut CPU) -> u8 {
        bit_n_reg(4, &mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_bit4_e(cpu: &mut CPU) -> u8 {
        bit_n_reg(4, &mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_bit4_h(cpu: &mut CPU) -> u8 {
        bit_n_reg(4, &mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_bit4_l(cpu: &mut CPU) -> u8 {
        bit_n_reg(4, &mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_bit5_a(cpu: &mut CPU) -> u8 {
        bit_n_reg(5, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_bit5_b(cpu: &mut CPU) -> u8 {
        bit_n_reg(5, &mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_bit5_c(cpu: &mut CPU) -> u8 {
        bit_n_reg(5, &mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_bit5_d(cpu: &mut CPU) -> u8 {
        bit_n_reg(5, &mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_bit5_e(cpu: &mut CPU) -> u8 {
        bit_n_reg(5, &mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_bit5_h(cpu: &mut CPU) -> u8 {
        bit_n_reg(5, &mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_bit5_l(cpu: &mut CPU) -> u8 {
        bit_n_reg(5, &mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_bit6_a(cpu: &mut CPU) -> u8 {
        bit_n_reg(6, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_bit6_b(cpu: &mut CPU) -> u8 {
        bit_n_reg(6, &mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_bit6_c(cpu: &mut CPU) -> u8 {
        bit_n_reg(6, &mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_bit6_d(cpu: &mut CPU) -> u8 {
        bit_n_reg(6, &mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_bit6_e(cpu: &mut CPU) -> u8 {
        bit_n_reg(6, &mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_bit6_h(cpu: &mut CPU) -> u8 {
        bit_n_reg(6, &mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_bit6_l(cpu: &mut CPU) -> u8 {
        bit_n_reg(6, &mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_bit7_a(cpu: &mut CPU) -> u8 {
        bit_n_reg(7, &mut cpu.registers.a, &mut cpu.registers.f)
    }

    fn op_bit7_b(cpu: &mut CPU) -> u8 {
        bit_n_reg(7, &mut cpu.registers.b, &mut cpu.registers.f)
    }

    fn op_bit7_c(cpu: &mut CPU) -> u8 {
        bit_n_reg(7, &mut cpu.registers.c, &mut cpu.registers.f)
    }

    fn op_bit7_d(cpu: &mut CPU) -> u8 {
        bit_n_reg(7, &mut cpu.registers.d, &mut cpu.registers.f)
    }

    fn op_bit7_e(cpu: &mut CPU) -> u8 {
        bit_n_reg(7, &mut cpu.registers.e, &mut cpu.registers.f)
    }

    fn op_bit7_h(cpu: &mut CPU) -> u8 {
        bit_n_reg(7, &mut cpu.registers.h, &mut cpu.registers.f)
    }

    fn op_bit7_l(cpu: &mut CPU) -> u8 {
        bit_n_reg(7, &mut cpu.registers.l, &mut cpu.registers.f)
    }

    fn op_bit0_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let b = cpu.read_memory(hl) & 1;

        cpu.set_flag(FLAG_Z, b == 0);
        cpu.set_flag(FLAG_N, false);
        cpu.set_flag(FLAG_H, true);

        3
    }

    fn op_bit1_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let b = (cpu.read_memory(hl) >> 1) & 1;

        cpu.set_flag(FLAG_Z, b == 0);
        cpu.set_flag(FLAG_N, false);
        cpu.set_flag(FLAG_H, true);

        3
    }

    fn op_bit2_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let b = (cpu.read_memory(hl) >> 2) & 1;

        cpu.set_flag(FLAG_Z, b == 0);
        cpu.set_flag(FLAG_N, false);
        cpu.set_flag(FLAG_H, true);

        3
    }

    fn op_bit3_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let b = (cpu.read_memory(hl) >> 3) & 1;

        cpu.set_flag(FLAG_Z, b == 0);
        cpu.set_flag(FLAG_N, false);
        cpu.set_flag(FLAG_H, true);

        3
    }

    fn op_bit4_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let b = (cpu.read_memory(hl) >> 4) & 1;

        cpu.set_flag(FLAG_Z, b == 0);
        cpu.set_flag(FLAG_N, false);
        cpu.set_flag(FLAG_H, true);

        3
    }

    fn op_bit5_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let b = (cpu.read_memory(hl) >> 5) & 1;

        cpu.set_flag(FLAG_Z, b == 0);
        cpu.set_flag(FLAG_N, false);
        cpu.set_flag(FLAG_H, true);

        3
    }

    fn op_bit6_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let b = (cpu.read_memory(hl) >> 6) & 1;

        cpu.set_flag(FLAG_Z, b == 0);
        cpu.set_flag(FLAG_N, false);
        cpu.set_flag(FLAG_H, true);

        3
    }

    fn op_bit7_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let b = (cpu.read_memory(hl) >> 7) & 1;

        cpu.set_flag(FLAG_Z, b == 0);
        cpu.set_flag(FLAG_N, false);
        cpu.set_flag(FLAG_H, true);

        3
    }

    fn op_set0_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a |= 1;

        2
    }

    fn op_set0_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b |= 1;
        
        2
    }

    fn op_set0_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c |= 1;
        
        2
    }

    fn op_set0_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d |= 1;
        
        2
    }

    fn op_set0_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e |= 1;
        
        2
    }

    fn op_set0_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h |= 1;
        
        2
    }

    fn op_set0_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l |= 1;
        
        2
    }

    fn op_set1_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a |= 1 << 1;

        2
    }

    fn op_set1_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b |= 1 << 1;
        
        2
    }

    fn op_set1_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c |= 1 << 1;
        
        2
    }

    fn op_set1_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d |= 1 << 1;
        
        2
    }

    fn op_set1_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e |= 1 << 1;
        
        2
    }

    fn op_set1_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h |= 1 << 1;
        
        2
    }

    fn op_set1_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l |= 1 << 1;
        
        2
    }

    fn op_set2_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a |= 1 << 2;

        2
    }

    fn op_set2_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b |= 1 << 2;
        
        2
    }

    fn op_set2_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c |= 1 << 2;
        
        2
    }

    fn op_set2_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d |= 1 << 2;
        
        2
    }

    fn op_set2_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e |= 1 << 2;
        
        2
    }

    fn op_set2_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h |= 1 << 2;
        
        2
    }

    fn op_set2_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l |= 1 << 2;
        
        2
    }

    fn op_set3_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a |= 1 << 3;

        2
    }

    fn op_set3_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b |= 1 << 3;
        
        2
    }

    fn op_set3_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c |= 1 << 3;
        
        2
    }

    fn op_set3_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d |= 1 << 3;
        
        2
    }

    fn op_set3_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e |= 1 << 3;
        
        2
    }

    fn op_set3_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h |= 1 << 3;
        
        2
    }

    fn op_set3_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l |= 1 << 3;
        
        2
    }

    fn op_set4_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a |= 1 << 4;

        2
    }

    fn op_set4_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b |= 1 << 4;
        
        2
    }

    fn op_set4_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c |= 1 << 4;
        
        2
    }

    fn op_set4_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d |= 1 << 4;
        
        2
    }

    fn op_set4_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e |= 1 << 4;
        
        2
    }

    fn op_set4_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h |= 1 << 4;
        
        2
    }

    fn op_set4_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l |= 1 << 4;
        
        2
    }

    fn op_set5_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a |= 1 << 5;

        2
    }

    fn op_set5_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b |= 1 << 5;
        
        2
    }

    fn op_set5_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c |= 1 << 5;
        
        2
    }

    fn op_set5_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d |= 1 << 5;
        
        2
    }

    fn op_set5_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e |= 1 << 5;
        
        2
    }

    fn op_set5_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h |= 1 << 5;
        
        2
    }

    fn op_set5_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l |= 1 << 5;
        
        2
    }

    fn op_set6_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a |= 1 << 6;

        2
    }

    fn op_set6_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b |= 1 << 6;
        
        2
    }

    fn op_set6_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c |= 1 << 6;
        
        2
    }

    fn op_set6_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d |= 1 << 6;
        
        2
    }

    fn op_set6_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e |= 1 << 6;
        
        2
    }

    fn op_set6_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h |= 1 << 6;
        
        2
    }

    fn op_set6_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l |= 1 << 6;
        
        2
    }

    fn op_set7_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a |= 1 << 7;

        2
    }

    fn op_set7_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b |= 1 << 7;
        
        2
    }

    fn op_set7_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c |= 1 << 7;
        
        2
    }

    fn op_set7_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d |= 1 << 7;
        
        2
    }

    fn op_set7_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e |= 1 << 7;
        
        2
    }

    fn op_set7_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h |= 1 << 7;
        
        2
    }

    fn op_set7_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l |= 1 << 7;
        
        2
    }

    fn op_res0_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a &= !(1);

        2
    }

    fn op_res0_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b &= !(1);

        2
    }

    fn op_res0_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c &= !(1);

        2
    }

    fn op_res0_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d &= !(1);

        2
    }

    fn op_res0_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e &= !(1);

        2
    }

    fn op_res0_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h &= !(1);

        2
    }

    fn op_res0_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l &= !(1);

        2
    }

    fn op_res1_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a &= !(1 << 1);

        2
    }

    fn op_res1_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b &= !(1 << 1);

        2
    }

    fn op_res1_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c &= !(1 << 1);

        2
    }

    fn op_res1_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d &= !(1 << 1);

        2
    }

    fn op_res1_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e &= !(1 << 1);

        2
    }

    fn op_res1_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h &= !(1 << 1);

        2
    }

    fn op_res1_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l &= !(1 << 1);

        2
    }

    fn op_res2_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a &= !(1 << 2);

        2
    }

    fn op_res2_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b &= !(1 << 2);

        2
    }

    fn op_res2_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c &= !(1 << 2);

        2
    }

    fn op_res2_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d &= !(1 << 2);

        2
    }

    fn op_res2_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e &= !(1 << 2);

        2
    }

    fn op_res2_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h &= !(1 << 2);

        2
    }

    fn op_res2_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l &= !(1 << 2);

        2
    }

    fn op_res3_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a &= !(1 << 3);

        2
    }

    fn op_res3_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b &= !(1 << 3);

        2
    }

    fn op_res3_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c &= !(1 << 3);

        2
    }

    fn op_res3_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d &= !(1 << 3);

        2
    }

    fn op_res3_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e &= !(1 << 3);

        2
    }

    fn op_res3_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h &= !(1 << 3);

        2
    }

    fn op_res3_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l &= !(1 << 3);

        2
    }

    fn op_res4_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a &= !(1 << 4);

        2
    }

    fn op_res4_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b &= !(1 << 4);

        2
    }

    fn op_res4_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c &= !(1 << 4);

        2
    }

    fn op_res4_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d &= !(1 << 4);

        2
    }

    fn op_res4_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e &= !(1 << 4);

        2
    }

    fn op_res4_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h &= !(1 << 4);

        2
    }

    fn op_res4_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l &= !(1 << 4);

        2
    }

    fn op_res5_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a &= !(1 << 5);

        2
    }

    fn op_res5_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b &= !(1 << 5);

        2
    }

    fn op_res5_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c &= !(1 << 5);

        2
    }

    fn op_res5_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d &= !(1 << 5);

        2
    }

    fn op_res5_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e &= !(1 << 5);

        2
    }

    fn op_res5_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h &= !(1 << 5);

        2
    }

    fn op_res5_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l &= !(1 << 5);

        2
    }

    fn op_res6_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a &= !(1 << 6);

        2
    }

    fn op_res6_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b &= !(1 << 6);

        2
    }

    fn op_res6_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c &= !(1 << 6);

        2
    }

    fn op_res6_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d &= !(1 << 6);

        2
    }

    fn op_res6_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e &= !(1 << 6);

        2
    }

    fn op_res6_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h &= !(1 << 6);

        2
    }

    fn op_res6_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l &= !(1 << 6);

        2
    }

    fn op_res7_a(cpu: &mut CPU) -> u8 {
        cpu.registers.a &= !(1 << 7);

        2
    }

    fn op_res7_b(cpu: &mut CPU) -> u8 {
        cpu.registers.b &= !(1 << 7);

        2
    }

    fn op_res7_c(cpu: &mut CPU) -> u8 {
        cpu.registers.c &= !(1 << 7);

        2
    }

    fn op_res7_d(cpu: &mut CPU) -> u8 {
        cpu.registers.d &= !(1 << 7);

        2
    }

    fn op_res7_e(cpu: &mut CPU) -> u8 {
        cpu.registers.e &= !(1 << 7);

        2
    }

    fn op_res7_h(cpu: &mut CPU) -> u8 {
        cpu.registers.h &= !(1 << 7);

        2
    }

    fn op_res7_l(cpu: &mut CPU) -> u8 {
        cpu.registers.l &= !(1 << 7);

        2
    }

    fn op_res0_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let v = cpu.read_memory(hl) & !(1);
        cpu.write_memory(hl, v);

        4
    }

    fn op_res1_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let v = cpu.read_memory(hl) & !(1 << 1);
        cpu.write_memory(hl, v);

        4
    }

    fn op_res2_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let v = cpu.read_memory(hl) & !(1 << 2);
        cpu.write_memory(hl, v);

        4
    }

    fn op_res3_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let v = cpu.read_memory(hl) & !(1 << 3);
        cpu.write_memory(hl, v);

        4
    }

    fn op_res4_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let v = cpu.read_memory(hl) & !(1 << 4);
        cpu.write_memory(hl, v);

        4
    }

    fn op_res5_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let v = cpu.read_memory(hl) & !(1 << 5);
        cpu.write_memory(hl, v);

        4
    }

    fn op_res6_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let v = cpu.read_memory(hl) & !(1 << 6);
        cpu.write_memory(hl, v);

        4
    }

    fn op_res7_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = ((cpu.registers.h as u16) << 8) | (cpu.registers.l as u16);
        let v = cpu.read_memory(hl) & !(1 << 7);
        cpu.write_memory(hl, v);

        4
    }

    fn op_set0_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl) | (1);
        cpu.write_memory(hl, v);

        4
    }

    fn op_set1_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl) | (1 << 1);
        cpu.write_memory(hl, v);

        4
    }

    fn op_set2_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl) | (1 << 2);
        cpu.write_memory(hl, v);

        4
    }

    fn op_set3_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl) | (1 << 3);
        cpu.write_memory(hl, v);

        4
    }

    fn op_set4_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl) | (1 << 4);
        cpu.write_memory(hl, v);

        4
    }

    fn op_set5_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl) | (1 << 5);
        cpu.write_memory(hl, v);

        4
    }

    fn op_set6_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl) | (1 << 6);
        cpu.write_memory(hl, v);

        4
    }

    fn op_set7_mem_hl(cpu: &mut CPU) -> u8 {
        let hl = cpu.get_hl();
        let v = cpu.read_memory(hl) | (1 << 7);
        cpu.write_memory(hl, v);

        4
    }

    fn push(&mut self, v: u16) {
        self.registers.sp = self.registers.sp.wrapping_sub(2);
        self.write_word(self.registers.sp, v);
    }

    fn get_hl(&self) -> u16 {
        ((self.registers.h as u16) << 8) | self.registers.l as u16
    }

    fn set_hl(&mut self, hl: u16) {
        self.registers.h = (hl >> 8) as u8;
        self.registers.l = hl as u8;
    }

    fn set_flag(&mut self, mask: u8, val: bool) {
        if val {
            self.registers.f |= mask;
        }
        else {
            self.registers.f &= !(mask); 
        }
    }

    fn get_flag(&self, mask: u8) -> bool {
        self.registers.f & mask != 0
    }
}

// OP HELPERS

fn and(accum: &mut u8, v: u8, flags: &mut u8) -> u8 {
    *accum &= v;

    set_flag2(flags, FLAG_Z, *accum == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, true);
    set_flag2(flags, FLAG_C, false);

    1
}

fn or(accum: &mut u8, v: u8, flags: &mut u8) -> u8 {
    *accum |= v;

    set_flag2(flags, FLAG_Z, *accum == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, false);
    set_flag2(flags, FLAG_C, false);

    1
}

fn xor(reg: u8, accum: &mut u8, flags: &mut u8) -> u8 {
    *accum ^= reg;

    set_flag2(flags, FLAG_Z, *accum == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, false);
    set_flag2(flags, FLAG_C, false);

    1
}

fn rl(reg: &mut u8, flags: &mut u8) -> u8 {
    let prev_carry: u8 = get_flag2(flags, FLAG_C) as u8;
    
    let carry = ((*reg) & (1 << 7)) != 0;
    *reg = (*reg << 1) | prev_carry;

    set_flag2(flags, FLAG_Z, *reg == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, false);
    set_flag2(flags, FLAG_C, carry);

    2
}

fn rr(reg: &mut u8, flags: &mut u8) -> u8 {
    let prev_carry: u8 = if get_flag2(flags, FLAG_C) { 1 } else { 0 };
    
    let carry = (*reg) & 1 != 0;
    *reg = (*reg >> 1) | (prev_carry << 7);

    set_flag2(flags, FLAG_Z, *reg == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, false);
    set_flag2(flags, FLAG_C, carry);

    2
}

fn rlc(reg: &mut u8, flags: &mut u8) -> u8 {
    let carry = (*reg & 0x80) >> 7;
    *reg = (*reg << 1) | carry;

    set_flag2(flags, FLAG_Z, *reg == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, false);
    set_flag2(flags, FLAG_C, carry != 0);

    2
}

fn rrc(reg: &mut u8, flags: &mut u8) -> u8 {
    let carry = *reg & 0x1;
    *reg = (*reg >> 1) | (carry << 7);

    set_flag2(flags, FLAG_Z, *reg == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, false);
    set_flag2(flags, FLAG_C, carry != 0);

    2
}

fn cp_reg(a: u8, b: u8, flags: &mut u8) -> u8 {
    let half_borrow = is_half_borrow(&a, &b);
    let full_borrow = is_full_borrow(&a, &b);

    let r = a.wrapping_sub(b);
    
    set_flag2(flags, FLAG_Z, r == 0);
    set_flag2(flags, FLAG_N, true);
    set_flag2(flags, FLAG_H, half_borrow);
    set_flag2(flags, FLAG_C, full_borrow);

    1
}

fn add_reg(reg: &mut u8, value: u8, flags: &mut u8) -> u8 {
    let is_half_carry = is_half_carry(reg, &value);
    let is_full_carry = is_full_carry(reg, &value);

    *reg = (*reg).wrapping_add(value);

    set_flag2(flags, FLAG_Z, *reg == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, is_half_carry);
    set_flag2(flags, FLAG_C, is_full_carry);

    1
}

fn add_reg16(reg: &mut u16, value: u16, flags: &mut u8) -> u8 {
    let is_half_carry = is_half_carry16(reg, &value);
    let is_full_carry = is_full_carry16(reg, &value);

    *reg = (*reg).wrapping_add(value);

    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, is_half_carry);
    set_flag2(flags, FLAG_C, is_full_carry);

    2
}

fn adc_reg(reg: &mut u8, value: u8, flags: &mut u8) -> u8 {
    let cy = if get_flag2(flags, FLAG_C) { 1 } else { 0 };

    let mut r = (*reg).wrapping_add(value);

    let is_full_carry = is_full_carry(reg, &value) || is_full_carry(&r, &cy);
    let is_half_carry = is_half_carry(reg, &value) || is_half_carry(&r, &cy);

    r = r.wrapping_add(cy);

    *reg = r;

    set_flag2(flags, FLAG_Z, *reg == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, is_half_carry);
    set_flag2(flags, FLAG_C, is_full_carry);

    1
}

fn sbc_reg(reg: &mut u8, value: u8, flags: &mut u8) -> u8 {
    let cy = if get_flag2(flags, FLAG_C) { 1 } else { 0 };

    let mut r = (*reg).wrapping_sub(value);

    let is_full_borrow = is_full_borrow(reg, &value) || is_full_borrow(&r, &cy);
    let is_half_borrow = is_half_borrow(reg, &value) || is_half_borrow(&r, &cy);

    r = r.wrapping_sub(cy);
    *reg = r;

    set_flag2(flags, FLAG_Z, *reg == 0);
    set_flag2(flags, FLAG_N, true);
    set_flag2(flags, FLAG_H, is_half_borrow);
    set_flag2(flags, FLAG_C, is_full_borrow);

    1
}

fn sub(accum: &mut u8, v: u8, flags: &mut u8) -> u8 {
    let half_borrow = is_half_borrow(accum, &v);
    let full_borrow = is_full_borrow(accum, &v);

    *accum = (*accum).wrapping_sub(v);

    set_flag2(flags, FLAG_Z, *accum == 0);
    set_flag2(flags, FLAG_N, true);
    set_flag2(flags, FLAG_H, half_borrow);
    set_flag2(flags, FLAG_C, full_borrow);

    1
}

fn inc(reg: &mut u8, flags: &mut u8) -> u8 {
    *reg = (*reg).wrapping_add(1);

    set_flag2(flags, FLAG_Z, *reg == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, *reg & 0x0F == 0);

    1
}

fn dec(reg: &mut u8, flags: &mut u8) -> u8 {
    *reg = (*reg).wrapping_sub(1);

    set_flag2(flags, FLAG_Z, *reg == 0);
    set_flag2(flags, FLAG_N, true);
    set_flag2(flags, FLAG_H, *reg & 0x0F == 0x0F);

    1
}

fn swap_reg(reg: &mut u8, flags: &mut u8) -> u8 {
    let l = *reg & 0x0F;
    let h = *reg & 0xF0;

    *reg = l << 4 | h >> 4;

    set_flag2(flags, FLAG_Z, *reg == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, false);
    set_flag2(flags, FLAG_C, false);

    2
}

fn bit_n_reg(bit: u8, reg: &mut u8, flags: &mut u8) -> u8 {
    let b = (*reg >> bit) & 1;

    set_flag2(flags, FLAG_Z, b == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, true);

    2
}

fn sla_reg(reg: &mut u8, flags: &mut u8) -> u8 {
    let carry = *reg & (1 << 7) != 0;
    *reg <<= 1;

    set_flag2(flags, FLAG_Z, *reg == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, false);
    set_flag2(flags, FLAG_C, carry);

    2
}

fn srl_reg(reg: &mut u8, flags: &mut u8) -> u8 {
    let carry = *reg & 1;
    *reg >>= 1;

    set_flag2(flags, FLAG_Z, *reg == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, false);
    set_flag2(flags, FLAG_C, carry != 0);

    2
}

fn sra_reg(reg: &mut u8, flags: &mut u8) -> u8 {
    let carry = *reg & 0x1;
    *reg = (*reg >> 1) | (*reg & 0x80); 

    set_flag2(flags, FLAG_Z, *reg == 0);
    set_flag2(flags, FLAG_N, false);
    set_flag2(flags, FLAG_H, false);
    set_flag2(flags, FLAG_C, carry != 0);

    2
}

