
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

pub fn is_half_borrow(x: &u8, y: &u8) -> bool
{
    (*x & 0x0F) < (*y & 0x0F)
}

pub fn is_full_borrow(x: &u8, y: &u8) -> bool
{
    ((*x as u16) & 0xFF) < ((*y as u16) & 0xFF)
}