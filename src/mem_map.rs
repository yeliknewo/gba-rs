use num::{FromPrimitive, Unsigned, PrimInt};
use std::mem::size_of;

use super::read_bytes::read_le;
use super::write_bytes::write_le;

fn read_bios_builder<T: Unsigned + FromPrimitive>(mask: u32) -> Box<Fn(u32, [u8; 4], u32, &MemMap) -> T> {
    Box::new(
        |address: u32, cpu_protected: [u8; 4], reg_15_i: u32, mem_map: &MemMap| {
            mem_map.read_bios(address, mask, cpu_protected, reg_15_i)
        }
    )
}

fn unreadable_builder<T: Unsigned + FromPrimitive>() -> Box<Fn(u32, [u8; 4], u32, &MemMap) -> T> {
    Box::new(
        |_, _, _, mem_map: &MemMap| {
            mem_map.read_unreadable()
        }
    )
}

fn unwritable_builder<T: Unsigned + PrimInt>() -> Box<Fn(u32, T)> {
    Box::new(
        |_, _, mem_map: &mut MemMap| {
            mem_map.write_unwritable()
        }
    )
}

pub struct MemMap {
    mem_access: [MemAccess; 15],
    memory: Vec<u8>,
}

impl MemMap {
    pub fn new() -> MemMap {
        MemMap {
            mem_access: [
                MemAccess::new(0x00003FFF, read_bios_builder::<u8>(0x03), read_bios_builder::<u16>(0x02), read_bios_builder::<u32>(0x0F), unwritable,         unwritable,             unwritable),
                MemAccess::new(0x00000000, unreadable_builder(),          unreadable_builder(),         unreadable_builder(),         unwritable,         unwritable,             unwritable),
                MemAccess::new(0x0003FFFF, read_generic_8(2),   read_generic_16(2), read_generic_32(2), write_generic_8(2), write_generic_16(3),    write_generic_32(3)),
                MemAccess::new(),
                MemAccess::new(),

                MemAccess::new(),
                MemAccess::new(),
                MemAccess::new(),
                MemAccess::new(),
                MemAccess::new(),

                MemAccess::new(),
                MemAccess::new(),
                MemAccess::new(),
                MemAccess::new(),
                MemAccess::new(),
            ],
            memory: vec!(),
        }
    }

    fn read_bios<T: Unsigned + FromPrimitive>(&self, address: u32, mask: u32, cpu_protected: [u8; 4], reg_15_i: u32) -> T {
        if reg_15_i >> 24 != 0 {
            if address > 0x4000 {
                read_le(address & mask, &cpu_protected)
            } else {
                self.read_unreadable()
            }
        } else {
            self.read_generic(address, 0)
        }
    }

    fn write_generic<T: Unsigned + PrimInt>(&mut self, address: u32, mask: u32, value: T) {
        let mask = self.mem_access[mask as usize].mask;

        write_le(&self.memory, address + mask, value)
    }

    fn write_unwritable(&mut self) {

    }
}

struct MemAccess {
    mask: u32,
    read_8: Box<Fn(u32, [u8; 4], u32, &MemMap) -> u8>,
    read_16: Box<Fn(u32, [u8; 4], u32, &MemMap) -> u16>,
    read_32: Box<Fn(u32, [u8; 4], u32, &MemMap) -> u32>,
    write_8: u8,
    write_16: u16,
    write_32: u32,
}

impl MemAccess {
    pub fn new(
        mask: u32,
        read_8: Box<Fn(u32, [u8; 4], u32, &MemMap) -> u8>,
        read_16: Box<Fn(u32, [u8; 4], u32, &MemMap) -> u16>,
        read_32: Box<Fn(u32, [u8; 4], u32, &MemMap) -> u32>,
        write_8: u8,
        write_16: u16,
        write_32: u32
    ) -> MemAccess {
        MemAccess {
            mask: mask,
            read_8: read_8,
            read_16: read_16,
            read_32: read_32,
            write_8: write_8,
            write_16: write_16,
            write_32: write_32,
        }
    }
}
