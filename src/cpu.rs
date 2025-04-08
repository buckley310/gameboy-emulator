use crate::bus;
use crate::verify;
use crate::GB;

#[derive(Default)]
pub struct Flags {
	pub z: bool,
	pub n: bool,
	pub h: bool,
	pub c: bool,
}

#[derive(Default)]
pub struct CPU {
	pub a: u8,
	pub b: u8,
	pub c: u8,
	pub d: u8,
	pub e: u8,
	pub h: u8,
	pub l: u8,
	pub sp: u16,
	pub pc: u16,
	pub f: Flags,

	pub ime: bool,
	pub ime_soon: bool,

	pub halt: bool,
	pub debug: bool,
	verify: verify::Verify,
}
impl CPU {
	pub fn get_bc(&self) -> u16 {
		(self.b as u16) << 8 | (self.c as u16)
	}
	pub fn get_de(&self) -> u16 {
		(self.d as u16) << 8 | (self.e as u16)
	}
	pub fn get_hl(&self) -> u16 {
		(self.h as u16) << 8 | (self.l as u16)
	}
	pub fn set_hl(&mut self, value: u16) {
		self.h = (value >> 8) as u8;
		self.l = (value & 0xff) as u8;
	}
}
impl std::fmt::Debug for CPU {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"znhc:{}{}{}{} ",
			if self.f.z { 1 } else { 0 },
			if self.f.n { 1 } else { 0 },
			if self.f.h { 1 } else { 0 },
			if self.f.c { 1 } else { 0 },
		)?;

		write!(f, "A:{:02x} ", self.a)?;
		write!(f, "BC:{:02x}{:02x} ", self.b, self.c)?;
		write!(f, "DE:{:02x}{:02x} ", self.d, self.e)?;
		write!(f, "HL:{:02x}{:02x} ", self.h, self.l)?;
		write!(f, "SP:{:04x} PC:{:04x} ", self.sp, self.pc)?;

		write!(
			f,
			"IME:{}",
			if self.ime {
				"Y"
			} else if self.ime_soon {
				"S"
			} else {
				"N"
			}
		)
	}
}

fn get_r8(cpu: &CPU, mem: &bus::Bus, r8: u8) -> (u8, u64) {
	let data = match r8 {
		0 => cpu.b,
		1 => cpu.c,
		2 => cpu.d,
		3 => cpu.e,
		4 => cpu.h,
		5 => cpu.l,
		6 => mem.peek(cpu.get_hl()),
		7 => cpu.a,
		_ => panic!("get_r8 bad register {r8}"),
	};
	if r8 == 6 {
		(data, 1)
	} else {
		(data, 0)
	}
}

fn set_r8(cpu: &mut CPU, mem: &mut bus::Bus, r8: u8, n: u8) -> u64 {
	match r8 {
		0 => cpu.b = n,
		1 => cpu.c = n,
		2 => cpu.d = n,
		3 => cpu.e = n,
		4 => cpu.h = n,
		5 => cpu.l = n,
		6 => mem.poke(cpu.get_hl(), n),
		7 => cpu.a = n,
		_ => panic!("set_r8 bad register {r8}"),
	};
	match r8 {
		6 => 1,
		_ => 0,
	}
}

fn get_r16(cpu: &mut CPU, r16: u8) -> u16 {
	match r16 {
		0 => u16::from_le_bytes([cpu.c, cpu.b]),
		1 => u16::from_le_bytes([cpu.e, cpu.d]),
		2 => u16::from_le_bytes([cpu.l, cpu.h]),
		3 => cpu.sp,
		_ => panic!("set_r16 bad register {r16}"),
	}
}

fn set_r16(cpu: &mut CPU, r16: u8, n: u16) {
	let by = n.to_le_bytes();
	let lo = by[0];
	let hi = by[1];
	match r16 {
		0 => (cpu.b, cpu.c) = (hi, lo),
		1 => (cpu.d, cpu.e) = (hi, lo),
		2 => (cpu.h, cpu.l) = (hi, lo),
		3 => cpu.sp = n,
		_ => panic!("set_r16 bad register {r16}"),
	}
}

