const R13_IRQ: usize = 18;
const R14_IRQ: usize = 19;
const SPSR_IRQ: usize = 20;
const R13_USR: usize = 26;
const R14_USR: usize = 27;
const R13_SVC: usize = 28;
const R14_SVC: usize = 29;
const SPSR_SVC: usize = 30;
const R13_ABT: usize = 31;
const R14_ABT: usize = 32;
const SPSR_ABT: usize = 33;
const R13_UND: usize = 34;
const R14_UND: usize = 35;
const SPSR_UND: usize = 36;
const R8_FIQ: usize = 37;
const R9_FIQ: usize = 38;
const R10_FIQ: usize = 39;
const R11_FIQ: usize = 40;
const R12_FIQ: usize = 41;
const R13_FIQ: usize = 42;
const R14_FIQ: usize = 43;
const SPSR_FIQ: usize = 44;

use super::mem_map;

struct Cpu {
    regs: [Reg; 45],

    cpu_bits_set: [u8; 256],
    cpu_prefetch: [u32; 2],
    n_flag: bool,
    c_flag: bool,
    z_flag: bool,
    v_flag: bool,

    arm_state: bool,
    arm_irq_enable: bool,
    arm_next_pc: u32,
    arm_mode: i32,

    bus_prefetch: bool,
    bus_prefetch_enable: bool,
    bus_prefetch_count: u32,

    memory_wait: [u8; 16],
    memory_wait_32: [u8; 16],
    memory_wait_seq: [u8; 16],
    memory_wait_seq_32: [u8; 16],

    g_ie: u16,
    g_if: u16,
    g_ime: u16,

    cpu_next_event: i32,
    cpu_total_ticks: i32,

    mem_map: mem_map::MemMap,

    bios_protected: [u8; 4],
}

impl Cpu {
    pub fn new() -> Cpu {
        let mut bits = [0; 256];

        for i in 0..256 {
            let mut count = 0;
            for j in 0..8 {
                if i & (1 << j) != 0 {
                    count += 1;
                }
            }

            bits[i] = count;
        }

        Cpu {
            regs: [Reg::I(0); 45],

            cpu_bits_set: bits,
            cpu_prefetch: [0; 2],

            n_flag: false,
            c_flag: false,
            z_flag: false,
            v_flag: false,

            arm_state: true,
            arm_irq_enable: true,
            arm_next_pc: 0,
            arm_mode: 0x1f,

            bus_prefetch: false,
            bus_prefetch_enable: false,
            bus_prefetch_count: 0,

            memory_wait: [0, 0, 2, 0, 0, 0, 0, 0, 4, 4, 4, 4, 4, 4, 4, 0],
            memory_wait_32: [0, 0, 5, 0, 0, 1, 1, 0, 7, 7, 9, 9, 13, 13, 4, 0],
            memory_wait_seq: [0, 0, 2, 0, 0, 0, 0, 0, 2, 2, 4, 4, 8, 8, 4, 0],
            memory_wait_seq_32: [0, 0, 5, 0, 0, 1, 1, 0, 5, 5, 9, 9, 17, 17, 4, 0],

            g_ie: 0x0000,
            g_if: 0x0000,
            g_ime: 0x0000,

            cpu_next_event: 0,
            cpu_total_ticks: 0,

            mem_map: mem_map::MemMap::new(),

            bios_protected: [0x00, 0xF0, 0x29, 0xE1],
        }
    }

    pub fn reset(&mut self) {
        //reset registers
        for i in 0..45 {
            self.regs[i] = Reg::I(0);
        }

        self.arm_mode = 0x1f;

        {
            self.regs[15] = Reg::I(0);
            self.arm_mode = 0x13;
            self.arm_irq_enable = false;
        }

        self.arm_state = true;
        self.c_flag = false;
        self.v_flag = false;
        self.n_flag = false;
        self.z_flag = false;

        self.regs[16] = Reg::I(self.get_reg_i(16) | 0x40);

        self.cpu_update_cpsr();

        self.arm_next_pc = self.get_reg_i(15);
        self.regs[15] = Reg::I(self.get_reg_i(15) + 4);

        //arm_prefetch
    }

    fn cpu_update_cpsr(&mut self) {
        let mut cpsr = self.get_reg_i(16) & 0x40;

        if self.n_flag {
            cpsr |= 0x80000000;
        }

        if self.z_flag {
            cpsr |= 0x40000000;
        }

        if self.c_flag {
            cpsr |= 0x20000000;
        }

        if self.v_flag {
            cpsr |= 0x10000000;
        }

        if !self.arm_state {
            cpsr |= 0x00000020;
        }

        if !self.arm_irq_enable {
            cpsr |= 0x80;
        }

        //cpsr |= (self.arm_mode & 0x1f);

        self.regs[16] = Reg::I(cpsr);
    }

