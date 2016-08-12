use num::{Unsigned, FromPrimitive};
use std::mem::size_of;

use super::mem_map::MemMap;

pub fn read_le<T: Unsigned + FromPrimitive>(memory: &[u8], address: u32) -> T {
    let mut value = 0;

    for i in 0..size_of::<T>() {
        match memory.get(address as usize + i) {
            Some(b) => value += (*b as u32) << (i * 8),
            None => unimplemented!(),
        }
    }

    T::from_u32(value).expect("Value can not be converted to type T")
}

pub fn read_generic<T: Unsigned + FromPrimitive>(memory: &[u8], address: u32, mask: u32, mem_map: &MemMap) -> T {
    read_le(memory, address & mem_map[mask].mask)
}

pub fn read_unreadable<T: Unsigned + FromPrimitive>() -> T {
    T::zero()
}
