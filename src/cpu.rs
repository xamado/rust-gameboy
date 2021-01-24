use crate::memorybus::MemoryBus;
use crate::bitutils::*;
use crate::machine::GameBoyModel;

use hashbrown::HashMap;
use core::cell::RefCell;

const FLAG_Z: u8 = 1 << 7;
const FLAG_N: u8 = 1 << 6;
const FLAG_H: u8 = 1 << 5;
const FLAG_C: u8 = 1 << 4;

#[derive(PartialEq)]
enum CPUMode {
    Normal,
    Halt,
    Stop
}

#[derive(Clone)]
pub struct Instruction {
    pub dissassembly: &'static str,
    bytes: u16,
    closure: fn(&CPU, InstructionContext) -> u8
}

struct InstructionContext<'a> {
    bus: &'a MemoryBus,
    r: &'a mut Registers,
}

struct Registers { // rename to CPURegisters ?
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

pub struct CPUDebugState {
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub sp: u16,
    pub pc: u16,
    pub next_opcode: u16,
}

pub struct InterruptRegisters {
    interrupts_enabled: bool,
    interrupts_enable_request: bool,
    flags: u8,
    enabled: u8,
}

struct CPUState {
    mode: CPUMode,
    next_op: u16,
}

pub struct CPU {
    model: GameBoyModel,
    state: RefCell<CPUState>,
    registers: RefCell<Registers>,
    instructions: HashMap<u16, Instruction>,
    interrupts: RefCell<InterruptRegisters>
}

impl CPU {
    pub fn new(model: GameBoyModel) -> Self {
        let instruction_table : HashMap<u16, Instruction> = [
            (0x0000_u16, Instruction { dissassembly: "NOP",         bytes: 1, closure: |cpu, _ctx| cpu.op_nop() }),
            (0x0010_u16, Instruction { dissassembly: "STOP",        bytes: 2, closure: |cpu, _ctx| cpu.op_stop() }),
            (0x0076_u16, Instruction { dissassembly: "HALT",        bytes: 1, closure: |cpu, _ctx| cpu.op_halt() }),
            (0x003C_u16, Instruction { dissassembly: "INC A",       bytes: 1, closure: |cpu, ctx| cpu.op_inc_r(&mut ctx.r.a, &mut ctx.r.f) }),
            (0x0004_u16, Instruction { dissassembly: "INC B",       bytes: 1, closure: |cpu, ctx| cpu.op_inc_r(&mut ctx.r.b, &mut ctx.r.f) }),
            (0x000C_u16, Instruction { dissassembly: "INC C",       bytes: 1, closure: |cpu, ctx| cpu.op_inc_r(&mut ctx.r.c, &mut ctx.r.f) }),
            (0x0014_u16, Instruction { dissassembly: "INC D",       bytes: 1, closure: |cpu, ctx| cpu.op_inc_r(&mut ctx.r.d, &mut ctx.r.f) }),
            (0x001C_u16, Instruction { dissassembly: "INC E",       bytes: 1, closure: |cpu, ctx| cpu.op_inc_r(&mut ctx.r.e, &mut ctx.r.f) }),
            (0x0024_u16, Instruction { dissassembly: "INC H",       bytes: 1, closure: |cpu, ctx| cpu.op_inc_r(&mut ctx.r.h, &mut ctx.r.f) }),
            (0x002C_u16, Instruction { dissassembly: "INC L",       bytes: 1, closure: |cpu, ctx| cpu.op_inc_r(&mut ctx.r.l, &mut ctx.r.f) }),
            (0x0003_u16, Instruction { dissassembly: "INC BC",      bytes: 1, closure: |cpu, ctx| cpu.op_inc_r16(&mut ctx.r.b, &mut ctx.r.c) }),
            (0x0013_u16, Instruction { dissassembly: "INC DE",      bytes: 1, closure: |cpu, ctx| cpu.op_inc_r16(&mut ctx.r.d, &mut ctx.r.e) }),
            (0x0023_u16, Instruction { dissassembly: "INC HL",      bytes: 1, closure: |cpu, ctx| cpu.op_inc_r16(&mut ctx.r.h, &mut ctx.r.l) }),
            (0x0033_u16, Instruction { dissassembly: "INC SP",      bytes: 1, closure: |cpu, ctx| cpu.op_inc_sp(&mut ctx.r.sp) }),
            (0x0034_u16, Instruction { dissassembly: "INC (HL)",    bytes: 1, closure: |cpu, ctx| cpu.op_inc_addr(ctx.bus, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0x003D_u16, Instruction { dissassembly: "DEC A",       bytes: 1, closure: |cpu, ctx| cpu.op_dec_r(&mut ctx.r.a, &mut ctx.r.f) }),
            (0x0005_u16, Instruction { dissassembly: "DEC B",       bytes: 1, closure: |cpu, ctx| cpu.op_dec_r(&mut ctx.r.b, &mut ctx.r.f) }),
            (0x000D_u16, Instruction { dissassembly: "DEC C",       bytes: 1, closure: |cpu, ctx| cpu.op_dec_r(&mut ctx.r.c, &mut ctx.r.f) }),
            (0x0015_u16, Instruction { dissassembly: "DEC D",       bytes: 1, closure: |cpu, ctx| cpu.op_dec_r(&mut ctx.r.d, &mut ctx.r.f) }),
            (0x001D_u16, Instruction { dissassembly: "DEC E",       bytes: 1, closure: |cpu, ctx| cpu.op_dec_r(&mut ctx.r.e, &mut ctx.r.f) }),
            (0x0025_u16, Instruction { dissassembly: "DEC H",       bytes: 1, closure: |cpu, ctx| cpu.op_dec_r(&mut ctx.r.h, &mut ctx.r.f) }),
            (0x002D_u16, Instruction { dissassembly: "DEC L",       bytes: 1, closure: |cpu, ctx| cpu.op_dec_r(&mut ctx.r.l, &mut ctx.r.f) }),
            (0x000B_u16, Instruction { dissassembly: "DEC BC",      bytes: 1, closure: |cpu, ctx| cpu.op_dec_r16(&mut ctx.r.b, &mut ctx.r.c) }),
            (0x001B_u16, Instruction { dissassembly: "DEC DE",      bytes: 1, closure: |cpu, ctx| cpu.op_dec_r16(&mut ctx.r.d, &mut ctx.r.e) }),
            (0x002B_u16, Instruction { dissassembly: "DEC HL",      bytes: 1, closure: |cpu, ctx| cpu.op_dec_r16(&mut ctx.r.h, &mut ctx.r.l) }),
            (0x003B_u16, Instruction { dissassembly: "DEC SP",      bytes: 1, closure: |cpu, ctx| cpu.op_dec_sp(&mut ctx.r.sp) }),
            (0x0035_u16, Instruction { dissassembly: "DEC (HL)",    bytes: 1, closure: |cpu, ctx| cpu.op_dec_addr(ctx.bus, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0x0087_u16, Instruction { dissassembly: "ADD A,A",     bytes: 1, closure: |cpu, ctx| { let v = ctx.r.a; cpu.op_add_r(&mut ctx.r.a, v, &mut ctx.r.f) } }),
            (0x0080_u16, Instruction { dissassembly: "ADD A,B",     bytes: 1, closure: |cpu, ctx| cpu.op_add_r(&mut ctx.r.a, ctx.r.b, &mut ctx.r.f) }),
            (0x0081_u16, Instruction { dissassembly: "ADD A,C",     bytes: 1, closure: |cpu, ctx| cpu.op_add_r(&mut ctx.r.a, ctx.r.c, &mut ctx.r.f) }),
            (0x0082_u16, Instruction { dissassembly: "ADD A,D",     bytes: 1, closure: |cpu, ctx| cpu.op_add_r(&mut ctx.r.a, ctx.r.d, &mut ctx.r.f) }),
            (0x0083_u16, Instruction { dissassembly: "ADD A,E",     bytes: 1, closure: |cpu, ctx| cpu.op_add_r(&mut ctx.r.a, ctx.r.e, &mut ctx.r.f) }),
            (0x0084_u16, Instruction { dissassembly: "ADD A,H",     bytes: 1, closure: |cpu, ctx| cpu.op_add_r(&mut ctx.r.a, ctx.r.h, &mut ctx.r.f) }),
            (0x0085_u16, Instruction { dissassembly: "ADD A,L",     bytes: 1, closure: |cpu, ctx| cpu.op_add_r(&mut ctx.r.a, ctx.r.l, &mut ctx.r.f) }),
            (0x00C6_u16, Instruction { dissassembly: "ADD A,d8",    bytes: 2, closure: |cpu, ctx| cpu.op_add_d8(ctx.bus, &mut ctx.r.a, &mut ctx.r.pc, &mut ctx.r.f) }),
            (0x0086_u16, Instruction { dissassembly: "ADD A,(HL)",  bytes: 1, closure: |cpu, ctx| { cpu.op_add_addr(ctx.bus, &mut ctx.r.a, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) } }),
            (0x0009_u16, Instruction { dissassembly: "ADD HL,BC",   bytes: 1, closure: |cpu, ctx| cpu.op_add_r16(&mut ctx.r.h, &mut ctx.r.l, to_u16(ctx.r.b, ctx.r.c), &mut ctx.r.f) }),
            (0x0019_u16, Instruction { dissassembly: "ADD HL,DE",   bytes: 1, closure: |cpu, ctx| cpu.op_add_r16(&mut ctx.r.h, &mut ctx.r.l, to_u16(ctx.r.d, ctx.r.e), &mut ctx.r.f) }),
            (0x0029_u16, Instruction { dissassembly: "ADD HL,HL",   bytes: 1, closure: |cpu, ctx| { let v = to_u16(ctx.r.h, ctx.r.l); cpu.op_add_r16(&mut ctx.r.h, &mut ctx.r.l, v, &mut ctx.r.f) } }),
            (0x0039_u16, Instruction { dissassembly: "ADD HL,SP",   bytes: 1, closure: |cpu, ctx| cpu.op_add_r16(&mut ctx.r.h, &mut ctx.r.l, ctx.r.sp, &mut ctx.r.f) }),
            (0x00E8_u16, Instruction { dissassembly: "ADD SP,s8",   bytes: 2, closure: |cpu, ctx| cpu.op_add_sp_s8(ctx.bus, &mut ctx.r.sp, &mut ctx.r.pc, &mut ctx.r.f) }),
            (0x0097_u16, Instruction { dissassembly: "SUB A",       bytes: 1, closure: |cpu, ctx| { let v = ctx.r.a; cpu.op_sub_r(&mut ctx.r.a, v, &mut ctx.r.f) } }),
            (0x0090_u16, Instruction { dissassembly: "SUB B",       bytes: 1, closure: |cpu, ctx| cpu.op_sub_r(&mut ctx.r.a, ctx.r.b, &mut ctx.r.f) }),
            (0x0091_u16, Instruction { dissassembly: "SUB C",       bytes: 1, closure: |cpu, ctx| cpu.op_sub_r(&mut ctx.r.a, ctx.r.c, &mut ctx.r.f) }),
            (0x0092_u16, Instruction { dissassembly: "SUB D",       bytes: 1, closure: |cpu, ctx| cpu.op_sub_r(&mut ctx.r.a, ctx.r.d, &mut ctx.r.f) }),
            (0x0093_u16, Instruction { dissassembly: "SUB E",       bytes: 1, closure: |cpu, ctx| cpu.op_sub_r(&mut ctx.r.a, ctx.r.e, &mut ctx.r.f) }),
            (0x0094_u16, Instruction { dissassembly: "SUB H",       bytes: 1, closure: |cpu, ctx| cpu.op_sub_r(&mut ctx.r.a, ctx.r.h, &mut ctx.r.f) }),
            (0x0095_u16, Instruction { dissassembly: "SUB L",       bytes: 1, closure: |cpu, ctx| cpu.op_sub_r(&mut ctx.r.a, ctx.r.l, &mut ctx.r.f) }),
            (0x00D6_u16, Instruction { dissassembly: "SUB d8",      bytes: 2, closure: |cpu, ctx| cpu.op_sub_d8(ctx.bus, &mut ctx.r.a, &mut ctx.r.pc, &mut ctx.r.f) }),
            (0x0096_u16, Instruction { dissassembly: "SUB (HL)",    bytes: 1, closure: |cpu, ctx| cpu.op_sub_addr(ctx.bus, &mut ctx.r.a, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0x008F_u16, Instruction { dissassembly: "ADC A,A",     bytes: 1, closure: |cpu, ctx| { let v = ctx.r.a; cpu.op_adc_r(&mut ctx.r.a, v, &mut ctx.r.f) } }),
            (0x0088_u16, Instruction { dissassembly: "ADC A,B",     bytes: 1, closure: |cpu, ctx| cpu.op_adc_r(&mut ctx.r.a, ctx.r.b, &mut ctx.r.f) }),
            (0x0089_u16, Instruction { dissassembly: "ADC A,C",     bytes: 1, closure: |cpu, ctx| cpu.op_adc_r(&mut ctx.r.a, ctx.r.c, &mut ctx.r.f) }),
            (0x008A_u16, Instruction { dissassembly: "ADC A,D",     bytes: 1, closure: |cpu, ctx| cpu.op_adc_r(&mut ctx.r.a, ctx.r.d, &mut ctx.r.f) }),
            (0x008B_u16, Instruction { dissassembly: "ADC A,E",     bytes: 1, closure: |cpu, ctx| cpu.op_adc_r(&mut ctx.r.a, ctx.r.e, &mut ctx.r.f) }),
            (0x008C_u16, Instruction { dissassembly: "ADC A,H",     bytes: 1, closure: |cpu, ctx| cpu.op_adc_r(&mut ctx.r.a, ctx.r.h, &mut ctx.r.f) }),
            (0x008D_u16, Instruction { dissassembly: "ADC A,L",     bytes: 1, closure: |cpu, ctx| cpu.op_adc_r(&mut ctx.r.a, ctx.r.l, &mut ctx.r.f) }),
            (0x00CE_u16, Instruction { dissassembly: "ADC A,d8",    bytes: 2, closure: |cpu, ctx| cpu.op_adc_d8(ctx.bus, &mut ctx.r.a, &mut ctx.r.pc, &mut ctx.r.f) }),
            (0x008E_u16, Instruction { dissassembly: "ADC A,(HL)",  bytes: 1, closure: |cpu, ctx| cpu.op_adc_addr(ctx.bus, &mut ctx.r.a, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0x009F_u16, Instruction { dissassembly: "SBC A,A",     bytes: 1, closure: |cpu, ctx| { let v = ctx.r.a; cpu.op_sbc_r(&mut ctx.r.a, v, &mut ctx.r.f) } }),
            (0x0098_u16, Instruction { dissassembly: "SBC A,B",     bytes: 1, closure: |cpu, ctx| cpu.op_sbc_r(&mut ctx.r.a, ctx.r.b, &mut ctx.r.f) }),
            (0x0099_u16, Instruction { dissassembly: "SBC A,C",     bytes: 1, closure: |cpu, ctx| cpu.op_sbc_r(&mut ctx.r.a, ctx.r.c, &mut ctx.r.f) }),
            (0x009A_u16, Instruction { dissassembly: "SBC A,D",     bytes: 1, closure: |cpu, ctx| cpu.op_sbc_r(&mut ctx.r.a, ctx.r.d, &mut ctx.r.f) }),
            (0x009B_u16, Instruction { dissassembly: "SBC A,E",     bytes: 1, closure: |cpu, ctx| cpu.op_sbc_r(&mut ctx.r.a, ctx.r.e, &mut ctx.r.f) }),
            (0x009C_u16, Instruction { dissassembly: "SBC A,H",     bytes: 1, closure: |cpu, ctx| cpu.op_sbc_r(&mut ctx.r.a, ctx.r.h, &mut ctx.r.f) }),
            (0x009D_u16, Instruction { dissassembly: "SBC A,L",     bytes: 1, closure: |cpu, ctx| cpu.op_sbc_r(&mut ctx.r.a, ctx.r.l, &mut ctx.r.f) }),
            (0x00DE_u16, Instruction { dissassembly: "SBC A,d8",    bytes: 2, closure: |cpu, ctx| cpu.op_sbc_d8(ctx.bus, &mut ctx.r.a, &mut ctx.r.pc, &mut ctx.r.f) }),
            (0x009E_u16, Instruction { dissassembly: "SBC A,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_sbc_addr(ctx.bus, &mut ctx.r.a, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0x0027_u16, Instruction { dissassembly: "DAA",         bytes: 1, closure: |cpu, ctx| cpu.op_daa(&mut ctx.r.a, &mut ctx.r.f) }),
            (0x0037_u16, Instruction { dissassembly: "SCF",         bytes: 1, closure: |cpu, ctx| cpu.op_scf(&mut ctx.r.f) }),
            (0x003F_u16, Instruction { dissassembly: "CCF",         bytes: 1, closure: |cpu, ctx| cpu.op_ccf(&mut ctx.r.f) }),
            (0x00BF_u16, Instruction { dissassembly: "CP A",        bytes: 1, closure: |cpu, ctx| cpu.op_cp_r(ctx.r.a, ctx.r.a, &mut ctx.r.f) }),
            (0x00B8_u16, Instruction { dissassembly: "CP B",        bytes: 1, closure: |cpu, ctx| cpu.op_cp_r(ctx.r.a, ctx.r.b, &mut ctx.r.f) }),
            (0x00B9_u16, Instruction { dissassembly: "CP C",        bytes: 1, closure: |cpu, ctx| cpu.op_cp_r(ctx.r.a, ctx.r.c, &mut ctx.r.f) }),
            (0x00BA_u16, Instruction { dissassembly: "CP D",        bytes: 1, closure: |cpu, ctx| cpu.op_cp_r(ctx.r.a, ctx.r.d, &mut ctx.r.f) }),
            (0x00BB_u16, Instruction { dissassembly: "CP E",        bytes: 1, closure: |cpu, ctx| cpu.op_cp_r(ctx.r.a, ctx.r.e, &mut ctx.r.f) }),
            (0x00BC_u16, Instruction { dissassembly: "CP H",        bytes: 1, closure: |cpu, ctx| cpu.op_cp_r(ctx.r.a, ctx.r.h, &mut ctx.r.f) }),
            (0x00BD_u16, Instruction { dissassembly: "CP L",        bytes: 1, closure: |cpu, ctx| cpu.op_cp_r(ctx.r.a, ctx.r.l, &mut ctx.r.f) }),
            (0x00FE_u16, Instruction { dissassembly: "CP d8",       bytes: 1, closure: |cpu, ctx| cpu.op_cp_d8(ctx.bus, ctx.r.a, &mut ctx.r.pc, &mut ctx.r.f) }),
            (0x00BE_u16, Instruction { dissassembly: "CP (HL)",     bytes: 1, closure: |cpu, ctx| cpu.op_cp_addr(ctx.bus, ctx.r.a, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            // LOAD instructions
            (0x007F_u16, Instruction { dissassembly: "LD A,A",      bytes: 1, closure: |cpu, _ctx| cpu.op_nop() }),
            (0x0078_u16, Instruction { dissassembly: "LD A,B",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.a, ctx.r.b) }),
            (0x0079_u16, Instruction { dissassembly: "LD A,C",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.a, ctx.r.c) }),
            (0x007A_u16, Instruction { dissassembly: "LD A,D",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.a, ctx.r.d) }),
            (0x007B_u16, Instruction { dissassembly: "LD A,E",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.a, ctx.r.e) }),
            (0x007C_u16, Instruction { dissassembly: "LD A,H",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.a, ctx.r.h) }),
            (0x007D_u16, Instruction { dissassembly: "LD A,L",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.a, ctx.r.l) }),
            (0x0047_u16, Instruction { dissassembly: "LD B,A",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.b, ctx.r.a) }),
            (0x0040_u16, Instruction { dissassembly: "LD B,B",      bytes: 1, closure: |cpu, _ctx| cpu.op_nop() }),
            (0x0041_u16, Instruction { dissassembly: "LD B,C",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.b, ctx.r.c) }),
            (0x0042_u16, Instruction { dissassembly: "LD B,D",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.b, ctx.r.d) }),
            (0x0043_u16, Instruction { dissassembly: "LD B,E",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.b, ctx.r.e) }),
            (0x0044_u16, Instruction { dissassembly: "LD B,H",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.b, ctx.r.h) }),
            (0x0045_u16, Instruction { dissassembly: "LD B,L",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.b, ctx.r.l) }),
            (0x004F_u16, Instruction { dissassembly: "LD C,A",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.c, ctx.r.a) }),
            (0x0048_u16, Instruction { dissassembly: "LD C,B",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.c, ctx.r.b) }),
            (0x0049_u16, Instruction { dissassembly: "LD C,C",      bytes: 1, closure: |cpu, _ctx| cpu.op_nop() }),
            (0x004A_u16, Instruction { dissassembly: "LD C,D",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.c, ctx.r.d) }),
            (0x004B_u16, Instruction { dissassembly: "LD C,E",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.c, ctx.r.e) }),
            (0x004C_u16, Instruction { dissassembly: "LD C,H",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.c, ctx.r.h) }),
            (0x004D_u16, Instruction { dissassembly: "LD C,L",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.c, ctx.r.l) }),
            (0x0057_u16, Instruction { dissassembly: "LD D,A",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.d, ctx.r.a) }),
            (0x0050_u16, Instruction { dissassembly: "LD D,B",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.d, ctx.r.b) }),
            (0x0051_u16, Instruction { dissassembly: "LD D,C",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.d, ctx.r.c) }),
            (0x0052_u16, Instruction { dissassembly: "LD D,D",      bytes: 1, closure: |cpu, _ctx| cpu.op_nop() }),
            (0x0053_u16, Instruction { dissassembly: "LD D,E",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.d, ctx.r.e) }),
            (0x0054_u16, Instruction { dissassembly: "LD D,H",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.d, ctx.r.h) }),
            (0x0055_u16, Instruction { dissassembly: "LD D,L",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.d, ctx.r.l) }),
            (0x005F_u16, Instruction { dissassembly: "LD E,A",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.e, ctx.r.a) }),
            (0x0058_u16, Instruction { dissassembly: "LD E,B",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.e, ctx.r.b) }),
            (0x0059_u16, Instruction { dissassembly: "LD E,C",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.e, ctx.r.c) }),
            (0x005A_u16, Instruction { dissassembly: "LD E,D",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.e, ctx.r.d) }),
            (0x005B_u16, Instruction { dissassembly: "LD E,E",      bytes: 1, closure: |cpu, _ctx| cpu.op_nop() }),
            (0x005C_u16, Instruction { dissassembly: "LD E,H",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.e, ctx.r.h) }),
            (0x005D_u16, Instruction { dissassembly: "LD E,L",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.e, ctx.r.l) }),
            (0x0067_u16, Instruction { dissassembly: "LD H,A",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.h, ctx.r.a) }),
            (0x0060_u16, Instruction { dissassembly: "LD H,B",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.h, ctx.r.b) }),
            (0x0061_u16, Instruction { dissassembly: "LD H,C",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.h, ctx.r.c) }),
            (0x0062_u16, Instruction { dissassembly: "LD H,D",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.h, ctx.r.d) }),
            (0x0063_u16, Instruction { dissassembly: "LD H,E",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.h, ctx.r.e) }),
            (0x0064_u16, Instruction { dissassembly: "LD H,H",      bytes: 1, closure: |cpu, _ctx| cpu.op_nop() }),
            (0x0065_u16, Instruction { dissassembly: "LD H,L",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.h, ctx.r.l) }),
            (0x006F_u16, Instruction { dissassembly: "LD L,A",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.l, ctx.r.a) }),
            (0x0068_u16, Instruction { dissassembly: "LD L,B",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.l, ctx.r.b) }),
            (0x0069_u16, Instruction { dissassembly: "LD L,C",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.l, ctx.r.c) }),
            (0x006A_u16, Instruction { dissassembly: "LD L,D",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.l, ctx.r.d) }),
            (0x006B_u16, Instruction { dissassembly: "LD L,E",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.l, ctx.r.e) }),
            (0x006C_u16, Instruction { dissassembly: "LD L,H",      bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_r(&mut ctx.r.l, ctx.r.h) }),
            (0x006D_u16, Instruction { dissassembly: "LD L,L",      bytes: 1, closure: |cpu, _ctx| cpu.op_nop() }),
            (0x0066_u16, Instruction { dissassembly: "LD H,(HL)",   bytes: 1, closure: |cpu, ctx| { let hl = to_u16(ctx.r.h, ctx.r.l); cpu.op_ld_r_addr(ctx.bus, &mut ctx.r.h, hl) } }),
            (0x006E_u16, Instruction { dissassembly: "LD L,(HL)",   bytes: 1, closure: |cpu, ctx| { let hl = to_u16(ctx.r.h, ctx.r.l); cpu.op_ld_r_addr(ctx.bus, &mut ctx.r.l, hl) } }),
            (0x003E_u16, Instruction { dissassembly: "LD A,d8",     bytes: 2, closure: |cpu, ctx| cpu.op_ld_r_d8(ctx.bus, &mut ctx.r.a, &mut ctx.r.pc) }),
            (0x0006_u16, Instruction { dissassembly: "LD B,d8",     bytes: 2, closure: |cpu, ctx| cpu.op_ld_r_d8(ctx.bus, &mut ctx.r.b, &mut ctx.r.pc) }),
            (0x000E_u16, Instruction { dissassembly: "LD C,d8",     bytes: 2, closure: |cpu, ctx| cpu.op_ld_r_d8(ctx.bus, &mut ctx.r.c, &mut ctx.r.pc) }),
            (0x0016_u16, Instruction { dissassembly: "LD D,d8",     bytes: 2, closure: |cpu, ctx| cpu.op_ld_r_d8(ctx.bus, &mut ctx.r.d, &mut ctx.r.pc) }),
            (0x001E_u16, Instruction { dissassembly: "LD E,d8",     bytes: 2, closure: |cpu, ctx| cpu.op_ld_r_d8(ctx.bus, &mut ctx.r.e, &mut ctx.r.pc) }),
            (0x0026_u16, Instruction { dissassembly: "LD H,d8",     bytes: 2, closure: |cpu, ctx| cpu.op_ld_r_d8(ctx.bus, &mut ctx.r.h, &mut ctx.r.pc) }),
            (0x002E_u16, Instruction { dissassembly: "LD L,d8",     bytes: 2, closure: |cpu, ctx| cpu.op_ld_r_d8(ctx.bus, &mut ctx.r.l, &mut ctx.r.pc) }),
            (0x0001_u16, Instruction { dissassembly: "LD BC,d16",   bytes: 3, closure: |cpu, ctx| cpu.op_ld_r_d16(ctx.bus, &mut ctx.r.b, &mut ctx.r.c, &mut ctx.r.pc) }),
            (0x0011_u16, Instruction { dissassembly: "LD DE,d16",   bytes: 3, closure: |cpu, ctx| cpu.op_ld_r_d16(ctx.bus, &mut ctx.r.d, &mut ctx.r.e, &mut ctx.r.pc) }),
            (0x0021_u16, Instruction { dissassembly: "LD HL,d16",   bytes: 3, closure: |cpu, ctx| cpu.op_ld_r_d16(ctx.bus, &mut ctx.r.h, &mut ctx.r.l, &mut ctx.r.pc) }),
            (0x0031_u16, Instruction { dissassembly: "LD SP,d16",   bytes: 3, closure: |cpu, ctx| cpu.op_ld_sp_d16(ctx.bus, &mut ctx.r.sp, &mut ctx.r.pc) }),
            (0x00F9_u16, Instruction { dissassembly: "LD SP,HL",    bytes: 1, closure: |cpu, ctx| cpu.op_ld_sp_r16(&mut ctx.r.sp, to_u16(ctx.r.h, ctx.r.l)) }),
            (0x00F8_u16, Instruction { dissassembly: "LD HL,SP+s8", bytes: 2, closure: |cpu, ctx| cpu.op_ld_hl_sp_add_s8(ctx.bus, &mut ctx.r.h, &mut ctx.r.l, ctx.r.sp, &mut ctx.r.pc, &mut ctx.r.f) }),
            (0x00F2_u16, Instruction { dissassembly: "LD A,(C)",    bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_addr(ctx.bus, &mut ctx.r.a, 0xFF00 | (ctx.r.c as u16)) }),
            (0x000A_u16, Instruction { dissassembly: "LD A,(BC)",   bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_addr(ctx.bus, &mut ctx.r.a, to_u16(ctx.r.b, ctx.r.c)) }),
            (0x001A_u16, Instruction { dissassembly: "LD A,(DE)",   bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_addr(ctx.bus, &mut ctx.r.a, to_u16(ctx.r.d, ctx.r.e)) }),
            (0x007E_u16, Instruction { dissassembly: "LD A,(HL)",   bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_addr(ctx.bus, &mut ctx.r.a, to_u16(ctx.r.h, ctx.r.l)) }),
            (0x0046_u16, Instruction { dissassembly: "LD B,(HL)",   bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_addr(ctx.bus, &mut ctx.r.b, to_u16(ctx.r.h, ctx.r.l)) }),
            (0x004E_u16, Instruction { dissassembly: "LD C,(HL)",   bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_addr(ctx.bus, &mut ctx.r.c, to_u16(ctx.r.h, ctx.r.l)) }),
            (0x0056_u16, Instruction { dissassembly: "LD D,(HL)",   bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_addr(ctx.bus, &mut ctx.r.d, to_u16(ctx.r.h, ctx.r.l)) }),
            (0x005E_u16, Instruction { dissassembly: "LD E,(HL)",   bytes: 1, closure: |cpu, ctx| cpu.op_ld_r_addr(ctx.bus, &mut ctx.r.e, to_u16(ctx.r.h, ctx.r.l)) }),
            (0x00F0_u16, Instruction { dissassembly: "LD A,(d8)",   bytes: 2, closure: |cpu, ctx| cpu.op_ld_r_a8(ctx.bus, &mut ctx.r.a, &mut ctx.r.pc) }),
            (0x00FA_u16, Instruction { dissassembly: "LD A,(a16)",  bytes: 3, closure: |cpu, ctx| cpu.op_ld_r_a16(ctx.bus, &mut ctx.r.a, &mut ctx.r.pc) }),
            (0x002A_u16, Instruction { dissassembly: "LD A,(HL+)",  bytes: 1, closure: |cpu, ctx| cpu.op_ld_a_mem_hl_inc(ctx.bus, &mut ctx.r.a, &mut ctx.r.h, &mut ctx.r.l) }),
            (0x003A_u16, Instruction { dissassembly: "LD A,(HL-)",  bytes: 1, closure: |cpu, ctx| cpu.op_ld_a_mem_hl_dec(ctx.bus, &mut ctx.r.a, &mut ctx.r.h, &mut ctx.r.l) }),
            (0x00E2_u16, Instruction { dissassembly: "LD (C),A",    bytes: 1, closure: |cpu, ctx| cpu.op_ld_addr_r(ctx.bus, 0xFF00 | (ctx.r.c as u16), ctx.r.a) }),
            (0x0002_u16, Instruction { dissassembly: "LD (BC),A",   bytes: 1, closure: |cpu, ctx| cpu.op_ld_addr_r(ctx.bus, to_u16(ctx.r.b, ctx.r.c), ctx.r.a) }),
            (0x0012_u16, Instruction { dissassembly: "LD (DE),A",   bytes: 1, closure: |cpu, ctx| cpu.op_ld_addr_r(ctx.bus, to_u16(ctx.r.d, ctx.r.e), ctx.r.a) }),
            (0x0077_u16, Instruction { dissassembly: "LD (HL),A",   bytes: 1, closure: |cpu, ctx| cpu.op_ld_addr_r(ctx.bus, to_u16(ctx.r.h, ctx.r.l), ctx.r.a) }),
            (0x0070_u16, Instruction { dissassembly: "LD (HL),B",   bytes: 1, closure: |cpu, ctx| cpu.op_ld_addr_r(ctx.bus, to_u16(ctx.r.h, ctx.r.l), ctx.r.b) }),
            (0x0071_u16, Instruction { dissassembly: "LD (HL),C",   bytes: 1, closure: |cpu, ctx| cpu.op_ld_addr_r(ctx.bus, to_u16(ctx.r.h, ctx.r.l), ctx.r.c) }),
            (0x0072_u16, Instruction { dissassembly: "LD (HL),D",   bytes: 1, closure: |cpu, ctx| cpu.op_ld_addr_r(ctx.bus, to_u16(ctx.r.h, ctx.r.l), ctx.r.d) }),
            (0x0073_u16, Instruction { dissassembly: "LD (HL),E",   bytes: 1, closure: |cpu, ctx| cpu.op_ld_addr_r(ctx.bus, to_u16(ctx.r.h, ctx.r.l), ctx.r.e) }),
            (0x0074_u16, Instruction { dissassembly: "LD (HL),H",   bytes: 2, closure: |cpu, ctx| cpu.op_ld_addr_r(ctx.bus, to_u16(ctx.r.h, ctx.r.l), ctx.r.h) }),
            (0x0075_u16, Instruction { dissassembly: "LD (HL),L",   bytes: 2, closure: |cpu, ctx| cpu.op_ld_addr_r(ctx.bus, to_u16(ctx.r.h, ctx.r.l), ctx.r.l) }),
            (0x0032_u16, Instruction { dissassembly: "LD (HL-),A",  bytes: 1, closure: |cpu, ctx| cpu.op_ld_addr_r_dec_hl(ctx.bus, &mut ctx.r.h, &mut ctx.r.l, ctx.r.a) }),
            (0x0022_u16, Instruction { dissassembly: "LD (HL+),A",  bytes: 1, closure: |cpu, ctx| cpu.op_ld_addr_r_inc_hl(ctx.bus, &mut ctx.r.h, &mut ctx.r.l, ctx.r.a) }),
            (0x0036_u16, Instruction { dissassembly: "LD (HL),d8",  bytes: 1, closure: |cpu, ctx| cpu.op_ld_addr_d8(ctx.bus, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.pc) }),
            (0x00E0_u16, Instruction { dissassembly: "LD (a8),A",   bytes: 2, closure: |cpu, ctx| cpu.op_ld_a8_r(ctx.bus, &mut ctx.r.pc, ctx.r.a) }),
            (0x00EA_u16, Instruction { dissassembly: "LD (a16),A",  bytes: 3, closure: |cpu, ctx| cpu.op_ld_a16_r(ctx.bus, &mut ctx.r.pc, ctx.r.a) }), 
            (0x0008_u16, Instruction { dissassembly: "LD (a16),SP", bytes: 3, closure: |cpu, ctx| cpu.op_ld_a16_r16(ctx.bus, &mut ctx.r.pc, ctx.r.sp) }),
            // BITWISE operations
            (0x00A7_u16, Instruction { dissassembly: "AND A",       bytes: 1, closure: |cpu, ctx| { let v = ctx.r.a; cpu.op_and_r(&mut ctx.r.a, v, &mut ctx.r.f) } }),
            (0x00A0_u16, Instruction { dissassembly: "AND B",       bytes: 1, closure: |cpu, ctx| cpu.op_and_r(&mut ctx.r.a, ctx.r.b, &mut ctx.r.f) }),
            (0x00A1_u16, Instruction { dissassembly: "AND C",       bytes: 1, closure: |cpu, ctx| cpu.op_and_r(&mut ctx.r.a, ctx.r.c, &mut ctx.r.f) }),
            (0x00A2_u16, Instruction { dissassembly: "AND D",       bytes: 1, closure: |cpu, ctx| cpu.op_and_r(&mut ctx.r.a, ctx.r.d, &mut ctx.r.f) }),
            (0x00A3_u16, Instruction { dissassembly: "AND E",       bytes: 1, closure: |cpu, ctx| cpu.op_and_r(&mut ctx.r.a, ctx.r.e, &mut ctx.r.f) }),
            (0x00A4_u16, Instruction { dissassembly: "AND H",       bytes: 1, closure: |cpu, ctx| cpu.op_and_r(&mut ctx.r.a, ctx.r.h, &mut ctx.r.f) }),
            (0x00A5_u16, Instruction { dissassembly: "AND L",       bytes: 1, closure: |cpu, ctx| cpu.op_and_r(&mut ctx.r.a, ctx.r.l, &mut ctx.r.f) }),
            (0x00E6_u16, Instruction { dissassembly: "AND d8",      bytes: 2, closure: |cpu, ctx| cpu.op_and_d8(ctx.bus, &mut ctx.r.a, &mut ctx.r.pc, &mut ctx.r.f) }),
            (0x00A6_u16, Instruction { dissassembly: "AND (HL)",    bytes: 1, closure: |cpu, ctx| cpu.op_and_addr(ctx.bus, &mut ctx.r.a, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0x00B7_u16, Instruction { dissassembly: "OR A",        bytes: 1, closure: |cpu, ctx| { let v = ctx.r.a; cpu.op_or_r(&mut ctx.r.a, v, &mut ctx.r.f) } }),
            (0x00B0_u16, Instruction { dissassembly: "OR B",        bytes: 1, closure: |cpu, ctx| cpu.op_or_r(&mut ctx.r.a, ctx.r.b, &mut ctx.r.f) }),
            (0x00B1_u16, Instruction { dissassembly: "OR C",        bytes: 1, closure: |cpu, ctx| cpu.op_or_r(&mut ctx.r.a, ctx.r.c, &mut ctx.r.f) }),
            (0x00B2_u16, Instruction { dissassembly: "OR D",        bytes: 1, closure: |cpu, ctx| cpu.op_or_r(&mut ctx.r.a, ctx.r.d, &mut ctx.r.f) }),
            (0x00B3_u16, Instruction { dissassembly: "OR E",        bytes: 1, closure: |cpu, ctx| cpu.op_or_r(&mut ctx.r.a, ctx.r.e, &mut ctx.r.f) }),
            (0x00B4_u16, Instruction { dissassembly: "OR H",        bytes: 1, closure: |cpu, ctx| cpu.op_or_r(&mut ctx.r.a, ctx.r.h, &mut ctx.r.f) }),
            (0x00B5_u16, Instruction { dissassembly: "OR L",        bytes: 1, closure: |cpu, ctx| cpu.op_or_r(&mut ctx.r.a, ctx.r.l, &mut ctx.r.f) }),
            (0x00F6_u16, Instruction { dissassembly: "OR d8",       bytes: 2, closure: |cpu, ctx| cpu.op_or_d8(ctx.bus, &mut ctx.r.a, &mut ctx.r.pc, &mut ctx.r.f) }),
            (0x00B6_u16, Instruction { dissassembly: "OR (HL)",     bytes: 1, closure: |cpu, ctx| cpu.op_or_addr(ctx.bus, &mut ctx.r.a, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0x00AF_u16, Instruction { dissassembly: "XOR A",       bytes: 1, closure: |cpu, ctx| { let v = ctx.r.a; cpu.op_xor_r(&mut ctx.r.a, v, &mut ctx.r.f) } }),
            (0x00A8_u16, Instruction { dissassembly: "XOR B",       bytes: 1, closure: |cpu, ctx| cpu.op_xor_r(&mut ctx.r.a, ctx.r.b, &mut ctx.r.f) }),
            (0x00A9_u16, Instruction { dissassembly: "XOR C",       bytes: 1, closure: |cpu, ctx| cpu.op_xor_r(&mut ctx.r.a, ctx.r.c, &mut ctx.r.f) }),
            (0x00AA_u16, Instruction { dissassembly: "XOR D",       bytes: 1, closure: |cpu, ctx| cpu.op_xor_r(&mut ctx.r.a, ctx.r.d, &mut ctx.r.f) }),
            (0x00AB_u16, Instruction { dissassembly: "XOR E",       bytes: 1, closure: |cpu, ctx| cpu.op_xor_r(&mut ctx.r.a, ctx.r.e, &mut ctx.r.f) }),
            (0x00AC_u16, Instruction { dissassembly: "XOR H",       bytes: 1, closure: |cpu, ctx| cpu.op_xor_r(&mut ctx.r.a, ctx.r.h, &mut ctx.r.f) }),
            (0x00AD_u16, Instruction { dissassembly: "XOR L",       bytes: 1, closure: |cpu, ctx| cpu.op_xor_r(&mut ctx.r.a, ctx.r.l, &mut ctx.r.f) }),
            (0x00EE_u16, Instruction { dissassembly: "XOR d8",      bytes: 2, closure: |cpu, ctx| cpu.op_xor_d8(ctx.bus, &mut ctx.r.a, &mut ctx.r.pc, &mut ctx.r.f) }),
            (0x00AE_u16, Instruction { dissassembly: "XOR (HL)",    bytes: 1, closure: |cpu, ctx| cpu.op_xor_addr(ctx.bus, &mut ctx.r.a, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0x002F_u16, Instruction { dissassembly: "CPL",         bytes: 1, closure: |cpu, ctx| cpu.op_cpl(&mut ctx.r.a, &mut ctx.r.f) }),
            (0x0017_u16, Instruction { dissassembly: "RLA",         bytes: 1, closure: |cpu, ctx| cpu.op_rla(&mut ctx.r.a, &mut ctx.r.f) }), 
            (0x001F_u16, Instruction { dissassembly: "RRA",         bytes: 1, closure: |cpu, ctx| cpu.op_rra(&mut ctx.r.a, &mut ctx.r.f) }), 
            (0x0007_u16, Instruction { dissassembly: "RLCA",        bytes: 1, closure: |cpu, ctx| cpu.op_rlca(&mut ctx.r.a, &mut ctx.r.f) }),
            (0x000F_u16, Instruction { dissassembly: "RRCA",        bytes: 1, closure: |cpu, ctx| cpu.op_rrca(&mut ctx.r.a, &mut ctx.r.f) }),
            // FLOW CONTROL
            (0x00E9_u16, Instruction { dissassembly: "JP HL",       bytes: 1, closure: |cpu, ctx| cpu.op_jp_v16(&mut ctx.r.pc, to_u16(ctx.r.h, ctx.r.l)) }),
            (0x00C3_u16, Instruction { dissassembly: "JP a16",      bytes: 3, closure: |cpu, ctx| cpu.op_jp_a16(ctx.bus, &mut ctx.r.pc, true) }),
            (0x00C2_u16, Instruction { dissassembly: "JP NZ,a16",   bytes: 3, closure: |cpu, ctx| cpu.op_jp_a16(ctx.bus, &mut ctx.r.pc, !get_flag2(ctx.r.f, FLAG_Z)) }),
            (0x00CA_u16, Instruction { dissassembly: "JP Z,a16",    bytes: 3, closure: |cpu, ctx| cpu.op_jp_a16(ctx.bus, &mut ctx.r.pc, get_flag2(ctx.r.f, FLAG_Z)) }),
            (0x00D2_u16, Instruction { dissassembly: "JP NC,a16",   bytes: 3, closure: |cpu, ctx| cpu.op_jp_a16(ctx.bus, &mut ctx.r.pc, !get_flag2(ctx.r.f, FLAG_C)) }),
            (0x00DA_u16, Instruction { dissassembly: "JP C,a16",    bytes: 3, closure: |cpu, ctx| cpu.op_jp_a16(ctx.bus, &mut ctx.r.pc, get_flag2(ctx.r.f, FLAG_C)) }),
            (0x0018_u16, Instruction { dissassembly: "JR s8",       bytes: 2, closure: |cpu, ctx| cpu.op_jr_s8(ctx.bus, &mut ctx.r.pc, true) }),
            (0x0020_u16, Instruction { dissassembly: "JR NZ,s8",    bytes: 2, closure: |cpu, ctx| cpu.op_jr_s8(ctx.bus, &mut ctx.r.pc, !get_flag2(ctx.r.f, FLAG_Z)) }),
            (0x0028_u16, Instruction { dissassembly: "JR Z,s8",     bytes: 2, closure: |cpu, ctx| cpu.op_jr_s8(ctx.bus, &mut ctx.r.pc, get_flag2(ctx.r.f, FLAG_Z)) }),
            (0x0030_u16, Instruction { dissassembly: "JR NC,s8",    bytes: 2, closure: |cpu, ctx| cpu.op_jr_s8(ctx.bus, &mut ctx.r.pc, !get_flag2(ctx.r.f, FLAG_C)) }),
            (0x0038_u16, Instruction { dissassembly: "JR C,s8",     bytes: 2, closure: |cpu, ctx| cpu.op_jr_s8(ctx.bus, &mut ctx.r.pc, get_flag2(ctx.r.f, FLAG_C)) }),
            (0x00CD_u16, Instruction { dissassembly: "CALL a16",    bytes: 3, closure: |cpu, ctx| cpu.op_call_a16(ctx.bus, &mut ctx.r.pc, &mut ctx.r.sp, true) }),
            (0x00C4_u16, Instruction { dissassembly: "CALL NZ,a16", bytes: 3, closure: |cpu, ctx| cpu.op_call_a16(ctx.bus, &mut ctx.r.pc, &mut ctx.r.sp, !get_flag2(ctx.r.f, FLAG_Z)) }),
            (0x00CC_u16, Instruction { dissassembly: "CALL Z,a16",  bytes: 3, closure: |cpu, ctx| cpu.op_call_a16(ctx.bus, &mut ctx.r.pc, &mut ctx.r.sp, get_flag2(ctx.r.f, FLAG_Z)) }),
            (0x00D4_u16, Instruction { dissassembly: "CALL NC,a16", bytes: 3, closure: |cpu, ctx| cpu.op_call_a16(ctx.bus, &mut ctx.r.pc, &mut ctx.r.sp, !get_flag2(ctx.r.f, FLAG_C)) }),
            (0x00DC_u16, Instruction { dissassembly: "CALL C,a16",  bytes: 3, closure: |cpu, ctx| cpu.op_call_a16(ctx.bus, &mut ctx.r.pc, &mut ctx.r.sp, get_flag2(ctx.r.f, FLAG_C)) }),
            (0x00C9_u16, Instruction { dissassembly: "RET",         bytes: 1, closure: |cpu, ctx| cpu.op_ret(ctx.bus, &mut ctx.r.pc, &mut ctx.r.sp, true) }),
            (0x00C0_u16, Instruction { dissassembly: "RET NZ",      bytes: 1, closure: |cpu, ctx| cpu.op_ret(ctx.bus, &mut ctx.r.pc, &mut ctx.r.sp, !get_flag2(ctx.r.f, FLAG_Z)) }),
            (0x00C8_u16, Instruction { dissassembly: "RET Z",       bytes: 1, closure: |cpu, ctx| cpu.op_ret(ctx.bus, &mut ctx.r.pc, &mut ctx.r.sp, get_flag2(ctx.r.f, FLAG_Z)) }),
            (0x00D0_u16, Instruction { dissassembly: "RET NC",      bytes: 1, closure: |cpu, ctx| cpu.op_ret(ctx.bus, &mut ctx.r.pc, &mut ctx.r.sp, !get_flag2(ctx.r.f, FLAG_C)) }),
            (0x00D8_u16, Instruction { dissassembly: "RET C",       bytes: 1, closure: |cpu, ctx| cpu.op_ret(ctx.bus, &mut ctx.r.pc, &mut ctx.r.sp, get_flag2(ctx.r.f, FLAG_C)) }),
            (0x00D9_u16, Instruction { dissassembly: "RETI",        bytes: 1, closure: |cpu, ctx| cpu.op_reti(ctx.bus, &mut ctx.r.pc, &mut ctx.r.sp) }),
            (0x00F5_u16, Instruction { dissassembly: "PUSH AF",     bytes: 1, closure: |cpu, ctx| cpu.op_push_r16(ctx.bus, &mut ctx.r.sp, ctx.r.a, ctx.r.f) }),
            (0x00C5_u16, Instruction { dissassembly: "PUSH BC",     bytes: 1, closure: |cpu, ctx| cpu.op_push_r16(ctx.bus, &mut ctx.r.sp, ctx.r.b, ctx.r.c) }),
            (0x00D5_u16, Instruction { dissassembly: "PUSH DE",     bytes: 1, closure: |cpu, ctx| cpu.op_push_r16(ctx.bus, &mut ctx.r.sp, ctx.r.d, ctx.r.e) }),
            (0x00E5_u16, Instruction { dissassembly: "PUSH HL",     bytes: 1, closure: |cpu, ctx| cpu.op_push_r16(ctx.bus, &mut ctx.r.sp, ctx.r.h, ctx.r.l) }),
            (0x00F1_u16, Instruction { dissassembly: "POP AF",      bytes: 1, closure: |cpu, ctx| cpu.op_pop_af(ctx.bus, &mut ctx.r.sp, &mut ctx.r.a, &mut ctx.r.f) }),
            (0x00C1_u16, Instruction { dissassembly: "POP BC",      bytes: 1, closure: |cpu, ctx| cpu.op_pop_r16(ctx.bus, &mut ctx.r.sp, &mut ctx.r.b, &mut ctx.r.c) }),
            (0x00D1_u16, Instruction { dissassembly: "POP DE",      bytes: 1, closure: |cpu, ctx| cpu.op_pop_r16(ctx.bus, &mut ctx.r.sp, &mut ctx.r.d, &mut ctx.r.e) }),
            (0x00E1_u16, Instruction { dissassembly: "POP HL",      bytes: 1, closure: |cpu, ctx| cpu.op_pop_r16(ctx.bus, &mut ctx.r.sp, &mut ctx.r.h, &mut ctx.r.l) }),
            (0x00C7_u16, Instruction { dissassembly: "RST 0",       bytes: 1, closure: |cpu, ctx| cpu.op_rst_n(ctx.bus, 0, &mut ctx.r.pc, &mut ctx.r.sp) }),
            (0x00CF_u16, Instruction { dissassembly: "RST 1",       bytes: 1, closure: |cpu, ctx| cpu.op_rst_n(ctx.bus, 1, &mut ctx.r.pc, &mut ctx.r.sp) }),
            (0x00D7_u16, Instruction { dissassembly: "RST 2",       bytes: 1, closure: |cpu, ctx| cpu.op_rst_n(ctx.bus, 2, &mut ctx.r.pc, &mut ctx.r.sp) }),
            (0x00DF_u16, Instruction { dissassembly: "RST 3",       bytes: 1, closure: |cpu, ctx| cpu.op_rst_n(ctx.bus, 3, &mut ctx.r.pc, &mut ctx.r.sp) }),
            (0x00E7_u16, Instruction { dissassembly: "RST 4",       bytes: 1, closure: |cpu, ctx| cpu.op_rst_n(ctx.bus, 4, &mut ctx.r.pc, &mut ctx.r.sp) }),
            (0x00EF_u16, Instruction { dissassembly: "RST 5",       bytes: 1, closure: |cpu, ctx| cpu.op_rst_n(ctx.bus, 5, &mut ctx.r.pc, &mut ctx.r.sp) }),
            (0x00F7_u16, Instruction { dissassembly: "RST 6",       bytes: 1, closure: |cpu, ctx| cpu.op_rst_n(ctx.bus, 6, &mut ctx.r.pc, &mut ctx.r.sp) }),
            (0x00FF_u16, Instruction { dissassembly: "RST 7",       bytes: 1, closure: |cpu, ctx| cpu.op_rst_n(ctx.bus, 7, &mut ctx.r.pc, &mut ctx.r.sp) }),
            (0x00F3_u16, Instruction { dissassembly: "DI",          bytes: 1, closure: |cpu, _ctx| cpu.op_di() }),
            (0x00FB_u16, Instruction { dissassembly: "EI",          bytes: 1, closure: |cpu, _ctx| cpu.op_ei() }),
            
            // 16 bit opcodes
            (0xCB07_u16, Instruction { dissassembly: "RLC A",       bytes: 2, closure: |cpu, ctx| cpu.op_rlc_r(&mut ctx.r.a, &mut ctx.r.f) }),
            (0xCB00_u16, Instruction { dissassembly: "RLC B",       bytes: 2, closure: |cpu, ctx| cpu.op_rlc_r(&mut ctx.r.b, &mut ctx.r.f) }),
            (0xCB01_u16, Instruction { dissassembly: "RLC C",       bytes: 2, closure: |cpu, ctx| cpu.op_rlc_r(&mut ctx.r.c, &mut ctx.r.f) }),
            (0xCB02_u16, Instruction { dissassembly: "RLC D",       bytes: 2, closure: |cpu, ctx| cpu.op_rlc_r(&mut ctx.r.d, &mut ctx.r.f) }),
            (0xCB03_u16, Instruction { dissassembly: "RLC E",       bytes: 2, closure: |cpu, ctx| cpu.op_rlc_r(&mut ctx.r.e, &mut ctx.r.f) }),
            (0xCB04_u16, Instruction { dissassembly: "RLC H",       bytes: 2, closure: |cpu, ctx| cpu.op_rlc_r(&mut ctx.r.h, &mut ctx.r.f) }),
            (0xCB05_u16, Instruction { dissassembly: "RLC L",       bytes: 2, closure: |cpu, ctx| cpu.op_rlc_r(&mut ctx.r.l, &mut ctx.r.f) }),
            (0xCB06_u16, Instruction { dissassembly: "RLC (HL)",    bytes: 2, closure: |cpu, ctx| cpu.op_rlc_addr(ctx.bus, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0xCB0F_u16, Instruction { dissassembly: "RRC A",       bytes: 2, closure: |cpu, ctx| cpu.op_rrc_r(&mut ctx.r.a, &mut ctx.r.f) }),
            (0xCB08_u16, Instruction { dissassembly: "RRC B",       bytes: 2, closure: |cpu, ctx| cpu.op_rrc_r(&mut ctx.r.b, &mut ctx.r.f) }),
            (0xCB09_u16, Instruction { dissassembly: "RRC C",       bytes: 2, closure: |cpu, ctx| cpu.op_rrc_r(&mut ctx.r.c, &mut ctx.r.f) }),
            (0xCB0A_u16, Instruction { dissassembly: "RRC D",       bytes: 2, closure: |cpu, ctx| cpu.op_rrc_r(&mut ctx.r.d, &mut ctx.r.f) }),
            (0xCB0B_u16, Instruction { dissassembly: "RRC E",       bytes: 2, closure: |cpu, ctx| cpu.op_rrc_r(&mut ctx.r.e, &mut ctx.r.f) }),
            (0xCB0C_u16, Instruction { dissassembly: "RRC H",       bytes: 2, closure: |cpu, ctx| cpu.op_rrc_r(&mut ctx.r.h, &mut ctx.r.f) }),
            (0xCB0D_u16, Instruction { dissassembly: "RRC L",       bytes: 2, closure: |cpu, ctx| cpu.op_rrc_r(&mut ctx.r.l, &mut ctx.r.f) }),
            (0xCB0E_u16, Instruction { dissassembly: "RRC (HL)",    bytes: 2, closure: |cpu, ctx| cpu.op_rrc_addr(ctx.bus, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0xCB17_u16, Instruction { dissassembly: "RL A",        bytes: 2, closure: |cpu, ctx| cpu.op_rl_r(&mut ctx.r.a, &mut ctx.r.f) }),
            (0xCB10_u16, Instruction { dissassembly: "RL B",        bytes: 2, closure: |cpu, ctx| cpu.op_rl_r(&mut ctx.r.b, &mut ctx.r.f) }),
            (0xCB11_u16, Instruction { dissassembly: "RL C",        bytes: 2, closure: |cpu, ctx| cpu.op_rl_r(&mut ctx.r.c, &mut ctx.r.f) }),
            (0xCB12_u16, Instruction { dissassembly: "RL D",        bytes: 2, closure: |cpu, ctx| cpu.op_rl_r(&mut ctx.r.d, &mut ctx.r.f) }),
            (0xCB13_u16, Instruction { dissassembly: "RL E",        bytes: 2, closure: |cpu, ctx| cpu.op_rl_r(&mut ctx.r.e, &mut ctx.r.f) }),
            (0xCB14_u16, Instruction { dissassembly: "RL H",        bytes: 2, closure: |cpu, ctx| cpu.op_rl_r(&mut ctx.r.h, &mut ctx.r.f) }),
            (0xCB15_u16, Instruction { dissassembly: "RL L",        bytes: 2, closure: |cpu, ctx| cpu.op_rl_r(&mut ctx.r.l, &mut ctx.r.f) }),
            (0xCB16_u16, Instruction { dissassembly: "RL (HL)",     bytes: 2, closure: |cpu, ctx| cpu.op_rl_addr(ctx.bus, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0xCB1F_u16, Instruction { dissassembly: "RR A",        bytes: 2, closure: |cpu, ctx| cpu.op_rr_r(&mut ctx.r.a, &mut ctx.r.f) }),
            (0xCB18_u16, Instruction { dissassembly: "RR B",        bytes: 2, closure: |cpu, ctx| cpu.op_rr_r(&mut ctx.r.b, &mut ctx.r.f) }),
            (0xCB19_u16, Instruction { dissassembly: "RR C",        bytes: 2, closure: |cpu, ctx| cpu.op_rr_r(&mut ctx.r.c, &mut ctx.r.f) }),
            (0xCB1A_u16, Instruction { dissassembly: "RR D",        bytes: 2, closure: |cpu, ctx| cpu.op_rr_r(&mut ctx.r.d, &mut ctx.r.f) }),
            (0xCB1B_u16, Instruction { dissassembly: "RR E",        bytes: 2, closure: |cpu, ctx| cpu.op_rr_r(&mut ctx.r.e, &mut ctx.r.f) }),
            (0xCB1C_u16, Instruction { dissassembly: "RR H",        bytes: 2, closure: |cpu, ctx| cpu.op_rr_r(&mut ctx.r.h, &mut ctx.r.f) }),
            (0xCB1D_u16, Instruction { dissassembly: "RR L",        bytes: 2, closure: |cpu, ctx| cpu.op_rr_r(&mut ctx.r.l, &mut ctx.r.f) }),
            (0xCB1E_u16, Instruction { dissassembly: "RR (HL)",     bytes: 2, closure: |cpu, ctx| cpu.op_rr_addr(ctx.bus, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0xCB27_u16, Instruction { dissassembly: "SLA A",       bytes: 2, closure: |cpu, ctx| cpu.op_sla_r(&mut ctx.r.a, &mut ctx.r.f) }),
            (0xCB20_u16, Instruction { dissassembly: "SLA B",       bytes: 2, closure: |cpu, ctx| cpu.op_sla_r(&mut ctx.r.b, &mut ctx.r.f) }),
            (0xCB21_u16, Instruction { dissassembly: "SLA C",       bytes: 2, closure: |cpu, ctx| cpu.op_sla_r(&mut ctx.r.c, &mut ctx.r.f) }),
            (0xCB22_u16, Instruction { dissassembly: "SLA D",       bytes: 2, closure: |cpu, ctx| cpu.op_sla_r(&mut ctx.r.d, &mut ctx.r.f) }),
            (0xCB23_u16, Instruction { dissassembly: "SLA E",       bytes: 2, closure: |cpu, ctx| cpu.op_sla_r(&mut ctx.r.e, &mut ctx.r.f) }),
            (0xCB24_u16, Instruction { dissassembly: "SLA H",       bytes: 2, closure: |cpu, ctx| cpu.op_sla_r(&mut ctx.r.h, &mut ctx.r.f) }),
            (0xCB25_u16, Instruction { dissassembly: "SLA L",       bytes: 2, closure: |cpu, ctx| cpu.op_sla_r(&mut ctx.r.l, &mut ctx.r.f) }),
            (0xCB26_u16, Instruction { dissassembly: "SLA (HL)",    bytes: 2, closure: |cpu, ctx| cpu.op_sla_addr(ctx.bus, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0xCB3F_u16, Instruction { dissassembly: "SRL A",       bytes: 2, closure: |cpu, ctx| cpu.op_srl_r(&mut ctx.r.a, &mut ctx.r.f) }),
            (0xCB38_u16, Instruction { dissassembly: "SRL B",       bytes: 2, closure: |cpu, ctx| cpu.op_srl_r(&mut ctx.r.b, &mut ctx.r.f) }),
            (0xCB39_u16, Instruction { dissassembly: "SRL C",       bytes: 2, closure: |cpu, ctx| cpu.op_srl_r(&mut ctx.r.c, &mut ctx.r.f) }),
            (0xCB3A_u16, Instruction { dissassembly: "SRL D",       bytes: 2, closure: |cpu, ctx| cpu.op_srl_r(&mut ctx.r.d, &mut ctx.r.f) }),
            (0xCB3B_u16, Instruction { dissassembly: "SRL E",       bytes: 2, closure: |cpu, ctx| cpu.op_srl_r(&mut ctx.r.e, &mut ctx.r.f) }),
            (0xCB3C_u16, Instruction { dissassembly: "SRL H",       bytes: 2, closure: |cpu, ctx| cpu.op_srl_r(&mut ctx.r.h, &mut ctx.r.f) }),
            (0xCB3D_u16, Instruction { dissassembly: "SRL L",       bytes: 2, closure: |cpu, ctx| cpu.op_srl_r(&mut ctx.r.l, &mut ctx.r.f) }),
            (0xCB3E_u16, Instruction { dissassembly: "SRL (HL)",    bytes: 2, closure: |cpu, ctx| cpu.op_srl_addr(ctx.bus, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0xCB2F_u16, Instruction { dissassembly: "SRA A",       bytes: 2, closure: |cpu, ctx| cpu.op_sra_r(&mut ctx.r.a, &mut ctx.r.f) }),
            (0xCB28_u16, Instruction { dissassembly: "SRA B",       bytes: 2, closure: |cpu, ctx| cpu.op_sra_r(&mut ctx.r.b, &mut ctx.r.f) }),
            (0xCB29_u16, Instruction { dissassembly: "SRA C",       bytes: 2, closure: |cpu, ctx| cpu.op_sra_r(&mut ctx.r.c, &mut ctx.r.f) }),
            (0xCB2A_u16, Instruction { dissassembly: "SRA D",       bytes: 2, closure: |cpu, ctx| cpu.op_sra_r(&mut ctx.r.d, &mut ctx.r.f) }),
            (0xCB2B_u16, Instruction { dissassembly: "SRA E",       bytes: 2, closure: |cpu, ctx| cpu.op_sra_r(&mut ctx.r.e, &mut ctx.r.f) }),
            (0xCB2C_u16, Instruction { dissassembly: "SRA H",       bytes: 2, closure: |cpu, ctx| cpu.op_sra_r(&mut ctx.r.h, &mut ctx.r.f) }),
            (0xCB2D_u16, Instruction { dissassembly: "SRA L",       bytes: 2, closure: |cpu, ctx| cpu.op_sra_r(&mut ctx.r.l, &mut ctx.r.f) }),
            (0xCB2E_u16, Instruction { dissassembly: "SRA (HL)",    bytes: 2, closure: |cpu, ctx| cpu.op_sra_addr(ctx.bus, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            
            (0xCB37_u16, Instruction { dissassembly: "SWAP A",      bytes: 2, closure: |cpu, ctx| cpu.op_swap_r(&mut ctx.r.a, &mut ctx.r.f) }),
            (0xCB30_u16, Instruction { dissassembly: "SWAP B",      bytes: 2, closure: |cpu, ctx| cpu.op_swap_r(&mut ctx.r.b, &mut ctx.r.f) }),
            (0xCB31_u16, Instruction { dissassembly: "SWAP C",      bytes: 2, closure: |cpu, ctx| cpu.op_swap_r(&mut ctx.r.c, &mut ctx.r.f) }),
            (0xCB32_u16, Instruction { dissassembly: "SWAP D",      bytes: 2, closure: |cpu, ctx| cpu.op_swap_r(&mut ctx.r.d, &mut ctx.r.f) }),
            (0xCB33_u16, Instruction { dissassembly: "SWAP E",      bytes: 2, closure: |cpu, ctx| cpu.op_swap_r(&mut ctx.r.e, &mut ctx.r.f) }),
            (0xCB34_u16, Instruction { dissassembly: "SWAP H",      bytes: 2, closure: |cpu, ctx| cpu.op_swap_r(&mut ctx.r.h, &mut ctx.r.f) }),
            (0xCB35_u16, Instruction { dissassembly: "SWAP L",      bytes: 2, closure: |cpu, ctx| cpu.op_swap_r(&mut ctx.r.l, &mut ctx.r.f) }),
            (0xCB36_u16, Instruction { dissassembly: "SWAP (HL)",   bytes: 2, closure: |cpu, ctx| cpu.op_swap_addr(ctx.bus, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }), // func: CPU::op_swap_mem_hl }),

            (0xCB47_u16, Instruction { dissassembly: "BIT 0,A",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(0, ctx.r.a, &mut ctx.r.f) }),
            (0xCB40_u16, Instruction { dissassembly: "BIT 0,B",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(0, ctx.r.b, &mut ctx.r.f) }),
            (0xCB41_u16, Instruction { dissassembly: "BIT 0,C",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(0, ctx.r.c, &mut ctx.r.f) }),
            (0xCB42_u16, Instruction { dissassembly: "BIT 0,D",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(0, ctx.r.d, &mut ctx.r.f) }),
            (0xCB43_u16, Instruction { dissassembly: "BIT 0,E",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(0, ctx.r.e, &mut ctx.r.f) }),
            (0xCB44_u16, Instruction { dissassembly: "BIT 0,H",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(0, ctx.r.h, &mut ctx.r.f) }),
            (0xCB45_u16, Instruction { dissassembly: "BIT 0,L",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(0, ctx.r.l, &mut ctx.r.f) }),
            (0xCB4F_u16, Instruction { dissassembly: "BIT 1,A",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(1, ctx.r.a, &mut ctx.r.f) }),
            (0xCB48_u16, Instruction { dissassembly: "BIT 1,B",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(1, ctx.r.b, &mut ctx.r.f) }),
            (0xCB49_u16, Instruction { dissassembly: "BIT 1,C",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(1, ctx.r.c, &mut ctx.r.f) }),
            (0xCB4A_u16, Instruction { dissassembly: "BIT 1,D",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(1, ctx.r.d, &mut ctx.r.f) }),
            (0xCB4B_u16, Instruction { dissassembly: "BIT 1,E",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(1, ctx.r.e, &mut ctx.r.f) }),
            (0xCB4C_u16, Instruction { dissassembly: "BIT 1,H",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(1, ctx.r.h, &mut ctx.r.f) }),
            (0xCB4D_u16, Instruction { dissassembly: "BIT 1,L",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(1, ctx.r.l, &mut ctx.r.f) }),
            (0xCB57_u16, Instruction { dissassembly: "BIT 2,A",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(2, ctx.r.a, &mut ctx.r.f) }),
            (0xCB50_u16, Instruction { dissassembly: "BIT 2,B",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(2, ctx.r.b, &mut ctx.r.f) }),
            (0xCB51_u16, Instruction { dissassembly: "BIT 2,C",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(2, ctx.r.c, &mut ctx.r.f) }),
            (0xCB52_u16, Instruction { dissassembly: "BIT 2,D",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(2, ctx.r.d, &mut ctx.r.f) }),
            (0xCB53_u16, Instruction { dissassembly: "BIT 2,E",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(2, ctx.r.e, &mut ctx.r.f) }),
            (0xCB54_u16, Instruction { dissassembly: "BIT 2,H",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(2, ctx.r.h, &mut ctx.r.f) }),
            (0xCB55_u16, Instruction { dissassembly: "BIT 2,L",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(2, ctx.r.l, &mut ctx.r.f) }),
            (0xCB5F_u16, Instruction { dissassembly: "BIT 3,A",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(3, ctx.r.a, &mut ctx.r.f) }),
            (0xCB58_u16, Instruction { dissassembly: "BIT 3,B",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(3, ctx.r.b, &mut ctx.r.f) }),
            (0xCB59_u16, Instruction { dissassembly: "BIT 3,C",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(3, ctx.r.c, &mut ctx.r.f) }),
            (0xCB5A_u16, Instruction { dissassembly: "BIT 3,D",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(3, ctx.r.d, &mut ctx.r.f) }),
            (0xCB5B_u16, Instruction { dissassembly: "BIT 3,E",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(3, ctx.r.e, &mut ctx.r.f) }),
            (0xCB5C_u16, Instruction { dissassembly: "BIT 3,H",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(3, ctx.r.h, &mut ctx.r.f) }),
            (0xCB5D_u16, Instruction { dissassembly: "BIT 3,L",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(3, ctx.r.l, &mut ctx.r.f) }),
            (0xCB67_u16, Instruction { dissassembly: "BIT 4,A",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(4, ctx.r.a, &mut ctx.r.f) }),
            (0xCB60_u16, Instruction { dissassembly: "BIT 4,B",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(4, ctx.r.b, &mut ctx.r.f) }),
            (0xCB61_u16, Instruction { dissassembly: "BIT 4,C",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(4, ctx.r.c, &mut ctx.r.f) }),
            (0xCB62_u16, Instruction { dissassembly: "BIT 4,D",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(4, ctx.r.d, &mut ctx.r.f) }),
            (0xCB63_u16, Instruction { dissassembly: "BIT 4,E",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(4, ctx.r.e, &mut ctx.r.f) }),
            (0xCB64_u16, Instruction { dissassembly: "BIT 4,H",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(4, ctx.r.h, &mut ctx.r.f) }),
            (0xCB65_u16, Instruction { dissassembly: "BIT 4,L",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(4, ctx.r.l, &mut ctx.r.f) }),
            (0xCB6F_u16, Instruction { dissassembly: "BIT 5,A",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(5, ctx.r.a, &mut ctx.r.f) }),
            (0xCB68_u16, Instruction { dissassembly: "BIT 5,B",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(5, ctx.r.b, &mut ctx.r.f) }),
            (0xCB69_u16, Instruction { dissassembly: "BIT 5,C",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(5, ctx.r.c, &mut ctx.r.f) }),
            (0xCB6A_u16, Instruction { dissassembly: "BIT 5,D",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(5, ctx.r.d, &mut ctx.r.f) }),
            (0xCB6B_u16, Instruction { dissassembly: "BIT 5,E",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(5, ctx.r.e, &mut ctx.r.f) }),
            (0xCB6C_u16, Instruction { dissassembly: "BIT 5,H",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(5, ctx.r.h, &mut ctx.r.f) }),
            (0xCB6D_u16, Instruction { dissassembly: "BIT 5,L",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(5, ctx.r.l, &mut ctx.r.f) }),
            (0xCB77_u16, Instruction { dissassembly: "BIT 6,A",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(6, ctx.r.a, &mut ctx.r.f) }),
            (0xCB70_u16, Instruction { dissassembly: "BIT 6,B",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(6, ctx.r.b, &mut ctx.r.f) }),
            (0xCB71_u16, Instruction { dissassembly: "BIT 6,C",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(6, ctx.r.c, &mut ctx.r.f) }),
            (0xCB72_u16, Instruction { dissassembly: "BIT 6,D",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(6, ctx.r.d, &mut ctx.r.f) }),
            (0xCB73_u16, Instruction { dissassembly: "BIT 6,E",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(6, ctx.r.e, &mut ctx.r.f) }),
            (0xCB74_u16, Instruction { dissassembly: "BIT 6,H",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(6, ctx.r.h, &mut ctx.r.f) }),
            (0xCB75_u16, Instruction { dissassembly: "BIT 6,L",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(6, ctx.r.l, &mut ctx.r.f) }),
            (0xCB7F_u16, Instruction { dissassembly: "BIT 7,A",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(7, ctx.r.a, &mut ctx.r.f) }),
            (0xCB78_u16, Instruction { dissassembly: "BIT 7,B",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(7, ctx.r.b, &mut ctx.r.f) }),
            (0xCB79_u16, Instruction { dissassembly: "BIT 7,C",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(7, ctx.r.c, &mut ctx.r.f) }),
            (0xCB7A_u16, Instruction { dissassembly: "BIT 7,D",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(7, ctx.r.d, &mut ctx.r.f) }),
            (0xCB7B_u16, Instruction { dissassembly: "BIT 7,E",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(7, ctx.r.e, &mut ctx.r.f) }),
            (0xCB7C_u16, Instruction { dissassembly: "BIT 7,H",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(7, ctx.r.h, &mut ctx.r.f) }),
            (0xCB7D_u16, Instruction { dissassembly: "BIT 7,L",     bytes: 2, closure: |cpu, ctx| cpu.op_bitn_r(7, ctx.r.l, &mut ctx.r.f) }),
            (0xCB46_u16, Instruction { dissassembly: "BIT 0,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_bitn_addr(ctx.bus, 0, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0xCB4E_u16, Instruction { dissassembly: "BIT 1,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_bitn_addr(ctx.bus, 1, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0xCB56_u16, Instruction { dissassembly: "BIT 2,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_bitn_addr(ctx.bus, 2, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0xCB5E_u16, Instruction { dissassembly: "BIT 3,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_bitn_addr(ctx.bus, 3, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0xCB66_u16, Instruction { dissassembly: "BIT 4,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_bitn_addr(ctx.bus, 4, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0xCB6E_u16, Instruction { dissassembly: "BIT 5,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_bitn_addr(ctx.bus, 5, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0xCB76_u16, Instruction { dissassembly: "BIT 6,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_bitn_addr(ctx.bus, 6, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),
            (0xCB7E_u16, Instruction { dissassembly: "BIT 7,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_bitn_addr(ctx.bus, 7, to_u16(ctx.r.h, ctx.r.l), &mut ctx.r.f) }),

            (0xCBC7_u16, Instruction { dissassembly: "SET 0,A",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(0, &mut ctx.r.a) }),
            (0xCBC0_u16, Instruction { dissassembly: "SET 0,B",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(0, &mut ctx.r.b) }),
            (0xCBC1_u16, Instruction { dissassembly: "SET 0,C",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(0, &mut ctx.r.c) }),
            (0xCBC2_u16, Instruction { dissassembly: "SET 0,D",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(0, &mut ctx.r.d) }),
            (0xCBC3_u16, Instruction { dissassembly: "SET 0,E",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(0, &mut ctx.r.e) }),
            (0xCBC4_u16, Instruction { dissassembly: "SET 0,H",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(0, &mut ctx.r.h) }),
            (0xCBC5_u16, Instruction { dissassembly: "SET 0,L",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(0, &mut ctx.r.l) }),
            (0xCBCF_u16, Instruction { dissassembly: "SET 1,A",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(1, &mut ctx.r.a) }),
            (0xCBC8_u16, Instruction { dissassembly: "SET 1,B",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(1, &mut ctx.r.b) }),
            (0xCBC9_u16, Instruction { dissassembly: "SET 1,C",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(1, &mut ctx.r.c) }),
            (0xCBCA_u16, Instruction { dissassembly: "SET 1,D",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(1, &mut ctx.r.d) }),
            (0xCBCB_u16, Instruction { dissassembly: "SET 1,E",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(1, &mut ctx.r.e) }),
            (0xCBCC_u16, Instruction { dissassembly: "SET 1,H",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(1, &mut ctx.r.h) }),
            (0xCBCD_u16, Instruction { dissassembly: "SET 1,L",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(1, &mut ctx.r.l) }),
            (0xCBD7_u16, Instruction { dissassembly: "SET 2,A",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(2, &mut ctx.r.a) }),
            (0xCBD0_u16, Instruction { dissassembly: "SET 2,B",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(2, &mut ctx.r.b) }),
            (0xCBD1_u16, Instruction { dissassembly: "SET 2,C",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(2, &mut ctx.r.c) }),
            (0xCBD2_u16, Instruction { dissassembly: "SET 2,D",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(2, &mut ctx.r.d) }),
            (0xCBD3_u16, Instruction { dissassembly: "SET 2,E",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(2, &mut ctx.r.e) }),
            (0xCBD4_u16, Instruction { dissassembly: "SET 2,H",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(2, &mut ctx.r.h) }),
            (0xCBD5_u16, Instruction { dissassembly: "SET 2,L",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(2, &mut ctx.r.l) }),
            (0xCBDF_u16, Instruction { dissassembly: "SET 3,A",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(3, &mut ctx.r.a) }),
            (0xCBD8_u16, Instruction { dissassembly: "SET 3,B",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(3, &mut ctx.r.b) }),
            (0xCBD9_u16, Instruction { dissassembly: "SET 3,C",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(3, &mut ctx.r.c) }),
            (0xCBDA_u16, Instruction { dissassembly: "SET 3,D",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(3, &mut ctx.r.d) }),
            (0xCBDB_u16, Instruction { dissassembly: "SET 3,E",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(3, &mut ctx.r.e) }),
            (0xCBDC_u16, Instruction { dissassembly: "SET 3,H",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(3, &mut ctx.r.h) }),
            (0xCBDD_u16, Instruction { dissassembly: "SET 3,L",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(3, &mut ctx.r.l) }),
            (0xCBE7_u16, Instruction { dissassembly: "SET 4,A",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(4, &mut ctx.r.a) }),
            (0xCBE0_u16, Instruction { dissassembly: "SET 4,B",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(4, &mut ctx.r.b) }),
            (0xCBE1_u16, Instruction { dissassembly: "SET 4,C",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(4, &mut ctx.r.c) }),
            (0xCBE2_u16, Instruction { dissassembly: "SET 4,D",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(4, &mut ctx.r.d) }),
            (0xCBE3_u16, Instruction { dissassembly: "SET 4,E",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(4, &mut ctx.r.e) }),
            (0xCBE4_u16, Instruction { dissassembly: "SET 4,H",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(4, &mut ctx.r.h) }),
            (0xCBE5_u16, Instruction { dissassembly: "SET 4,L",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(4, &mut ctx.r.l) }),
            (0xCBEF_u16, Instruction { dissassembly: "SET 5,A",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(5, &mut ctx.r.a) }),
            (0xCBE8_u16, Instruction { dissassembly: "SET 5,B",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(5, &mut ctx.r.b) }),
            (0xCBE9_u16, Instruction { dissassembly: "SET 5,C",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(5, &mut ctx.r.c) }),
            (0xCBEA_u16, Instruction { dissassembly: "SET 5,D",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(5, &mut ctx.r.d) }),
            (0xCBEB_u16, Instruction { dissassembly: "SET 5,E",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(5, &mut ctx.r.e) }),
            (0xCBEC_u16, Instruction { dissassembly: "SET 5,H",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(5, &mut ctx.r.h) }),
            (0xCBED_u16, Instruction { dissassembly: "SET 5,L",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(5, &mut ctx.r.l) }),
            (0xCBF7_u16, Instruction { dissassembly: "SET 6,A",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(6, &mut ctx.r.a) }),
            (0xCBF0_u16, Instruction { dissassembly: "SET 6,B",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(6, &mut ctx.r.b) }),
            (0xCBF1_u16, Instruction { dissassembly: "SET 6,C",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(6, &mut ctx.r.c) }),
            (0xCBF2_u16, Instruction { dissassembly: "SET 6,D",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(6, &mut ctx.r.d) }),
            (0xCBF3_u16, Instruction { dissassembly: "SET 6,E",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(6, &mut ctx.r.e) }),
            (0xCBF4_u16, Instruction { dissassembly: "SET 6,H",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(6, &mut ctx.r.h) }),
            (0xCBF5_u16, Instruction { dissassembly: "SET 6,L",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(6, &mut ctx.r.l) }),
            (0xCBFF_u16, Instruction { dissassembly: "SET 7,A",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(7, &mut ctx.r.a) }),
            (0xCBF8_u16, Instruction { dissassembly: "SET 7,B",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(7, &mut ctx.r.b) }),
            (0xCBF9_u16, Instruction { dissassembly: "SET 7,C",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(7, &mut ctx.r.c) }),
            (0xCBFA_u16, Instruction { dissassembly: "SET 7,D",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(7, &mut ctx.r.d) }),
            (0xCBFB_u16, Instruction { dissassembly: "SET 7,E",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(7, &mut ctx.r.e) }),
            (0xCBFC_u16, Instruction { dissassembly: "SET 7,H",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(7, &mut ctx.r.h) }),
            (0xCBFD_u16, Instruction { dissassembly: "SET 7,L",     bytes: 2, closure: |cpu, ctx| cpu.op_setn_r(7, &mut ctx.r.l) }),
            (0xCB87_u16, Instruction { dissassembly: "RES 0,A",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(0, &mut ctx.r.a) }),
            (0xCB80_u16, Instruction { dissassembly: "RES 0,B",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(0, &mut ctx.r.b) }),
            (0xCB81_u16, Instruction { dissassembly: "RES 0,C",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(0, &mut ctx.r.c) }),
            (0xCB82_u16, Instruction { dissassembly: "RES 0,D",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(0, &mut ctx.r.d) }),
            (0xCB83_u16, Instruction { dissassembly: "RES 0,E",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(0, &mut ctx.r.e) }),
            (0xCB84_u16, Instruction { dissassembly: "RES 0,H",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(0, &mut ctx.r.h) }),
            (0xCB85_u16, Instruction { dissassembly: "RES 0,L",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(0, &mut ctx.r.l) }),
            (0xCB8F_u16, Instruction { dissassembly: "RES 1,A",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(1, &mut ctx.r.a) }),
            (0xCB88_u16, Instruction { dissassembly: "RES 1,B",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(1, &mut ctx.r.b) }),
            (0xCB89_u16, Instruction { dissassembly: "RES 1,C",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(1, &mut ctx.r.c) }),
            (0xCB8A_u16, Instruction { dissassembly: "RES 1,D",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(1, &mut ctx.r.d) }),
            (0xCB8B_u16, Instruction { dissassembly: "RES 1,E",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(1, &mut ctx.r.e) }),
            (0xCB8C_u16, Instruction { dissassembly: "RES 1,H",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(1, &mut ctx.r.h) }),
            (0xCB8D_u16, Instruction { dissassembly: "RES 1,L",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(1, &mut ctx.r.l) }),
            (0xCB97_u16, Instruction { dissassembly: "RES 2,A",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(2, &mut ctx.r.a) }),
            (0xCB90_u16, Instruction { dissassembly: "RES 2,B",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(2, &mut ctx.r.b) }),
            (0xCB91_u16, Instruction { dissassembly: "RES 2,C",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(2, &mut ctx.r.c) }),
            (0xCB92_u16, Instruction { dissassembly: "RES 2,D",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(2, &mut ctx.r.d) }),
            (0xCB93_u16, Instruction { dissassembly: "RES 2,E",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(2, &mut ctx.r.e) }),
            (0xCB94_u16, Instruction { dissassembly: "RES 2,H",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(2, &mut ctx.r.h) }),
            (0xCB95_u16, Instruction { dissassembly: "RES 2,L",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(2, &mut ctx.r.l) }),
            (0xCB9F_u16, Instruction { dissassembly: "RES 3,A",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(3, &mut ctx.r.a) }),
            (0xCB98_u16, Instruction { dissassembly: "RES 3,B",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(3, &mut ctx.r.b) }),
            (0xCB99_u16, Instruction { dissassembly: "RES 3,C",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(3, &mut ctx.r.c) }),
            (0xCB9A_u16, Instruction { dissassembly: "RES 3,D",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(3, &mut ctx.r.d) }),
            (0xCB9B_u16, Instruction { dissassembly: "RES 3,E",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(3, &mut ctx.r.e) }),
            (0xCB9C_u16, Instruction { dissassembly: "RES 3,H",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(3, &mut ctx.r.h) }),
            (0xCB9D_u16, Instruction { dissassembly: "RES 3,L",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(3, &mut ctx.r.l) }),
            (0xCBA7_u16, Instruction { dissassembly: "RES 4,A",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(4, &mut ctx.r.a) }),
            (0xCBA0_u16, Instruction { dissassembly: "RES 4,B",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(4, &mut ctx.r.b) }),
            (0xCBA1_u16, Instruction { dissassembly: "RES 4,C",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(4, &mut ctx.r.c) }),
            (0xCBA2_u16, Instruction { dissassembly: "RES 4,D",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(4, &mut ctx.r.d) }),
            (0xCBA3_u16, Instruction { dissassembly: "RES 4,E",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(4, &mut ctx.r.e) }),
            (0xCBA4_u16, Instruction { dissassembly: "RES 4,H",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(4, &mut ctx.r.h) }),
            (0xCBA5_u16, Instruction { dissassembly: "RES 4,L",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(4, &mut ctx.r.l) }),
            (0xCBAF_u16, Instruction { dissassembly: "RES 5,A",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(5, &mut ctx.r.a) }),
            (0xCBA8_u16, Instruction { dissassembly: "RES 5,B",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(5, &mut ctx.r.b) }),
            (0xCBA9_u16, Instruction { dissassembly: "RES 5,C",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(5, &mut ctx.r.c) }),
            (0xCBAA_u16, Instruction { dissassembly: "RES 5,D",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(5, &mut ctx.r.d) }),
            (0xCBAB_u16, Instruction { dissassembly: "RES 5,E",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(5, &mut ctx.r.e) }),
            (0xCBAC_u16, Instruction { dissassembly: "RES 5,H",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(5, &mut ctx.r.h) }),
            (0xCBAD_u16, Instruction { dissassembly: "RES 5,L",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(5, &mut ctx.r.l) }),
            (0xCBB7_u16, Instruction { dissassembly: "RES 6,A",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(6, &mut ctx.r.a) }),
            (0xCBB0_u16, Instruction { dissassembly: "RES 6,B",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(6, &mut ctx.r.b) }),
            (0xCBB1_u16, Instruction { dissassembly: "RES 6,C",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(6, &mut ctx.r.c) }),
            (0xCBB2_u16, Instruction { dissassembly: "RES 6,D",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(6, &mut ctx.r.d) }),
            (0xCBB3_u16, Instruction { dissassembly: "RES 6,E",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(6, &mut ctx.r.e) }),
            (0xCBB4_u16, Instruction { dissassembly: "RES 6,H",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(6, &mut ctx.r.h) }),
            (0xCBB5_u16, Instruction { dissassembly: "RES 6,L",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(6, &mut ctx.r.l) }),
            (0xCBBF_u16, Instruction { dissassembly: "RES 7,A",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(7, &mut ctx.r.a) }),
            (0xCBB8_u16, Instruction { dissassembly: "RES 7,B",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(7, &mut ctx.r.b) }),
            (0xCBB9_u16, Instruction { dissassembly: "RES 7,C",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(7, &mut ctx.r.c) }),
            (0xCBBA_u16, Instruction { dissassembly: "RES 7,D",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(7, &mut ctx.r.d) }),
            (0xCBBB_u16, Instruction { dissassembly: "RES 7,E",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(7, &mut ctx.r.e) }),
            (0xCBBC_u16, Instruction { dissassembly: "RES 7,H",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(7, &mut ctx.r.h) }),
            (0xCBBD_u16, Instruction { dissassembly: "RES 7,L",     bytes: 2, closure: |cpu, ctx| cpu.op_resn_r(7, &mut ctx.r.l) }),
            (0xCB86_u16, Instruction { dissassembly: "RES 0,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_resn_addr(ctx.bus, 0, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCB8E_u16, Instruction { dissassembly: "RES 1,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_resn_addr(ctx.bus, 1, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCB96_u16, Instruction { dissassembly: "RES 2,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_resn_addr(ctx.bus, 2, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCB9E_u16, Instruction { dissassembly: "RES 3,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_resn_addr(ctx.bus, 3, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCBA6_u16, Instruction { dissassembly: "RES 4,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_resn_addr(ctx.bus, 4, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCBAE_u16, Instruction { dissassembly: "RES 5,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_resn_addr(ctx.bus, 5, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCBB6_u16, Instruction { dissassembly: "RES 6,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_resn_addr(ctx.bus, 6, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCBBE_u16, Instruction { dissassembly: "RES 7,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_resn_addr(ctx.bus, 7, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCBC6_u16, Instruction { dissassembly: "SET 0,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_setn_addr(ctx.bus, 0, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCBCE_u16, Instruction { dissassembly: "SET 1,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_setn_addr(ctx.bus, 1, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCBD6_u16, Instruction { dissassembly: "SET 2,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_setn_addr(ctx.bus, 2, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCBDE_u16, Instruction { dissassembly: "SET 3,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_setn_addr(ctx.bus, 3, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCBE6_u16, Instruction { dissassembly: "SET 4,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_setn_addr(ctx.bus, 4, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCBEE_u16, Instruction { dissassembly: "SET 5,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_setn_addr(ctx.bus, 5, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCBF6_u16, Instruction { dissassembly: "SET 6,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_setn_addr(ctx.bus, 6, to_u16(ctx.r.h, ctx.r.l)) }),
            (0xCBFE_u16, Instruction { dissassembly: "SET 7,(HL)",  bytes: 2, closure: |cpu, ctx| cpu.op_setn_addr(ctx.bus, 7, to_u16(ctx.r.h, ctx.r.l)) }),
        ].iter().cloned().collect();

        Self {
            model,
            instructions: instruction_table,
            registers: RefCell::new(Registers { 
                a: 0x00, f: 0x00,
                b: 0x00, c: 0x00,
                d: 0x00, e: 0x00,
                h: 0x00, l: 0x00,
                sp: 0x0000,
                pc: 0x0000
            }),
            state: RefCell::new(CPUState {
                mode: CPUMode::Normal,
                next_op: 0x0000,
            }),
            interrupts: RefCell::new(InterruptRegisters {
                interrupts_enabled: false,
                interrupts_enable_request: false,
                flags: 0xE1,
                enabled: 0x00
            }),
        }
    }

    pub fn set_initial_state(&mut self, skip_bootrom: bool) {
        if skip_bootrom {
            let mut registers = self.registers.borrow_mut();

            match self.model {
                GameBoyModel::DMG => {
                    registers.a = 0x01;
                    registers.f = 0xB0;
                    registers.d = 0x00;
                    registers.e = 0xD8;
                    registers.h = 0x01;
                    registers.l = 0x4D;
                    registers.pc = 0x100;
                    registers.sp = 0xFFFE;
                },

                GameBoyModel::GBC => {
                    registers.a = 0x11;
                    registers.f = 0x80;
                    registers.d = 0x00;
                    registers.e = 0x08;
                    registers.h = 0x00;
                    registers.l = 0x7C;
                    registers.pc = 0x100;
                    registers.sp = 0xFFFE;
                }
            }            
        }
    }

    pub fn get_debug_state(&self) -> CPUDebugState {
        let registers = self.registers.borrow();

        CPUDebugState {
            af: to_u16(registers.a, registers.f),
            bc: to_u16(registers.b, registers.c),
            de: to_u16(registers.d, registers.e),
            hl: to_u16(registers.h, registers.l),
            sp: registers.sp,
            pc: registers.pc,
            next_opcode: self.state.borrow().next_op,
        }
    }

    pub fn tick(&self, bus: &MemoryBus) -> u8 {
        let mut cycles = 0;

        cycles += self.dispatch_interrupts(bus);

        if self.state.borrow().mode == CPUMode::Normal {
            let mut registers = self.registers.borrow_mut();

            let op : u16 = self.read_next_instruction(bus, &mut registers.pc, true);

            // if !self.instructions.contains_key(&op) {
            //     panic!("Undefined instruction: @{:#06x} {:#04x}", pc, op);
            // }

            let inst : &Instruction = &(self.instructions[&op]);        
            let func = inst.closure;

            // call the instruction
            cycles += func(self, InstructionContext {
                bus,
                r: &mut registers
            });

            // prefetch the next instruction
            self.state.borrow_mut().next_op = self.read_next_instruction(bus, &mut registers.pc, false);
        }

        if cycles == 0 { 1 } else { cycles } 
    }

    fn read_next_instruction(&self, bus: &MemoryBus, pc: &mut u16, advance_pc: bool) -> u16 {
        let b1 = bus.read_byte(*pc);
        if advance_pc {
            *pc += 1;
        }
        
        if b1 != 0xCB {
            b1 as u16
        }
        else {
            let b2: u8 = bus.read_byte(*pc);
            if advance_pc {
                *pc += 1;
            }

            (b1 as u16) << 8 | (b2 as u16)
        }
    }

    fn dispatch_interrupts(&self, bus: &MemoryBus) -> u8 {
        let mut cycles = 0;
        let mut interrupts = self.interrupts.borrow_mut();
        let mut registers = self.registers.borrow_mut();
        let mut state = self.state.borrow_mut();

        let masked_interrupts = interrupts.enabled & interrupts.flags & 0x1F;

        // if halted and an interrupt is triggered, exit halt even if IME=0 (4 clocks)
        if state.mode == CPUMode::Halt && masked_interrupts != 0 {
            state.mode = CPUMode::Normal;
            cycles += 1;
        }

        // if IME=1 and IF and IE are enabled, do the interrupt dispatch (20 clocks)
        if interrupts.interrupts_enabled && masked_interrupts != 0 {
            if (1 << Interrupts::VBlank as u8) & masked_interrupts != 0 {
                self.execute_interrupt(bus, Interrupts::VBlank, &mut interrupts, &mut registers);
            }
            else if(1 << Interrupts::LCDStat as u8) & masked_interrupts != 0 {
                self.execute_interrupt(bus, Interrupts::LCDStat, &mut interrupts, &mut registers);
            }
            else if(1 << Interrupts::Timer as u8) & masked_interrupts != 0 {
                self.execute_interrupt(bus, Interrupts::Timer, &mut interrupts, &mut registers);
            }
            else if(1 << Interrupts::Serial as u8) & masked_interrupts != 0 {
                self.execute_interrupt(bus, Interrupts::Serial, &mut interrupts, &mut registers);
            }
            else if(1 << Interrupts::Joypad as u8) & masked_interrupts != 0 {
                self.execute_interrupt(bus, Interrupts::Joypad, &mut interrupts, &mut registers);
            }

            cycles += 5;
        }

        // when EI is called, we don't enable interrupts, instead we do this here, after checking
        // and interrupts will be enabled after the next cycle
        if interrupts.interrupts_enable_request {
            interrupts.interrupts_enable_request = false;
            interrupts.interrupts_enabled = true;
        }

        cycles
    }

    fn execute_interrupt(&self, bus: &MemoryBus, interrupt : Interrupts, interrupts: &mut InterruptRegisters, registers: &mut Registers) {
        interrupts.interrupts_enabled = false;

        registers.sp = registers.sp.wrapping_sub(2);
        self.write_word(bus, registers.sp, registers.pc);

        registers.pc = INTERRUPT_ADDRESS[interrupt as usize];

        interrupts.flags &= !(1 << interrupt as u8);
    }

    fn read_byte_from_pc(&self, bus: &MemoryBus, pc: &mut u16) -> u8 {
        let b = bus.read_byte(*pc);
        *pc += 1;
        
        b
    }

    fn read_word_from_pc(&self, bus: &MemoryBus, pc: &mut u16) -> u16 {
        self.read_byte_from_pc(bus, pc) as u16 | ((self.read_byte_from_pc(bus, pc) as u16) << 8)
    }

    fn write_word(&self, bus: &MemoryBus, address: u16, data: u16) {
        bus.write_byte(address, (data & 0xFF) as u8);
        bus.write_byte(address + 1, ((data & 0xFF00) >> 8) as u8);
    }

    // INSTRUCTIONS

    fn op_nop(&self) -> u8 {
        1
    }

    fn op_stop(&self) -> u8 {
        // TODO: P10-P13 should be LOW
        let mut state = self.state.borrow_mut();

        if self.interrupts.borrow().enabled == 0 {
            state.mode = CPUMode::Stop;
        }

        1
    }

    fn op_halt(&self) -> u8 {
        let mut state = self.state.borrow_mut();
        let interrupts = self.interrupts.borrow();
        let masked_interrupts = interrupts.enabled & interrupts.flags & 0x1f;

        if masked_interrupts == 0 {
            state.mode = CPUMode::Halt;
        }

        1
    }

    fn op_inc_r(&self, reg: &mut u8, flags: &mut u8) -> u8 {
        *reg = (*reg).wrapping_add(1);

        set_flag2(flags, FLAG_Z, *reg == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, *reg & 0x0F == 0);

        1
    }

    fn op_inc_r16(&self, reg_hi: &mut u8, reg_lo: &mut u8) -> u8 {
        let mut reg: u16 = (*reg_hi as u16) << 8 | (*reg_lo as u16);
        reg = reg.wrapping_add(1);

        *reg_hi = (reg >> 8) as u8;
        *reg_lo = reg as u8;

        2
    }

    fn op_inc_sp(&self, r: &mut u16) -> u8 {
        *r = (*r).wrapping_add(1);

        2
    }

    fn op_inc_addr(&self, bus: &MemoryBus, addr: u16, flags: &mut u8) -> u8 {
        let v = bus.read_byte(addr);

        let is_half_carry = is_half_carry(&v, &1);
        let r = v.wrapping_add(1);
        bus.write_byte(addr, r);

        set_flag2(flags, FLAG_Z, r == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, is_half_carry);

        3
    }

    fn op_dec_r(&self, reg: &mut u8, flags: &mut u8) -> u8 {
        *reg = (*reg).wrapping_sub(1);

        set_flag2(flags, FLAG_Z, *reg == 0);
        set_flag2(flags, FLAG_N, true);
        set_flag2(flags, FLAG_H, *reg & 0x0F == 0x0F);

        1
    }  

    fn op_dec_r16(&self, hi: &mut u8, lo: &mut u8) -> u8 {
        let mut reg: u16 = (*hi as u16) << 8 | (*lo as u16);
        reg = reg.wrapping_sub(1);

        *hi = (reg >> 8) as u8;
        *lo = reg as u8;

        2
    }

    fn op_dec_sp(&self, reg: &mut u16) -> u8 {
        *reg = (*reg).wrapping_sub(1);

        2
    }
    
    fn op_dec_addr(&self, bus: &MemoryBus, addr: u16, flags: &mut u8) -> u8 {
        let v = bus.read_byte(addr).wrapping_sub(1);
        bus.write_byte(addr, v);

        set_flag2(flags, FLAG_Z, v == 0);
        set_flag2(flags, FLAG_N, true);
        set_flag2(flags, FLAG_H, v & 0x0F == 0x0F);

        3
    }

    fn op_add_r(&self, accum: &mut u8, reg: u8, flags: &mut u8) -> u8 {
        let is_half_carry = is_half_carry(accum, &reg);
        let is_full_carry = is_full_carry(accum, &reg);
    
        *accum = (*accum).wrapping_add(reg);
    
        set_flag2(flags, FLAG_Z, *accum == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, is_half_carry);
        set_flag2(flags, FLAG_C, is_full_carry);
    
        1
    }

    fn op_add_d8(&self, bus: &MemoryBus, accum: &mut u8, pc: &mut u16, flags: &mut u8) -> u8 {
        let d8 = self.read_byte_from_pc(bus, pc);
        self.op_add_r(accum, d8, flags);

        2
    }

    fn op_add_addr(&self, bus: &MemoryBus, accum: &mut u8, addr: u16, flags: &mut u8) -> u8 {
        let v = bus.read_byte(addr);
        self.op_add_r(accum, v, flags);

        2
    }

    fn op_add_r16(&self, dhi: &mut u8, dlo: &mut u8, v: u16, flags: &mut u8) -> u8 {
        let mut reg = ((*dhi as u16) << 8) | *dlo as u16;
        
        let is_half_carry = is_half_carry16(&reg, &v);
        let is_full_carry = is_full_carry16(&reg, &v);
    
        reg = reg.wrapping_add(v);
        *dhi = (reg >> 8) as u8;
        *dlo = reg as u8;
    
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, is_half_carry);
        set_flag2(flags, FLAG_C, is_full_carry);

        2
    }

    fn op_add_sp_s8(&self, bus: &MemoryBus, sp: &mut u16, pc: &mut u16, flags: &mut u8) -> u8 {
        let s8: i8 = self.read_byte_from_pc(bus, pc) as i8;
        
        let is_half_carry = is_half_carry(&(*sp as u8), &(s8 as u8));
        let is_full_carry = is_full_carry(&(*sp as u8), &(s8 as u8));

        *sp = (*sp as i32).wrapping_add(s8 as i32) as u16;

        set_flag2(flags, FLAG_Z, false);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, is_half_carry);
        set_flag2(flags, FLAG_C, is_full_carry);

        4
    }

    fn op_sub_r(&self, accum: &mut u8, v: u8, flags: &mut u8) -> u8 {
        let half_borrow = is_half_borrow(accum, &v);
        let full_borrow = is_full_borrow(accum, &v);

        *accum = (*accum).wrapping_sub(v);

        set_flag2(flags, FLAG_Z, *accum == 0);
        set_flag2(flags, FLAG_N, true);
        set_flag2(flags, FLAG_H, half_borrow);
        set_flag2(flags, FLAG_C, full_borrow);

        1
    }

    fn op_sub_d8(&self, bus: &MemoryBus, accum: &mut u8, pc: &mut u16, flags: &mut u8) -> u8 {
        let d8 = self.read_byte_from_pc(bus, pc);
        self.op_sub_r(accum, d8, flags) + 1
    }

    fn op_sub_addr(&self, bus: &MemoryBus, accum: &mut u8, addr: u16, flags: &mut u8) -> u8 {
        let v = bus.read_byte(addr);
        self.op_sub_r(accum, v, flags) + 1
    }

    fn op_adc_r(&self, accum: &mut u8, v: u8, flags: &mut u8) -> u8 {
        let cy = if get_flag2(*flags, FLAG_C) { 1 } else { 0 };

        let mut r = (*accum).wrapping_add(v);
    
        let is_full_carry = is_full_carry(accum, &v) || is_full_carry(&r, &cy);
        let is_half_carry = is_half_carry(accum, &v) || is_half_carry(&r, &cy);
    
        r = r.wrapping_add(cy);
        *accum = r;
    
        set_flag2(flags, FLAG_Z, *accum == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, is_half_carry);
        set_flag2(flags, FLAG_C, is_full_carry);
    
        1
    }

    fn op_adc_d8(&self, bus: &MemoryBus, accum: &mut u8, pc: &mut u16, flags: &mut u8) -> u8 {
        let d8 = self.read_byte_from_pc(bus, pc);
        self.op_adc_r(accum, d8, flags) + 1
    }

    fn op_adc_addr(&self, bus: &MemoryBus, accum: &mut u8, addr: u16, flags: &mut u8) -> u8 {
        let v = bus.read_byte(addr);
        self.op_adc_r(accum, v, flags) + 1
    }

    fn op_sbc_r(&self, accum: &mut u8, v: u8, flags: &mut u8) -> u8 {
        let cy = if get_flag2(*flags, FLAG_C) { 1 } else { 0 };

        let mut r = (*accum).wrapping_sub(v);
    
        let is_full_borrow = is_full_borrow(accum, &v) || is_full_borrow(&r, &cy);
        let is_half_borrow = is_half_borrow(accum, &v) || is_half_borrow(&r, &cy);
    
        r = r.wrapping_sub(cy);
        *accum = r;
    
        set_flag2(flags, FLAG_Z, *accum == 0);
        set_flag2(flags, FLAG_N, true);
        set_flag2(flags, FLAG_H, is_half_borrow);
        set_flag2(flags, FLAG_C, is_full_borrow);
    
        1
    }

    fn op_sbc_d8(&self, bus: &MemoryBus, accum: &mut u8, pc: &mut u16, flags: &mut u8) -> u8 {
        let d8 = self.read_byte_from_pc(bus, pc);
        self.op_sbc_r(accum, d8, flags) + 1
    }

    fn op_sbc_addr(&self, bus: &MemoryBus, accum: &mut u8, addr: u16, flags: &mut u8) -> u8 {
        let v = bus.read_byte(addr);
        self.op_sbc_r(accum, v, flags) + 1
    }

    fn op_daa(&self, accum: &mut u8, flags: &mut u8) -> u8 {
        // https://forums.nesdev.com/viewtopic.php?t=15944
        // note: assumes a is a uint8_t and wraps from 0xff to 0
        let c = get_flag2(*flags, FLAG_C);
        let h = get_flag2(*flags, FLAG_H);

        if !get_flag2(*flags, FLAG_N) {  // after an addition, adjust if (half-)carry occurred or if result is out of bounds
            if c || *accum > 0x99 { 
                *accum = accum.wrapping_add(0x60);
                set_flag2(flags, FLAG_C, true);
            }
            if h || (*accum & 0x0f) > 0x09 {
                *accum = accum.wrapping_add(0x6);
            }
        } else {  // after a subtraction, only adjust if (half-)carry occurred
            if c { 
                *accum = accum.wrapping_sub(0x60);
            }
            if h { 
                *accum = accum.wrapping_sub(0x6);
            }
        }
        
        set_flag2(flags, FLAG_Z, *accum == 0);
        set_flag2(flags, FLAG_H, false);

        1
    }

    fn op_scf(&self, flags: &mut u8) -> u8 {
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, true);

        1
    }

    fn op_ccf(&self, flags: &mut u8) -> u8 {
        let cy = get_flag2(*flags, FLAG_C);

        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, !cy);

        1
    }

    fn op_cp_r(&self, a: u8, r: u8, flags: &mut u8) -> u8 {
        let half_borrow = is_half_borrow(&a, &r);
        let full_borrow = is_full_borrow(&a, &r);

        let r = a.wrapping_sub(r);
        
        set_flag2(flags, FLAG_Z, r == 0);
        set_flag2(flags, FLAG_N, true);
        set_flag2(flags, FLAG_H, half_borrow);
        set_flag2(flags, FLAG_C, full_borrow);

        1
    }

    fn op_cp_d8(&self, bus: &MemoryBus, a: u8, pc: &mut u16, flags: &mut u8) -> u8 {
        let d8 = self.read_byte_from_pc(bus, pc);
        self.op_cp_r(a, d8, flags) + 1
    }
    
    fn op_cp_addr(&self, bus: &MemoryBus, a: u8, addr: u16, flags: &mut u8) -> u8 {
        let v = bus.read_byte(addr);

        let r = a.wrapping_sub(v);

        set_flag2(flags, FLAG_Z, r == 0);
        set_flag2(flags, FLAG_N, true);

        let hc = (a as i8 & 0xF) - (v as i8 & 0xF);
        set_flag2(flags, FLAG_H, hc < 0);
        set_flag2(flags, FLAG_C, a < v);

        2
    }

    fn op_ld_r_r(&self, a: &mut u8, b: u8) -> u8 {
        *a = b;

        1
    }

    fn op_ld_r_addr(&self, bus: &MemoryBus, r: &mut u8, addr: u16) -> u8 {
        *r = bus.read_byte(addr);

        2
    }

    fn op_ld_r_d8(&self, bus: &MemoryBus, r: &mut u8, pc: &mut u16) -> u8 {
        *r = self.read_byte_from_pc(bus, pc);

        2
    }
    
    fn op_ld_r_d16(&self, bus: &MemoryBus, hi: &mut u8, lo: &mut u8, pc: &mut u16) -> u8 {
        *lo = self.read_byte_from_pc(bus, pc);
        *hi = self.read_byte_from_pc(bus, pc);

        3
    }

    fn op_ld_sp_d16(&self, bus: &MemoryBus, sp: &mut u16, pc: &mut u16) -> u8 {
        *sp = self.read_word_from_pc(bus, pc);

        3
    }

    fn op_ld_sp_r16(&self, sp: &mut u16, r: u16) -> u8 {
        *sp = r;

        2
    }

    fn op_ld_hl_sp_add_s8(&self, bus: &MemoryBus, h: &mut u8, l: &mut u8, sp: u16, pc: &mut u16, flags: &mut u8) -> u8 {
        let d8 = self.read_byte_from_pc(bus, pc) as i8;
        
        let v = d8 as u8;
        let lb = sp as u8;

        let is_half_carry = is_half_carry(&lb, &v);
        let is_full_carry = is_full_carry(&lb, &v);

        let hl = (sp as i32).wrapping_add(d8 as i32) as u16;
        *h = (hl >> 8) as u8;
        *l = hl as u8;

        set_flag2(flags, FLAG_Z, false);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, is_half_carry);
        set_flag2(flags, FLAG_C, is_full_carry);

        3
    }
    
    fn op_ld_r_a8(&self, bus: &MemoryBus, r: &mut u8, pc: &mut u16) -> u8 {
        let a8 = self.read_byte_from_pc(bus, pc);
        *r = bus.read_byte(0xFF00 | (a8 as u16));

        3
    }

    fn op_ld_r_a16(&self, bus: &MemoryBus, r: &mut u8, pc: &mut u16) -> u8 {
        let a16 = self.read_word_from_pc(bus, pc);
        *r = bus.read_byte(a16);

        4
    }

    fn op_ld_a_mem_hl_inc(&self, bus: &MemoryBus, r: &mut u8, h: &mut u8, l: &mut u8) -> u8 {
        let hl = to_u16(*h, *l);
        *r = bus.read_byte(hl);

        let d_hl = hl.wrapping_add(1);
        *h = (d_hl >> 8) as u8;
        *l = d_hl as u8;

        2
    }

    fn op_ld_a_mem_hl_dec(&self, bus: &MemoryBus, r: &mut u8, h: &mut u8, l: &mut u8) -> u8 {
        let hl = to_u16(*h, *l);
        *r = bus.read_byte(hl);

        let d_hl = hl.wrapping_sub(1);
        *h = (d_hl >> 8) as u8;
        *l = d_hl as u8;

        2
    }

    fn op_ld_addr_r(&self, bus: &MemoryBus, addr: u16, r: u8) -> u8 {
        bus.write_byte(addr, r);

        2
    }

    fn op_ld_addr_r_dec_hl(&self, bus: &MemoryBus, h: &mut u8, l: &mut u8, r: u8) -> u8 {
        let hl = to_u16(*h, *l);
        bus.write_byte(hl, r);

        let d_hl = hl.wrapping_sub(1);
        *h = (d_hl >> 8) as u8;
        *l = d_hl as u8;

        2
    }

    fn op_ld_addr_r_inc_hl(&self, bus: &MemoryBus, h: &mut u8, l: &mut u8, r: u8) -> u8 {
        let hl = to_u16(*h, *l);
        bus.write_byte(hl, r);

        let d_hl = hl.wrapping_add(1);
        *h = (d_hl >> 8) as u8;
        *l = d_hl as u8;

        2
    }

    fn op_ld_addr_d8(&self, bus: &MemoryBus, addr: u16, pc: &mut u16) -> u8 {
        let d8 = self.read_byte_from_pc(bus, pc);
        bus.write_byte(addr, d8);

        3
    }

    fn op_ld_a8_r(&self, bus: &MemoryBus, pc: &mut u16, reg: u8) -> u8 {
        let address: u16 = 0xFF00 | (self.read_byte_from_pc(bus, pc) as u16);
        bus.write_byte(address, reg);

        3
    }

    fn op_ld_a16_r(&self, bus: &MemoryBus, pc: &mut u16, reg: u8) -> u8 {
        let a16 = self.read_word_from_pc(bus, pc);
        bus.write_byte(a16, reg);

        4
    }

    fn op_ld_a16_r16(&self, bus: &MemoryBus, pc: &mut u16, reg: u16) -> u8 {
        let a16 = self.read_word_from_pc(bus, pc);
        self.write_word(bus, a16, reg);

        5
    }

    fn op_and_r(&self, accum: &mut u8, v: u8, flags: &mut u8) -> u8 {
        *accum &= v;

        set_flag2(flags, FLAG_Z, *accum == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, true);
        set_flag2(flags, FLAG_C, false);

        1
    }

    fn op_and_d8(&self, bus: &MemoryBus, accum: &mut u8, pc: &mut u16, flags: &mut u8) -> u8 {
        let d8 = self.read_byte_from_pc(bus, pc);
        self.op_and_r(accum, d8, flags) + 1
    }

    fn op_and_addr(&self, bus: &MemoryBus, accum: &mut u8, addr: u16, flags: &mut u8) -> u8 {
        let v = bus.read_byte(addr);
        self.op_and_r(accum, v, flags) + 1
    }

    fn op_or_r(&self, accum: &mut u8, v: u8, flags: &mut u8) -> u8 {
        *accum |= v;

        set_flag2(flags, FLAG_Z, *accum == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, false);

        1
    }
    
    fn op_or_d8(&self, bus: &MemoryBus, accum: &mut u8, pc: &mut u16, flags: &mut u8) -> u8 {
        let d8 = self.read_byte_from_pc(bus, pc);
        self.op_or_r(accum, d8, flags) + 1
    }

    fn op_or_addr(&self, bus: &MemoryBus, accum: &mut u8, addr: u16, flags: &mut u8) -> u8 {
        let v = bus.read_byte(addr);
        self.op_or_r(accum, v, flags) + 1
    }

    fn op_xor_r(&self, accum: &mut u8, v: u8, flags: &mut u8) -> u8 {
        *accum ^= v;

        set_flag2(flags, FLAG_Z, *accum == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, false);

        1
    }

    fn op_xor_d8(&self, bus: &MemoryBus, accum: &mut u8, pc: &mut u16, flags: &mut u8) -> u8 {
        let d8 = self.read_byte_from_pc(bus, pc);
        self.op_xor_r(accum, d8, flags) + 1
    }

    fn op_xor_addr(&self, bus: &MemoryBus, accum: &mut u8, addr: u16, flags: &mut u8) -> u8 {
        let v = bus.read_byte(addr);
        self.op_xor_r(accum, v, flags) + 1
    }

    fn op_cpl(&self, accum: &mut u8, flags: &mut u8) -> u8 {
        *accum = !(*accum);

        set_flag2(flags, FLAG_N, true);
        set_flag2(flags, FLAG_H, true);

        1
    }

    fn op_jp_v16(&self, pc: &mut u16, v: u16) -> u8 {
        *pc = v;

        1
    }

    fn op_jp_a16(&self, bus: &MemoryBus, pc: &mut u16, condition: bool) -> u8 {
        let a16 = self.read_word_from_pc(bus, pc);
        
        if condition {
            *pc = a16;

            4
        }
        else {
            3
        }
    }

    fn op_jr_s8(&self, bus: &MemoryBus, pc: &mut u16, condition: bool) -> u8 {
        let offset = self.read_byte_from_pc(bus, pc) as i8;

        if condition {
            *pc = (*pc as i32 + offset as i32) as u16;

            3
        }
        else {
            2
        }
    }

    fn op_call_a16(&self, bus: &MemoryBus, pc: &mut u16, sp: &mut u16, condition: bool) -> u8 {
        let a16 = self.read_word_from_pc(bus, pc);

        if condition {
            *sp -= 1;
            bus.write_byte(*sp, ((*pc & 0xFF00) >> 8) as u8);
            *sp -= 1;
            bus.write_byte(*sp, (*pc & 0x00FF) as u8);

            *pc = a16;

            6
        }
        else {
            3
        }
    }

    fn op_ret(&self, bus: &MemoryBus, pc: &mut u16, sp: &mut u16, condition: bool) -> u8 {
        if condition {
            let l = bus.read_byte(*sp) as u16;
            *sp += 1;
            let h = bus.read_byte(*sp) as u16;
            *sp += 1;

            *pc = h << 8 | l;

            5
        }
        else {
            2
        }
    }

    fn op_reti(&self, bus: &MemoryBus, pc: &mut u16, sp: &mut u16) -> u8 {
        *pc = bus.read_byte(*sp) as u16;
        *sp += 1;
        *pc |= (bus.read_byte(*sp) as u16) << 8;
        *sp += 1;

        self.interrupts.borrow_mut().interrupts_enabled = true;

        4
    }

    fn op_push_r16(&self, bus: &MemoryBus, sp: &mut u16, hi: u8, lo: u8) -> u8 {
        *sp -= 1;
        bus.write_byte(*sp, hi);
        *sp -= 1;
        bus.write_byte(*sp, lo);

        4
    }

    fn op_pop_af(&self, bus: &MemoryBus, sp: &mut u16, a: &mut u8, f: &mut u8) -> u8 {
        *f = bus.read_byte(*sp);
        *sp += 1;
        *a = bus.read_byte(*sp);
        *sp += 1;

        // only the higher 4 bits are used for flags
        *f &= 0xF0; 

        3
    }

    fn op_pop_r16(&self, bus: &MemoryBus, sp: &mut u16, hi: &mut u8, lo: &mut u8) -> u8 {
        *lo = bus.read_byte(*sp);
        *sp += 1;
        *hi = bus.read_byte(*sp);
        *sp += 1;

        3
    }

    fn op_rla(&self, a: &mut u8, flags: &mut u8) -> u8 {
        let prev_carry: u8 = get_flag2(*flags, FLAG_C) as u8;
        
        let carry = *a & (1 << 7);
        *a = (*a << 1) | prev_carry;

        set_flag2(flags, FLAG_Z, false);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, carry != 0);

        1
    }

    fn op_rra(&self, a: &mut u8, flags: &mut u8) -> u8 {
        let prev_carry: u8 = get_flag2(*flags, FLAG_C) as u8;
        
        let carry = *a & 0x1;
        *a = (*a >> 1) | (prev_carry << 7);

        set_flag2(flags, FLAG_Z, false);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, carry != 0);

        1
    }

    fn op_rlca(&self, a: &mut u8, flags: &mut u8) -> u8 {
        let carry = *a & (1 << 7);
        *a = (*a << 1) | (carry >> 7);
    
        set_flag2(flags, FLAG_Z, false);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, carry != 0);
    
        1
    }

    fn op_rrca(&self, a: &mut u8, flags: &mut u8) -> u8 {
        let carry = *a & 0x1;
        *a = (*a >> 1) | (carry << 7);
    
        set_flag2(flags, FLAG_Z, false);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, carry != 0);
    
        1
    }

    fn op_rst_n(&self, bus: &MemoryBus, n: u8, pc: &mut u16, sp: &mut u16) -> u8 {
        *sp -= 1;
        bus.write_byte(*sp, (*pc >> 8) as u8);
        *sp -= 1;
        bus.write_byte(*sp, *pc as u8);

        *pc = match n {
            0 => 0x0000,
            1 => 0x0008,
            2 => 0x0010,
            3 => 0x0018,
            4 => 0x0020,
            5 => 0x0028,
            6 => 0x0030,
            7 => 0x0038,
            _ => panic!("invalid rst"),
        };

        4
    }

    fn op_rlc_r(&self, r: &mut u8, flags: &mut u8) -> u8 {
        let carry = (*r & 0x80) >> 7;
        *r = (*r << 1) | carry;

        set_flag2(flags, FLAG_Z, *r == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, carry != 0);

        2
    }

    fn op_rlc_addr(&self, bus: &MemoryBus, addr: u16, flags: &mut u8) -> u8 {
        let mut v = bus.read_byte(addr);
        self.op_rlc_r(&mut v, flags);
        bus.write_byte(addr, v);

        4
    }

    fn op_rrc_r(&self, r: &mut u8, flags: &mut u8) -> u8 {
        let carry = *r & 0x1;
        *r = (*r >> 1) | (carry << 7);

        set_flag2(flags, FLAG_Z, *r == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, carry != 0);

        2
    }

    fn op_rrc_addr(&self, bus: &MemoryBus, addr: u16, flags: &mut u8) -> u8 {
        let mut v = bus.read_byte(addr);
        self.op_rrc_r(&mut v, flags);
        bus.write_byte(addr, v);

        4
    }

    fn op_rl_r(&self, r: &mut u8, flags: &mut u8) -> u8 {
        let prev_carry: u8 = get_flag2(*flags, FLAG_C) as u8;
    
        let carry = ((*r) & (1 << 7)) != 0;
        *r = (*r << 1) | prev_carry;

        set_flag2(flags, FLAG_Z, *r == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, carry);

        2
    }

    fn op_rl_addr(&self, bus: &MemoryBus, addr: u16, flags: &mut u8) -> u8 {
        let mut v = bus.read_byte(addr);
        self.op_rl_r(&mut v, flags);
        bus.write_byte(addr, v);

        4
    }

    fn op_rr_r(&self, r: &mut u8, flags: &mut u8) -> u8 {
        let prev_carry: u8 = if get_flag2(*flags, FLAG_C) { 1 } else { 0 };
    
        let carry = (*r) & 1 != 0;
        *r = (*r >> 1) | (prev_carry << 7);
    
        set_flag2(flags, FLAG_Z, *r == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, carry);
    
        2
    }

    fn op_rr_addr(&self, bus: &MemoryBus, addr: u16, flags: &mut u8) -> u8 {
        let mut v = bus.read_byte(addr);
        self.op_rr_r(&mut v, flags);
        bus.write_byte(addr, v);

        4
    }

    fn op_sla_r(&self, r: &mut u8, flags: &mut u8) -> u8 {
        let carry = *r & (1 << 7) != 0;
        *r <<= 1;

        set_flag2(flags, FLAG_Z, *r == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, carry);

        2
    }

    fn op_sla_addr(&self, bus: &MemoryBus, addr: u16, flags: &mut u8) -> u8 {
        let mut v = bus.read_byte(addr);
        self.op_sla_r(&mut v, flags);
        bus.write_byte(addr, v);

        4
    }

    fn op_srl_r(&self, r: &mut u8, flags: &mut u8) -> u8 {
        let carry = *r & 1;
        *r >>= 1;

        set_flag2(flags, FLAG_Z, *r == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, carry != 0);

        2
    }

    fn op_srl_addr(&self, bus: &MemoryBus, addr: u16, flags: &mut u8) -> u8 {
        let mut v = bus.read_byte(addr);
        self.op_srl_r(&mut v, flags);
        bus.write_byte(addr, v);

        4
    }

    fn op_sra_r(&self, r: &mut u8, flags: &mut u8) -> u8 {
        let carry = *r & 0x1;
        *r = (*r >> 1) | (*r & 0x80); 

        set_flag2(flags, FLAG_Z, *r == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, carry != 0);

        2
    }

    fn op_sra_addr(&self, bus: &MemoryBus, addr: u16, flags: &mut u8) -> u8 {
        let mut v = bus.read_byte(addr);
        self.op_sra_r(&mut v, flags);
        bus.write_byte(addr, v);

        4
    }

    fn op_di(&self) -> u8 {
        let mut interrupts = self.interrupts.borrow_mut();

        interrupts.interrupts_enabled = false;
        interrupts.interrupts_enable_request = false;

        1
    }

    fn op_ei(&self) -> u8 {
        let mut interrupts = self.interrupts.borrow_mut();

        interrupts.interrupts_enable_request = true;

        1
    }

    fn op_swap_r(&self, r: &mut u8, flags: &mut u8) -> u8 {
        let l = *r & 0x0F;
        let h = *r & 0xF0;

        *r = l << 4 | h >> 4;

        set_flag2(flags, FLAG_Z, *r == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, false);
        set_flag2(flags, FLAG_C, false);

        2
    }

    fn op_swap_addr(&self, bus: &MemoryBus, addr: u16, flags: &mut u8) -> u8 {
        let mut v = bus.read_byte(addr);
        self.op_swap_r(&mut v, flags);
        bus.write_byte(addr, v);

        4
    }

    fn op_bitn_r(&self, bit: u8, reg: u8, flags: &mut u8) -> u8 {
        let b = (reg >> bit) & 1;

        set_flag2(flags, FLAG_Z, b == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, true);

        2
    }

    fn op_bitn_addr(&self, bus: &MemoryBus, bit: u8, addr: u16, flags: &mut u8) -> u8 {
        let b = bus.read_byte(addr) & (1 << bit);

        set_flag2(flags, FLAG_Z, b == 0);
        set_flag2(flags, FLAG_N, false);
        set_flag2(flags, FLAG_H, true);

        3
    }

    fn op_setn_r(&self, bit: u8, r: &mut u8) -> u8 {
        *r |= 1 << bit;

        2
    }
    
    fn op_resn_r(&self, bit: u8, r: &mut u8) -> u8 {
        *r &= !(1 << bit);

        2
    }

    fn op_resn_addr(&self, bus: &MemoryBus, bit: u8, addr: u16) -> u8 {
        let v = bus.read_byte(addr) & !(1 << bit);
        bus.write_byte(addr, v);

        4
    }

    fn op_setn_addr(&self, bus: &MemoryBus, bit: u8, addr: u16) -> u8 {
        let v = bus.read_byte(addr) | (1 << bit);
        bus.write_byte(addr, v);

        4
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        match address {
            0xFFFF => self.interrupts.borrow().enabled,
            0xFF0F => self.interrupts.borrow().flags,
            _ => panic!("Invalid address")
        }
    }

    pub fn write_byte(&self, address: u16, data: u8) {
        match address {
            0xFFFF => self.interrupts.borrow_mut().enabled = data,
            0xFF0F => self.interrupts.borrow_mut().flags = data,
            _ => panic!("Invalid address")
        }
    }
}