    fn data_ticks_access_16(&mut self, address: usize) -> u8 {
        let addr = (address >> 24) & 15;
        let value = self.memory_wait[addr];

        if (addr >= 0x08) || (addr < 0x02) {
            self.bus_prefetch_count = 0;
            self.bus_prefetch_enable = false;
        } else if self.bus_prefetch {
            let mut wait_state = value;
            if wait_state == 0 {
                wait_state = 1;
            }
            self.bus_prefetch_count = ((self.bus_prefetch_count + 1) << wait_state) - 1;
        }

        value
    }

    fn data_ticks_access_32(&mut self, address: usize) -> u8 {
        let addr = (address >> 24) & 15;
        let value = self.memory_wait_32[addr];

        if (addr > 0x08) || (addr < 0x02) {
            self.bus_prefetch_count = 0;
            self.bus_prefetch = false;
        } else if self.bus_prefetch {
            let mut wait_state = value;
            if wait_state == 0 {
                wait_state = 1;
            }
            self.bus_prefetch_count = ((self.bus_prefetch_count + 1) << wait_state) - 1;
        }

        value
    }

    fn data_ticks_access_seq_16(&mut self, address: usize) -> u8 {
        let addr = (address >> 24) & 15;
        let value = self.memory_wait_seq[addr];

        if addr >= 0x08 || addr < 0x02 {
            self.bus_prefetch_count = 0;
            self.bus_prefetch_enable = false;
        } else if self.bus_prefetch {
            let mut wait_state = value;
            if wait_state == 0 {
                wait_state = 1;
            }
            self.bus_prefetch_count = ((self.bus_prefetch_count + 1) << wait_state) - 1;
        }

        value
    }

    fn data_ticks_access_seq_32(&mut self, address: usize) -> u8 {
        let addr = (address >> 24) & 15;
        let value = self.memory_wait_seq_32[addr];

        if addr >= 0x08 || addr < 0x02 {
            self.bus_prefetch_count = 0;
            self.bus_prefetch_enable = false;
        } else if self.bus_prefetch {
            let mut wait_state = value;
            if wait_state == 0 {
                wait_state = 1;
            }
            self.bus_prefetch_count = ((self.bus_prefetch_count + 1) << wait_state) - 1;
        }

        value
    }

    fn code_ticks_access_16(&mut self, address: usize) -> u8 {
        let addr = (address >> 24) & 15;

        if addr >= 0x08 && addr <= 0x0D {
            if self.bus_prefetch_count & 0x1 != 0 {
                if self.bus_prefetch_count & 0x2 != 0 {
                    self.bus_prefetch_count = ((self.bus_prefetch_count & 0xFF) >> 2) | (self.bus_prefetch_count & 0xFFFFFF00);
                    return 0;
                }
                self.bus_prefetch_count = ((self.bus_prefetch_count & 0xFF) >> 1) | (self.bus_prefetch_count & 0xFFFFFF00);
                return self.memory_wait_seq[addr] - 1;
            } else {
                self.bus_prefetch_count = 0;
                return self.memory_wait[addr];
            }
        } else {
            self.bus_prefetch_count = 0;
            return self.memory_wait[addr];
        }
    }

    fn code_ticks_access_32(&mut self, address: usize) -> u8 {
        let addr = (address >> 24) & 15;

        if addr >= 0x08 && addr <= 0x0D {
            if self.bus_prefetch_count & 0x1 != 0 {
                if self.bus_prefetch_count & 0x2 != 0 {
                    self.bus_prefetch_count = ((self.bus_prefetch_count & 0xFF) >> 2) | (self.bus_prefetch_count & 0xFFFFFF00);
                    return 0;
                }
                self.bus_prefetch_count = ((self.bus_prefetch_count & 0xFF) >> 1) | (self.bus_prefetch_count & 0xFFFFFF00);
                return self.memory_wait_seq[addr] - 1;
            } else {
                self.bus_prefetch_count = 0;
                return self.memory_wait_32[addr];
            }
        } else {
            self.bus_prefetch_count = 0;
            return self.memory_wait_32[addr];
        }
    }

