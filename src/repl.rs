use crate::verify::Verify;
use crate::GB;
use std::io::{stdin, stdout, Write};
use std::sync::{Arc, Mutex};

fn disasm_multi(verify: &Verify, gb: &GB, mut addr: u16, n: usize) -> Vec<String> {
	let mut r = vec![];
	for _ in 0..n {
		let (a, b) = verify.disasm([
			gb.bus.peek(addr.overflowing_add(0).0),
			gb.bus.peek(addr.overflowing_add(1).0),
			gb.bus.peek(addr.overflowing_add(2).0),
		]);

		let mut new_s = format!("{addr:04x}: ");
		new_s.push_str(&a);
		r.push(new_s);

		addr = addr.overflowing_add(b).0;
	}
	r
}

pub fn go(lgb: Arc<Mutex<GB>>) {
	let verify = Verify::default();
	loop {
		print!(">>> ");
		stdout().flush().unwrap();
		let mut buf = String::new();
		stdin().read_line(&mut buf).unwrap();

		let words: Vec<String> = buf.split_whitespace().map(|s| s.to_string()).collect();
		if words.len() == 0 {
			continue;
		}
		let mut gb = lgb.lock().unwrap();
		match words[0].as_str() {
			"r" => println!("{:?}", gb.cpu),
			"p" => {
				gb.on_break ^= true;
				println!("Paused: {}", gb.on_break);
			}
			"si" => {
				gb.on_break = false;
				gb.single_step = true;
			}
			"c" => {
				println!("\n-------- BANKS --------");
				println!(
					"ROM: {:02x}, EXRAM: {:02x}, EXRAM_ENABLE: {}, ADVANCED_BANK_MODE: {}",
					gb.bus.rom_bank, gb.bus.exram_bank, gb.bus.exram_enable, gb.bus.bank_mode,
				);
				println!("\n-------- REGISTERS --------");
				println!("{:?}", gb.cpu);
				println!("\n-------- DISASM --------");
				println!("{}", disasm_multi(&verify, &gb, gb.cpu.pc, 10).join("\n"));
				println!("\n-------- STACK --------");
				for i in (0..20).step_by(2) {
					let addr = gb.cpu.sp.overflowing_add(i).0;
					let data = gb.bus.peek16(addr);
					println!("{addr:04x}: {:02x?} ({data:04x})", data.to_le_bytes());
				}
				println!();
			}
			"pd" => {
				let n = if words.len() > 2 {
					usize::from_str_radix(&words[2], 16).unwrap_or(10)
				} else {
					10
				};

				let addr = if words.len() > 1 {
					u16::from_str_radix(&words[1], 16).unwrap_or(gb.cpu.pc)
				} else {
					gb.cpu.pc
				};

				println!("{}", disasm_multi(&verify, &gb, addr, n).join("\n"));
			}
			"v" => {
				if words.len() > 1 {
					match words[1].to_lowercase().as_str() {
						"bank" => gb.bus.debug_bank_switch ^= true,
						"cpu" => gb.cpu.debug ^= true,
						"io" => gb.bus.io.debug ^= true,
						_ => {}
					}
				}
				println!("\nverbose:");
				println!("          IO:{}", gb.bus.io.debug);
				println!("         CPU:{}", gb.cpu.debug);
				println!("        BANK:{}", gb.bus.debug_bank_switch);
				println!("\n");
			}
			"d" => {
				if words.len() >= 2 {
					if let Ok(n) = u16::from_str_radix(&words[1], 16) {
						gb.breakpoints = gb
							.breakpoints
							.iter()
							.filter(|x| **x != n)
							.map(|x| *x)
							.collect();
					} else {
						println!("bad addr")
					}
				} else {
					println!("missing addr")
				}
			}
			"b" => {
				if words.len() >= 2 {
					if let Ok(n) = u16::from_str_radix(&words[1], 16) {
						if !gb.breakpoints.contains(&n) {
							gb.breakpoints.push(n);
						}
					} else {
						println!("bad addr")
					}
				} else {
					println!("{:04x?}", gb.breakpoints);
				}
			}
			"poke" => {
				if words.len() < 3 {
					continue;
				}
				let Ok(addr) = u16::from_str_radix(&words[1], 16) else {
					continue;
				};
				let Ok(data) = u8::from_str_radix(&words[2], 16) else {
					continue;
				};
				gb.bus.poke(addr, data);
				println!("OK");
			}
			"hd" => {
				let mut addr = gb.cpu.sp;
				let mut size = 0x20;
				if words.len() >= 2 {
					addr = u16::from_str_radix(&words[1], 16).unwrap_or(addr);
				}
				if words.len() >= 3 {
					size = u16::from_str_radix(&words[2], 16).unwrap_or(size);
				}

				let end = match addr.overflowing_add(size) {
					(_, true) => 0xffff,
					(e, false) => e,
				};

				for loc in (addr..end).step_by(2) {
					let data = gb.bus.peek16(loc);
					println!("  {loc:04x}: {:02x?} ({data:04x})", data.to_le_bytes());
				}
			}
			_ => println!("Unknown command"),
		}
	}
}
