
pub fn to_u16(hi: u8, lo: u8) -> u16 {
    ((hi as u16) << 8) | lo as u16
}

pub fn get_bit(mask: u8, bit: u8) -> u8 {
    (mask & (1 << bit)) >> bit
}

pub fn get_flag2(mask: &u8, flag: u8) -> bool {
    (*mask) & flag != 0
}

pub fn set_flag2(mask: &mut u8, flag: u8, val: bool) {
    if val {
        *mask |= flag;
    }
    else {
        *mask &= !(flag);
    }
}

pub fn is_half_borrow(x: &u8, y: &u8) -> bool {
    (*x & 0x0F) < (*y & 0x0F)
}

pub fn is_full_borrow(x: &u8, y: &u8) -> bool {
    ((*x as u16) & 0xFF) < ((*y as u16) & 0xFF)
}

pub fn is_full_carry(x: &u8, y: &u8) -> bool {
    ((((*x as u16) & 0xFF) + ((*y as u16) & 0xFF)) & 0x100) != 0
}

pub fn is_half_carry(x: &u8, y: &u8) -> bool {
    (((*x & 0x0F) + (*y & 0x0F)) & 0x10) != 0
}

pub fn is_half_carry16(x: &u16, y: &u16) -> bool {
    (((*x & 0x0FFF) + (*y & 0x0FFF)) & 0x1000) != 0
} 

pub fn is_full_carry16(x: &u16, y: &u16) -> bool {
    ((( (*x as u32) & 0xFFFF) + ((*y as u32) & 0xFFFF)) & 0x10000) != 0
}