fn get_r16mem(cpu: &mut CPU, mem: &mut bus::Bus, r16: u8) {
	assert!(r16 <= 3);
	match r16 {
		0 => cpu.a = mem.peek(cpu.get_bc()),
		1 => cpu.a = mem.peek(cpu.get_de()),
		_ => cpu.a = mem.peek(cpu.get_hl()),
	}
	if r16 == 2 {
		cpu.set_hl(cpu.get_hl() + 1);
	} else if r16 == 3 {
		cpu.set_hl(cpu.get_hl() - 1);
	}
}

fn set_r16mem(cpu: &mut CPU, mem: &mut bus::Bus, r16: u8) {
	assert!(r16 <= 3);
	match r16 {
		0 => mem.poke(cpu.get_bc(), cpu.a),
		1 => mem.poke(cpu.get_de(), cpu.a),
		_ => mem.poke(cpu.get_hl(), cpu.a),
	}
	if r16 == 2 {
		cpu.set_hl(cpu.get_hl() + 1);
	} else if r16 == 3 {
		cpu.set_hl(cpu.get_hl() - 1);
	}
}

fn get_r16stk(cpu: &mut CPU, r16: u8) -> u16 {
	match r16 {
		0 => u16::from_le_bytes([cpu.c, cpu.b]),
		1 => u16::from_le_bytes([cpu.e, cpu.d]),
		2 => u16::from_le_bytes([cpu.l, cpu.h]),
		3 => {
			((cpu.a as u16) << 8)
				| ((cpu.f.z as u16) << 7)
				| ((cpu.f.n as u16) << 6)
				| ((cpu.f.h as u16) << 5)
				| ((cpu.f.c as u16) << 4)
		}
		_ => panic!("set_r16 bad register {r16}"),
	}
}

fn set_r16stk(cpu: &mut CPU, r16: u8, n: u16) {
	let by = n.to_le_bytes();
	let lo = by[0];
	let hi = by[1];
	match r16 {
		0 => (cpu.b, cpu.c) = (hi, lo),
		1 => (cpu.d, cpu.e) = (hi, lo),
		2 => (cpu.h, cpu.l) = (hi, lo),
		3 => {
			cpu.a = (n >> 8) as u8;
			cpu.f.z = n & (1 << 7) != 0;
			cpu.f.n = n & (1 << 6) != 0;
			cpu.f.h = n & (1 << 5) != 0;
			cpu.f.c = n & (1 << 4) != 0;
		}
		_ => panic!("set_r16 bad register {r16}"),
	}
}

fn get_cond(f: &Flags, cond: u8) -> bool {
	match cond {
		0 => !f.z,
		1 => f.z,
		2 => !f.c,
		3 => f.c,
		_ => panic!("unknown condition"),
	}
}

fn u8_as_signed_ofs(ofs: u8) -> u16 {
	let mut ofs = ofs as u16;
	if ofs >= 0x80 {
		ofs |= 0xff00;
	}
	ofs
}

