use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

pub mod audio;
pub mod bus;
pub mod cart;
pub mod cpu;
pub mod ioreg;
pub mod ui;
pub mod video;

pub struct GB {
	bus: bus::Bus,
	cpu: cpu::CPU,
	framebuffer: [u8; 160 * 144 * 3],
}
impl std::default::Default for GB {
	fn default() -> GB {
		GB {
			bus: bus::Bus::default(),
			cpu: cpu::CPU::default(),
			framebuffer: [30; 160 * 144 * 3],
		}
	}
}

fn slow_down(real_elapsed: Duration, elapsed_dots: u64) {
	const DOTS_HZ: u32 = 1 << 22;
	let ingame_elapsed = Duration::from_secs(elapsed_dots) / DOTS_HZ;
	sleep(ingame_elapsed.saturating_sub(real_elapsed));
}

fn main() {
	let mut gb = GB::default();
	let mut rom: Vec<u8> = vec![];

	let audio_device = audio::init_audio();
	let apu = audio::APU::new(&audio_device);
	gb.bus.io.audio_params = apu.audio_params;

	let argv: Vec<String> = std::env::args().collect();
	for arg in &argv[1..] {
		let mut arg_iter = arg.chars();
		if arg_iter.next().unwrap() == '-' {
			for arg_char in arg_iter {
				match arg_char {
					'c' => gb.cpu.debug = true,
					'i' => gb.bus.io.debug = true,
					'b' => gb.bus.cart.debug_bank_switch = true,
					_ => panic!("Bad commandline flag: {arg}"),
				}
			}
		} else {
			rom = std::fs::File::open(&std::path::Path::new(arg))
				.unwrap()
				.bytes()
				.map(|x| x.unwrap())
				.collect();
		}
	}

	let mut ui = ui::UI::default();

	assert!(rom.len() > 0);
	gb.bus.cart.load_rom(&rom);

	let lgb = Arc::new(Mutex::new(gb));

	let start = Instant::now();
	const DOTS_PER_SCANLINE: u64 = 456;
	let mut sprites = vec![];
	let mut dots = 0;
	let mut dots_cpu = 0;

	let mut play = true;
	while play {
		slow_down(start.elapsed(), dots);

		let mut gb = lgb.lock().unwrap();

		ui.draw(&mut gb, &mut play);

		while play {
			dots += 1;

			// Advance screen
			if gb.bus.io.lcdc & 0x80 != 0 {
				if gb.bus.io.lx >= DOTS_PER_SCANLINE {
					gb.bus.io.lx = 0;
					gb.bus.io.ly += 1;
					if gb.bus.io.ly >= 154 {
						gb.bus.io.ly = 0;
					}

					if gb.bus.io.ly == gb.bus.io.lyc {
						gb.bus.io.interrupt |= ioreg::INT_LCD;
						gb.cpu.halt = false; // TODO: this should wake the system, right?
					}

					if gb.bus.io.ly == 144 {
						gb.bus.io.interrupt |= ioreg::INT_VBLANK;
						gb.cpu.halt = false;
						break;
					}
				}
				if gb.bus.io.lx == 80 {
					sprites = video::oam_scan(&gb);
				}
				{
					let tmp = gb.bus.io.lx;
					video::render_dot(&mut gb, tmp, &sprites);
				}
				gb.bus.io.lx += 1;
			} else if dots & 0xffff == 0 {
				// if lcd is off, break "sometimes" to draw
				break;
			}

			// Advance CPU
			if dots_cpu < dots {
				let mcycles = cpu::cycle(&mut gb);
				dots_cpu += mcycles * 4;
				if gb.bus.io.advance_counter_div(mcycles) {
					gb.cpu.halt = false;
				}
			}
		}
	}
}