    fn code_ticks_access_seq_16(&mut self, address: usize) -> u8 {
        let addr = (address >> 24) & 15;

        if addr >= 0x08 && addr <= 0x0D {
            if self.bus_prefetch_count & 0x1 != 0 {
                self.bus_prefetch_count = ((self.bus_prefetch_count & 0xFF) >> 1) | (self.bus_prefetch_count & 0xFFFFFF00);
                return 0;
            } else if self.bus_prefetch_count > 0xFF {
                self.bus_prefetch_count = 0;
                return self.memory_wait[addr];
            } else {
                return self.memory_wait_seq[addr];
            }
        } else {
            self.bus_prefetch_count = 0;
            return self.memory_wait_seq[addr];
        }
    }

    fn code_ticks_access_seq_32(&mut self, address: usize) -> u8 {
        let addr = (address >> 24) & 15;

        if addr >= 0x08 && addr <= 0x0D {
            if self.bus_prefetch_count & 0x1 != 0 {
                if self.bus_prefetch_count & 0x2 != 0 {
                    self.bus_prefetch_count = ((self.bus_prefetch_count & 0xFF) >> 2) | (self.bus_prefetch_count & 0xFFFFFF00);
                    return 0;
                }
                self.bus_prefetch_count = ((self.bus_prefetch_count & 0xFF) >> 1) | (self.bus_prefetch_count & 0xFFFFFF00);
                return self.memory_wait_seq[addr];
            } else if self.bus_prefetch_count > 0xFF {
                self.bus_prefetch_count = 0;
                return self.memory_wait_32[addr];
            } else {
                return self.memory_wait_seq_32[addr];
            }
        } else {
            return self.memory_wait_seq_32[addr];
        }
    }

    fn cpu_switch_mode(&mut self, mode: i32, save_state: bool, break_loop: bool) {
        self.cpu_update_cpsr();

        match self.arm_mode {
            0x10 | 0x1F => {
                self.regs[R13_USR] = Reg::I(self.get_reg_i(13));
                self.regs[R14_USR] = Reg::I(self.get_reg_i(14));
                self.regs[17] = Reg::I(self.get_reg_i(16));
            },
            0x11 => {
                self.cpu_swap(R8_FIQ, 8);
                self.cpu_swap(R9_FIQ, 9);
                self.cpu_swap(R10_FIQ, 10);
                self.cpu_swap(R11_FIQ, 11);
                self.cpu_swap(R12_FIQ, 12);
                self.regs[R13_FIQ] = Reg::I(self.get_reg_i(13));
                self.regs[R14_FIQ] = Reg::I(self.get_reg_i(14));
                self.regs[SPSR_FIQ] = Reg::I(self.get_reg_i(17));
            },
            0x12 => {
                self.regs[R13_IRQ] = Reg::I(self.get_reg_i(13));
                self.regs[R14_IRQ] = Reg::I(self.get_reg_i(14));
                self.regs[SPSR_IRQ] = Reg::I(self.get_reg_i(17));
            },
            0x13 => {
                self.regs[R13_SVC] = Reg::I(self.get_reg_i(13));
                self.regs[R14_SVC] = Reg::I(self.get_reg_i(14));
                self.regs[SPSR_SVC] = Reg::I(self.get_reg_i(17));
            },
            0x17 => {
                self.regs[R13_ABT] = Reg::I(self.get_reg_i(13));
                self.regs[R14_ABT] = Reg::I(self.get_reg_i(14));
                self.regs[SPSR_ABT] = Reg::I(self.get_reg_i(17));
            },
            0x1B => {
                self.regs[R13_UND] = Reg::I(self.get_reg_i(13));
                self.regs[R14_UND] = Reg::I(self.get_reg_i(14));
                self.regs[SPSR_UND] = Reg::I(self.get_reg_i(17));
            },
            _ => (),
        }

        let cpsr = self.get_reg_i(16);
        let spsr = self.get_reg_i(17);

        match mode {
            0x10 | 0x1F => {
                self.regs[13] = Reg::I(self.get_reg_i(R13_USR));
                self.regs[14] = Reg::I(self.get_reg_i(R14_USR));
                self.regs[16] = Reg::I(spsr);
            },
            0x11 => {
                self.cpu_swap(8, R8_FIQ);
                self.cpu_swap(9, R9_FIQ);
                self.cpu_swap(10, R10_FIQ);
                self.cpu_swap(11, R11_FIQ);
                self.cpu_swap(12, R12_FIQ);
                self.regs[13] = Reg::I(self.get_reg_i(R13_FIQ));
                self.regs[14] = Reg::I(self.get_reg_i(R14_FIQ));
                if save_state {
                    self.regs[17] = Reg::I(cpsr);
                } else {
                    self.regs[17] = Reg::I(self.get_reg_i(SPSR_FIQ));
                }
            },
            0x12 => {
                self.regs[13] = Reg::I(self.get_reg_i(R13_IRQ));
                self.regs[14] = Reg::I(self.get_reg_i(R14_IRQ));
                self.regs[16] = Reg::I(spsr);
                if save_state {
                    self.regs[17] = Reg::I(cpsr);
                } else {
                    self.regs[17] = Reg::I(self.get_reg_i(SPSR_IRQ));
                }
            },
            0x13 => {
                self.regs[13] = Reg::I(self.get_reg_i(R13_SVC));
                self.regs[14] = Reg::I(self.get_reg_i(R14_SVC));
                self.regs[16] = Reg::I(spsr);
                if save_state {
                    self.regs[17] = Reg::I(cpsr);
                } else {
                    self.regs[17] = Reg::I(self.get_reg_i(SPSR_SVC));
                }
            },
            0x17 => {
                self.regs[13] = Reg::I(self.get_reg_i(R13_ABT));
                self.regs[14] = Reg::I(self.get_reg_i(R14_ABT));
                self.regs[16] = Reg::I(spsr);
                if save_state {
                    self.regs[17] = Reg::I(cpsr);
                } else {
                    self.regs[17] = Reg::I(self.get_reg_i(SPSR_ABT));
                }
            },
            0x1B => {
                self.regs[13] = Reg::I(self.get_reg_i(R13_UND));
                self.regs[14] = Reg::I(self.get_reg_i(R14_UND));
                self.regs[16] = Reg::I(spsr);
                if save_state {
                    self.regs[17] = Reg::I(cpsr);
                } else {
                    self.regs[17] = Reg::I(self.get_reg_i(SPSR_UND));
                }
            },
            _ => {
                println!("unsupported ARM mode: {}", mode);
            },
        }

        self.arm_mode = mode;

        self.cpu_update_flags(break_loop);
        self.cpu_update_cpsr();
    }