pub fn cycle(gb: &mut GB) -> u64 {
	let cpu = &mut gb.cpu;
	let mem = &mut gb.bus;
	if cpu.halt {
		return 1;
	}

	let opcode = mem.peek(cpu.pc);

	let imm8 = mem.peek(cpu.pc.overflowing_add(1).0);
	let imm8_2 = mem.peek(cpu.pc.overflowing_add(2).0);
	let imm16 = ((imm8_2 as u16) << 8) | (imm8 as u16);

	if cpu.ime {
		let inter = mem.io.interrupt & mem.io.ie & 0b11111;
		if inter > 0 {
			cpu.ime = false;
			cpu.ime_soon = false;
			cpu.sp -= 2;
			mem.poke16(cpu.sp, cpu.pc);
			for bit in 0..5 {
				if inter & (1 << bit) > 0 {
					if cpu.debug {
						println!("triggering interrupt: {inter:08b}");
					}
					mem.io.interrupt &= !(1 << bit);
					cpu.pc = 0x40 + 8 * bit;
					return 5;
				}
			}
			panic!("Interrupt bug!");
		}
	}

	if cpu.debug {
		println!(
			"{cpu:>2x?} - {}",
			cpu.verify.disasm([opcode, imm8, imm8_2]).0
		);
	}

	let mut ime_enabled_this_cycle = false;

	let (bytes, mcycles) = match opcode {
		0b_00_000000..=0b_00_111111 => {
			// Block 0
			//

			// split based on low 3 bits (octal column)
			match opcode & 0b111 {
				0 => {
					match opcode {
						// 00---000
						0b00_000_000 => (1, 1),
						0b00_001_000 => {
							mem.poke16(imm16, cpu.sp);
							(3, 5)
						}
						0b00_010_000 => {
							if mem.io.speed_switch & 1 != 0 {
								mem.io.speed_switch ^= 0x81; // zero low bit. flip high bit
							}
							cpu.halt = true; // TODO: might want to add more here
							todo!("SPEED SWITCH");
							// (2, 1)
						}
						0b00_011_000 => {
							cpu.pc = cpu.pc.overflowing_add(u8_as_signed_ofs(imm8)).0;
							(2, 3)
						}
						_ => {
							// 0b001--000
							if get_cond(&cpu.f, (opcode >> 3) & 0b11) {
								cpu.pc = cpu.pc.overflowing_add(u8_as_signed_ofs(imm8)).0;
								(2, 3)
							} else {
								(2, 2)
							}
						}
					}
				}
				1 => {
					// 00---001
					if opcode & 0b1000 == 0 {
						// 00--0001
						let r16 = opcode >> 4;
						set_r16(cpu, r16, imm16);
						(3, 3)
					} else {
						// 00--1001
						let value = get_r16(cpu, opcode >> 4);
						let old_hl = cpu.get_hl();
						cpu.set_hl(old_hl.overflowing_add(value).0);
						cpu.f.n = false;
						cpu.f.h = (old_hl & 0xfff) + (value & 0xfff) > 0xfff;
						cpu.f.c = old_hl.overflowing_add(value).1;
						(1, 2)
					}
				}
				2 => {
					// 00---010
					if opcode & 0b1000 == 0 {
						// 00--0010
						set_r16mem(cpu, mem, opcode >> 4);
						(1, 2)
					} else {
						// 00--1010
						get_r16mem(cpu, mem, opcode >> 4);
						(1, 2)
					}
				}
				3 => {
					// 00---011
					if opcode & 0b1000 == 0 {
						// 00--0011
						let r16 = (opcode >> 4) & 0b11;
						let value = get_r16(cpu, r16).overflowing_add(1).0;
						set_r16(cpu, r16, value);
						(1, 2)
					} else {
						// 00--1011
						let r16 = (opcode >> 4) & 0b11;
						let value = get_r16(cpu, r16).overflowing_sub(1).0;
						set_r16(cpu, r16, value);
						(1, 2)
					}
				}
				4 => {
					// 00---100
					let (r8, r8_cost) = get_r8(cpu, mem, opcode >> 3);
					let n = r8.overflowing_add(1).0;
					let r8dst_cost = set_r8(cpu, mem, opcode >> 3, n);
					cpu.f.z = n == 0;
					cpu.f.n = false;
					cpu.f.h = n & 0xf == 0;
					(1, 1 + r8_cost + r8dst_cost)
				}
				5 => {
					// 00---101
					let (r8, r8_cost) = get_r8(cpu, mem, opcode >> 3);
					let n = r8.overflowing_sub(1).0;
					let r8dst_cost = set_r8(cpu, mem, opcode >> 3, n);
					cpu.f.z = n == 0;
					cpu.f.n = true;
					cpu.f.h = n & 0xf == 0xf;
					(1, 1 + r8_cost + r8dst_cost)
				}
				6 => {
					// 00---110
					let r8 = opcode >> 3;
					let r8dst_cost = set_r8(cpu, mem, r8, imm8);
					(2, 2 + r8dst_cost)
				}
				7 | _ => {
					// 00---111
					match opcode {
						0b00_000_111 => {
							cpu.a = (cpu.a >> 7) | (cpu.a << 1);
							cpu.f.z = false;
							cpu.f.n = false;
							cpu.f.h = false;
							cpu.f.c = cpu.a & 1 != 0;
							(1, 1)
						}
						0b00_001_111 => {
							cpu.f.c = cpu.a & 1 != 0;
							cpu.a >>= 1;
							if cpu.f.c {
								cpu.a |= 0x80;
							}
							cpu.f.z = false;
							cpu.f.n = false;
							cpu.f.h = false;
							(1, 1)
						}
						0b00_010_111 => {
							let carry = cpu.f.c;
							cpu.f.c = cpu.a & 0x80 != 0;
							cpu.a <<= 1;
							if carry {
								cpu.a |= 1;
							}
							cpu.f.z = false;
							cpu.f.n = false;
							cpu.f.h = false;
							(1, 1)
						}
						0b00_011_111 => {
							let old_a = cpu.a;
							cpu.a >>= 1;
							if cpu.f.c {
								cpu.a |= 0b10000000
							}
							cpu.f.z = false;
							cpu.f.n = false;
							cpu.f.h = false;
							cpu.f.c = old_a & 1 != 0;
							(1, 1)
						}
						0b00_100_111 => {
							let mut adj = 0;
							if cpu.f.n {
								if cpu.f.h {
									adj += 0x6;
								}
								if cpu.f.c {
									adj += 0x60;
								}
								cpu.a = cpu.a.overflowing_sub(adj).0;
							} else {
								if cpu.f.h || (cpu.a & 0xf) > 0x9 {
									adj += 0x6;
								}
								if cpu.f.c || cpu.a > 0x99 {
									adj += 0x60;
									cpu.f.c = true;
								}
								cpu.a = cpu.a.overflowing_add(adj).0;
							}
							cpu.f.z = cpu.a == 0;
							cpu.f.h = false;
							(1, 1)
						}
						0b00_101_111 => {
							cpu.a = !cpu.a;
							cpu.f.n = true;
							cpu.f.h = true;
							(1, 1)
						}
						0b00_110_111 => {
							cpu.f.n = false;
							cpu.f.h = false;
							cpu.f.c = true;
							(1, 1)
						}
						0b00_111_111 | _ => {
							cpu.f.n = false;
							cpu.f.h = false;
							cpu.f.c = !cpu.f.c;
							(1, 1)
						}
					}
				}
			}
		}
		0b_01_000000..=0b_01_111111 => {
			// Block 1
			//
			if opcode == 0x76 {
				cpu.halt = true;
				(1, 1)
			} else {
				let r8_src = opcode & 0b111;
				let r8_dst = (opcode & 0b00111000) >> 3;
				let (r8, r8_cost) = get_r8(cpu, mem, r8_src);
				let r8dst_cost = set_r8(cpu, mem, r8_dst, r8);
				(1, 1 + r8_cost + r8dst_cost)
			}
		}
		0b_10_000000..=0b_10_111111 => {
			// Block 2
			//
			match (opcode & 0b00111000) >> 3 {
				0 => {
					let (r8, r8_cost) = get_r8(cpu, mem, opcode & 0b111);

					let new_value = cpu.a.overflowing_add(r8);

					cpu.f.z = new_value.0 == 0;
					cpu.f.n = false;
					cpu.f.h = (r8 & 0xf) + (cpu.a & 0xf) > 0xf;
					cpu.f.c = new_value.1;

					cpu.a = new_value.0;

					(1, 1 + r8_cost)
				}
				1 => {
					let carry = if cpu.f.c { 1 } else { 0 };
					let (r8, r8_cost) = get_r8(cpu, mem, opcode & 0b111);
					let r8c = r8.overflowing_add(carry);

					let new_value = cpu.a.overflowing_add(r8c.0);

					cpu.f.z = new_value.0 == 0;
					cpu.f.n = false;
					cpu.f.h = (r8 & 0xf) + (cpu.a & 0xf) + carry > 0xf;
					cpu.f.c = new_value.1 || r8c.1;

					cpu.a = new_value.0;

					(1, 1 + r8_cost)
				}
				2 => {
					let (r8, r8_cost) = get_r8(cpu, mem, opcode & 0b111);
					let new_value = cpu.a.overflowing_sub(r8);

					// if zero, then they were equal
					cpu.f.z = cpu.a == r8;
					// always 1
					cpu.f.n = true;
					// Set if borrow from bit 4
					cpu.f.h = r8 & 0xf > cpu.a & 0xf;
					// Set if borrow
					cpu.f.c = new_value.1;

					cpu.a = new_value.0;
					(1, 1 + r8_cost)
				}
				3 => {
					let carry = if cpu.f.c { 1 } else { 0 };
					let (r8, r8_cost) = get_r8(cpu, mem, opcode & 0b111);
					let r8_plus_carry = r8.overflowing_add(carry);

					let new_value = cpu.a.overflowing_sub(r8_plus_carry.0);

					// always 1
					cpu.f.n = true;
					// Set if borrow from bit 4
					cpu.f.h = (cpu.a & 0xf)
						.overflowing_sub(r8 & 0xf)
						.0
						.overflowing_sub(carry)
						.0 > 0xf;
					// Set if borrow
					cpu.f.c = new_value.1 || r8_plus_carry.1;

					cpu.a = new_value.0;

					cpu.f.z = cpu.a == 0;

					(1, 1 + r8_cost)
				}
				4 => {
					let (r8, r8_cost) = get_r8(cpu, mem, opcode & 0b111);
					cpu.a &= r8;
					cpu.f.z = cpu.a == 0;
					cpu.f.n = false;
					cpu.f.h = true;
					cpu.f.c = false;
					(1, 1 + r8_cost)
				}
				5 => {
					let (r8, r8_cost) = get_r8(cpu, mem, opcode & 0b111);
					cpu.a ^= r8;
					cpu.f.z = cpu.a == 0;
					cpu.f.n = false;
					cpu.f.h = false;
					cpu.f.c = false;
					(1, 1 + r8_cost)
				}
				6 => {
					let (r8, r8_cost) = get_r8(cpu, mem, opcode & 0b111);
					cpu.a |= r8;
					cpu.f.z = cpu.a == 0;
					cpu.f.n = false;
					cpu.f.h = false;
					cpu.f.c = false;
					(1, 1 + r8_cost)
				}
				7 | _ => {
					let (r8, r8_cost) = get_r8(cpu, mem, opcode & 0b111);

					// calculate by subtracting (a - r8v)

					// if zero, then they were equal
					cpu.f.z = cpu.a == r8;
					// always 1
					cpu.f.n = true;
					// Set if borrow from bit 4
					cpu.f.h = r8 & 0xf > cpu.a & 0xf;
					// Set if borrow
					cpu.f.c = r8 > cpu.a;

					(1, 1 + r8_cost)
				}
			}
		}
		0b_11_000000..=0b_11_111111 => {
			// Block 3
			//
			match opcode & 0b00111111 {
				0b000110 => {
					let value = cpu.a.overflowing_add(imm8);
					cpu.f.z = value.0 == 0;
					cpu.f.n = false;
					cpu.f.h = (cpu.a & 0xf) + (imm8 & 0xf) > 0xf;
					cpu.f.c = value.1;
					cpu.a = value.0;
					(2, 2)
				}
				0b001110 => {
					let carry_flag = if cpu.f.c { 1 } else { 0 };

					let imm8_plus_carry = imm8.overflowing_add(carry_flag);

					let new_value = cpu.a.overflowing_add(imm8_plus_carry.0);

					cpu.f.z = new_value.0 == 0;
					cpu.f.n = false;
					cpu.f.h = (imm8 & 0xf) + (cpu.a & 0xf) + carry_flag > 0xf;
					cpu.f.c = new_value.1 || imm8_plus_carry.1;

					cpu.a = new_value.0;

					(2, 2)
				}
				0b010110 => {
					let value = cpu.a.overflowing_sub(imm8);
					cpu.f.z = value.0 == 0;
					cpu.f.n = true;
					cpu.f.h = (cpu.a & 0xf) < (imm8 & 0xf);
					cpu.f.c = value.1;
					cpu.a = value.0;
					(2, 2)
				}
				0b011110 => {
					let carry = if cpu.f.c { 1 } else { 0 };
					let imm8_plus_carry = imm8.overflowing_add(carry);

					let new_value = cpu.a.overflowing_sub(imm8_plus_carry.0);

					cpu.f.n = true;
					// Set if borrow from bit 4
					cpu.f.h = (cpu.a & 0xf)
						.overflowing_sub(imm8 & 0xf)
						.0
						.overflowing_sub(carry)
						.0 > 0xf;
					// Set if borrow
					cpu.f.c = new_value.1 || imm8_plus_carry.1;

					cpu.a = new_value.0;

					cpu.f.z = cpu.a == 0;

					(2, 2)
				}
				0b100110 => {
					cpu.a &= imm8;
					cpu.f.z = cpu.a == 0;
					cpu.f.n = false;
					cpu.f.h = true;
					cpu.f.c = false;
					(2, 2)
				}
				0b101110 => {
					cpu.a ^= imm8;
					cpu.f.z = cpu.a == 0;
					cpu.f.n = false;
					cpu.f.h = false;
					cpu.f.c = false;
					(2, 2)
				}
				0b110110 => {
					cpu.a |= imm8;
					cpu.f.z = cpu.a == 0;
					cpu.f.n = false;
					cpu.f.h = false;
					cpu.f.c = false;
					(2, 2)
				}
				0b111110 => {
					// calculate by subtracting (a - imm8)

					// if zero, then they were equal
					cpu.f.z = cpu.a == imm8;
					// always 1
					cpu.f.n = true;
					// Set if borrow from bit 4
					cpu.f.h = imm8 & 0xf > cpu.a & 0xf;
					// Set if borrow
					cpu.f.c = imm8 > cpu.a;
					(2, 2)
				}

				0b001001 => {
					cpu.pc = mem.peek16(cpu.sp);
					cpu.sp += 2;
					(0, 4)
				}
				0b011001 => {
					cpu.ime = true;
					cpu.ime_soon = false;
					cpu.pc = mem.peek16(cpu.sp);
					cpu.sp += 2;
					(0, 4)
				}
				0b000011 => {
					cpu.pc = imm16;
					(0, 4)
				}
				0b101001 => {
					cpu.pc = ((cpu.h as u16) << 8) | (cpu.l as u16);
					(0, 1)
				}
				0b001101 => {
					cpu.sp -= 2;
					mem.poke16(cpu.sp, cpu.pc + 3);
					cpu.pc = imm16;
					(0, 6)
				}
				0b000000 | 0b001000 | 0b010000 | 0b011000 => {
					if get_cond(&cpu.f, (opcode >> 3) & 0b11) {
						cpu.pc = mem.peek16(cpu.sp);
						cpu.sp += 2;
						(0, 5)
					} else {
						(1, 2)
					}
				}
				0b000010 | 0b001010 | 0b010010 | 0b011010 => {
					let cond = (opcode >> 3) & 0b11;
					if get_cond(&cpu.f, cond) {
						cpu.pc = imm16;
						(0, 4)
					} else {
						(3, 3)
					}
				}
				0b000100 | 0b001100 | 0b010100 | 0b011100 => {
					if get_cond(&cpu.f, (opcode >> 3) & 0b11) {
						cpu.sp -= 2;
						mem.poke16(cpu.sp, cpu.pc + 3);
						cpu.pc = imm16;
						(0, 6)
					} else {
						(3, 3)
					}
				}
				0b000001 | 0b010001 | 0b100001 | 0b110001 => {
					let r16 = (opcode >> 4) & 0b11;
					set_r16stk(cpu, r16, mem.peek16(cpu.sp));
					cpu.sp += 2;
					(1, 3)
				}
				0b000101 | 0b010101 | 0b100101 | 0b110101 => {
					let r16 = (opcode >> 4) & 0b11;
					cpu.sp -= 2;
					mem.poke16(cpu.sp, get_r16stk(cpu, r16));
					(1, 4)
				}
				0b001011 => {
					let b3_id = (imm8 >> 3) & 0b111;
					let r8_id = imm8 & 0b111;
					match imm8 >> 6 {
						1 => {
							let (r8, r8_cost) = get_r8(cpu, mem, r8_id);
							cpu.f.z = r8 & (1 << b3_id) == 0;
							cpu.f.n = false;
							cpu.f.h = true;
							(2, 2 + r8_cost)
						}
						2 => {
							let (r8, r8_cost) = get_r8(cpu, mem, r8_id);
							let r8dst_cost = set_r8(cpu, mem, r8_id, r8 & (!(1 << b3_id)));
							(2, 2 + r8_cost + r8dst_cost)
						}
						3 => {
							let (mut r8, r8_cost) = get_r8(cpu, mem, r8_id);
							r8 |= 1 << b3_id;
							let r8dst_cost = set_r8(cpu, mem, r8_id, r8);
							(2, 2 + r8_cost + r8dst_cost)
						}
						0 | _ => match b3_id {
							0 => {
								let (mut r8, r8_cost) = get_r8(cpu, mem, r8_id);
								cpu.f.c = r8 & 0x80 != 0;
								r8 <<= 1;
								if cpu.f.c {
									r8 |= 1;
								}
								let r8dst_cost = set_r8(cpu, mem, r8_id, r8);
								cpu.f.z = r8 == 0;
								cpu.f.n = false;
								cpu.f.h = false;
								(2, 2 + r8_cost + r8dst_cost)
							}
							1 => {
								let (mut r8, r8_cost) = get_r8(cpu, mem, r8_id);
								cpu.f.c = r8 & 1 != 0;
								r8 >>= 1;
								if cpu.f.c {
									r8 |= 0x80;
								}
								let r8dst_cost = set_r8(cpu, mem, r8_id, r8);
								cpu.f.z = r8 == 0;
								cpu.f.n = false;
								cpu.f.h = false;
								(2, 2 + r8_cost + r8dst_cost)
							}
							2 => {
								let carry = cpu.f.c;
								let (mut r8, r8_cost) = get_r8(cpu, mem, r8_id);
								cpu.f.c = r8 & 0x80 != 0;
								r8 <<= 1;
								if carry {
									r8 |= 1;
								}
								let r8dst_cost = set_r8(cpu, mem, r8_id, r8);
								cpu.f.n = false;
								cpu.f.h = false;
								cpu.f.z = r8 == 0;
								(2, 2 + r8_cost + r8dst_cost)
							}
							3 => {
								let (r8, r8_cost) = get_r8(cpu, mem, r8_id);
								let mut n = r8 >> 1;
								if cpu.f.c {
									n |= 0b10000000;
								}
								let r8dst_cost = set_r8(cpu, mem, r8_id, n);
								cpu.f.z = n == 0;
								cpu.f.n = false;
								cpu.f.h = false;
								cpu.f.c = r8 & 1 != 0;
								(2, 2 + r8_cost + r8dst_cost)
							}
							4 => {
								let (mut r8, r8_cost) = get_r8(cpu, mem, r8_id);
								cpu.f.c = r8 & 0x80 != 0;
								r8 <<= 1;
								let r8dst_cost = set_r8(cpu, mem, r8_id, r8);
								cpu.f.z = r8 == 0;
								cpu.f.h = false;
								cpu.f.n = false;
								(2, 2 + r8_cost + r8dst_cost)
							}
							5 => {
								let (mut r8, r8_cost) = get_r8(cpu, mem, r8_id);
								let sign_bit = r8 & 0x80;
								cpu.f.c = r8 & 1 != 0;
								r8 >>= 1;
								r8 |= sign_bit;
								let r8dst_cost = set_r8(cpu, mem, r8_id, r8);
								cpu.f.z = r8 == 0;
								cpu.f.n = false;
								cpu.f.h = false;
								(2, 2 + r8_cost + r8dst_cost)
							}
							6 => {
								let (mut r8, r8_cost) = get_r8(cpu, mem, r8_id);
								r8 = (r8 >> 4) | (r8 << 4);
								let r8dst_cost = set_r8(cpu, mem, r8_id, r8);
								cpu.f.z = r8 == 0;
								cpu.f.n = false;
								cpu.f.h = false;
								cpu.f.c = false;
								(2, 2 + r8_cost + r8dst_cost)
							}
							7 | _ => {
								let (r8, r8_cost) = get_r8(cpu, mem, r8_id);
								let n = r8 >> 1;
								let r8dst_cost = set_r8(cpu, mem, r8_id, n);
								cpu.f.z = n == 0;
								cpu.f.n = false;
								cpu.f.h = false;
								cpu.f.c = r8 & 1 != 0;
								(2, 2 + r8_cost + r8dst_cost)
							}
						},
					}
				}

				0b100010 => {
					mem.poke(0xFF00 | (cpu.c as u16), cpu.a);
					(1, 2)
				}
				0b100000 => {
					mem.poke(0xFF00 | (imm8 as u16), cpu.a);
					(2, 3)
				}
				0b101010 => {
					mem.poke(imm16, cpu.a);
					(3, 4)
				}
				0b110010 => {
					cpu.a = mem.peek(0xff00 | (cpu.c as u16));
					(1, 2)
				}
				0b110000 => {
					cpu.a = mem.peek(0xFF00 | (imm8 as u16));
					(2, 3)
				}
				0b111010 => {
					let value = mem.peek(imm16);
					cpu.a = value;
					(3, 4)
				}
				0b101000 => {
					let uadd = u8_as_signed_ofs(imm8);
					let old_sp = cpu.sp;
					cpu.sp = old_sp.overflowing_add(uadd).0;
					cpu.f.z = false;
					cpu.f.n = false;
					cpu.f.h = (old_sp & 0xf) + (uadd & 0xf) > 0xf;
					cpu.f.c = (old_sp & 0xff) + (uadd & 0xff) > 0xff;
					(2, 4)
				}
				0b111000 => {
					let uadd = u8_as_signed_ofs(imm8);
					cpu.set_hl(cpu.sp.overflowing_add(uadd).0);
					cpu.f.z = false;
					cpu.f.n = false;
					cpu.f.h = (cpu.sp & 0xf) + (uadd & 0xf) > 0xf;
					cpu.f.c = (cpu.sp & 0xff) + (uadd & 0xff) > 0xff;
					(2, 3)
				}
				0b111001 => {
					cpu.sp = cpu.get_hl();
					(1, 2)
				}
				0b110011 => {
					cpu.ime = false;
					cpu.ime_soon = false;
					(1, 1)
				}
				0b111011 => {
					ime_enabled_this_cycle = true;
					(1, 1)
				}

				// TODO: invalid opcodes:
				// $D3, $DB, $DD, $E3, $E4, $EB, $EC, $ED, $F4, $FC, and $FD

				//
				_ => {
					assert!(opcode & 0b11000111 == 0b11000111);
					cpu.sp -= 2;
					mem.poke16(cpu.sp, cpu.pc + 1);
					cpu.pc = (opcode as u16) & 0b111000;
					(0, 4)
				}
			}
		}
	};

	cpu.verify.cycles(mcycles, (opcode, imm8));

	if cpu.ime_soon {
		cpu.ime = true;
		cpu.ime_soon = false;
	} else if ime_enabled_this_cycle {
		cpu.ime_soon = true;
	}

	cpu.pc += bytes;
	mcycles
}
