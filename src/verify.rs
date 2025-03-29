use crate::opcodes::Opcodes;

pub struct Verify {
	cycles_cb: [Vec<u64>; 0x100],
	cycles_un: [Vec<u64>; 0x100],
	instructions: [String; 0x200],
	instruction_size: [u16; 0x100],
}

impl std::default::Default for Verify {
	fn default() -> Verify {
		let opcodes = Opcodes::new();
		let mut cycles_cb: [Vec<u64>; 0x100] = core::array::from_fn(|_| vec![]);
		let mut cycles_un: [Vec<u64>; 0x100] = core::array::from_fn(|_| vec![]);
		let mut instructions: [String; 0x200] = core::array::from_fn(|_| String::new());
		let mut instruction_size = [0; 0x100];
		for i in 0..0x100 {
			let un_inst = opcodes.unprefixed.get(&format!("{i:#04X}")).unwrap();
			let cb_inst = opcodes.cbprefixed.get(&format!("{i:#04X}")).unwrap();
			cycles_un[i] = un_inst.cycles.clone();
			cycles_cb[i] = cb_inst.cycles.clone();
			instruction_size[i] = if i == 0xcb { 2 } else { un_inst.bytes };
			{
				let mut inst = un_inst.mnemonic.clone();
				for oper in &un_inst.operands {
					inst.push(' ');
					if oper.immediate {
						inst.push_str(&oper.name);
					} else {
						inst.push('[');
						inst.push_str(&oper.name);
						inst.push(']');
					}
				}
				instructions[i] = inst;
			}
			{
				let mut inst = un_inst.mnemonic.clone();
				for oper in &un_inst.operands {
					inst.push(' ');
					if oper.immediate {
						inst.push_str(&oper.name);
					} else {
						inst.push('[');
						inst.push_str(&oper.name);
						inst.push(']');
					}
				}
				instructions[i + 0x100] = inst;
			}
		}
		Verify {
			cycles_cb,
			cycles_un,
			instructions,
			instruction_size,
		}
	}
}

impl Verify {
	pub fn cycles(&self, cyc: u64, (b1, b2): (u8, u8)) {
		let tcycles = cyc * 4;
		if b1 == 0xcb {
			let timings = self.cycles_cb[b2 as usize].clone();
			if !timings.contains(&tcycles) {
				println!("wrong cycles {tcycles}. CB/{b2} should be one of {timings:?}");
			}
		} else {
			let timings = self.cycles_un[b1 as usize].clone();
			if !timings.contains(&tcycles) {
				println!("wrong cycles {tcycles}. {b1} should be one of {timings:?}");
			}
		}
	}
	pub fn disasm(&self, mem: [u8; 3]) -> (String, u16) {
		let inst_index = if mem[0] == 0xcb {
			mem[1] as usize + 0x100
		} else {
			mem[0] as usize
		};

		let inst = &self.instructions[inst_index];
		let size = self.instruction_size[mem[0] as usize];

		let mut r = mem
			.iter()
			.take(size as usize)
			.map(|x| format!("{x:02x}"))
			.collect::<Vec<String>>()
			.join(" ");

		while r.len() < 8 {
			r.push(' ');
		}
		r.push_str(" - ");

		r.push_str(inst);

		(r, size)
	}
}
