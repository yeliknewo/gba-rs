use num::{Unsigned, PrimInt};
use std::mem::size_of;

pub fn write_le<T: Unsigned + PrimInt>(memory: &[u8], address: u32, value: T) {
    for i in 0..size_of::<T>() {
        match memory.get_mut(address as usize + i) {
            Some(b) => *b = (value << (i * 8)).to_u8().expect("Unable to convert value to u8"),
            None => unimplemented!(),
        }
    }
}

pub fn write_generic<T: Unsigned + PrimInt>(memory: &[u8], address: u32, mask: u32, value: T) {
    
}