    fn cpu_switch_mode_s(&mut self, mode: i32, save_state: bool) {
        self.cpu_switch_mode(mode, save_state, true);
    }

    fn cpu_update_flags(&mut self,  break_loop: bool) {
        let cpsr = self.get_reg_i(16);

        self.n_flag = cpsr & 0x80000000 != 0;
        self.z_flag = cpsr & 0x40000000 != 0;
        self.c_flag = cpsr & 0x20000000 != 0;
        self.v_flag = cpsr & 0x10000000 != 0;

        self.arm_state = cpsr & 0x20 == 0;

        self.arm_irq_enable = cpsr & 0x80 == 0;

        if break_loop {
            if self.arm_irq_enable && (self.g_if & self.g_ie) != 0 && (self.g_ime & 1) != 0 {
                self.cpu_next_event = self.cpu_total_ticks;
            }
        }
    }

    fn cpu_update_flags_s(&mut self) {
        self.cpu_update_flags(true);
    }

    fn cpu_swap(&mut self, a: usize, b: usize) {
        let temp = self.get_reg_i(b);
        self.regs[b] = Reg::I(self.get_reg_i(a));
        self.regs[a] = Reg::I(temp);
    }

    fn cpu_undefined_exception(&mut self) {
        let pc = self.get_reg_i(15);

        let saved_arm_state = self.arm_state;

        self.cpu_switch_mode(0x1B, true, false);

        self.regs[14] = Reg::I(pc - (
            if saved_arm_state {
                4
            } else {
                2
            }
        ));
        self.regs[15] = Reg::I(0x04);
        self.arm_state = true;
        self.arm_irq_enable = false;
        self.arm_next_pc = 0x04;
        self.arm_prefetch();
        self.regs[15] = Reg::I(self.get_reg_i(15) + 4);
    }

    fn arm_prefetch(&mut self) {
        self.cpu_prefetch[0] = self.mem_map.read_32(self.arm_next_pc);
        self.cpu_prefetch[1] = self.mem_map.read_32(self.arm_next_pc + 4);
    }

    fn get_reg_b(&self, reg: usize) -> (u8, u8, u8, u8) {
        match self.regs[reg] {
            Reg::B(val) => val,
            _ => panic!("reg {} was not a 8 bit reg", reg),
        }
    }

    fn get_reg_w(&self, reg: usize) -> (u16, u16) {
        match self.regs[reg] {
            Reg::W(val) => val,
            _ => panic!("reg {} was not a 16 bit reg", reg),
        }
    }

    fn get_reg_i(&self, reg: usize) -> u32 {
        match self.regs[reg] {
            Reg::I(val) => val,
            _ => panic!("reg {} was not a 32 bit reg", reg),
        }
    }
}

#[derive(Copy, Clone)]
pub enum Reg {
    B((u8, u8, u8, u8)),
    W((u16, u16)),
    I(u32),
}
