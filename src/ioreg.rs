use crate::audio::AudioParams;
use std::sync::{Arc, Mutex};

pub const INT_VBLANK: u8 = 1;
pub const INT_LCD: u8 = 2;
pub const INT_TIMER: u8 = 4;
pub const INT_SERIAL: u8 = 8;
pub const INT_JOYPAD: u8 = 16;

fn name_of(addr: usize) -> &'static str {
	match addr {
		0xFF00 => "P1/JOYP",          // Joypad
		0xFF01 => "SB",               // Serial transfer data
		0xFF02 => "SC",               // Serial transfer control
		0xFF04 => "DIV",              // Divider register
		0xFF05 => "TIMA",             // Timer counter
		0xFF06 => "TMA",              // Timer modulo
		0xFF07 => "TAC",              // Timer control
		0xFF0F => "IF",               // Interrupt flag
		0xFF10 => "NR10",             // Sound channel 1 sweep
		0xFF11 => "NR11",             // Sound channel 1 length timer & duty cycle
		0xFF12 => "NR12",             // Sound channel 1 volume & envelope
		0xFF13 => "NR13",             // Sound channel 1 period low
		0xFF14 => "NR14",             // Sound channel 1 period high & control
		0xFF16 => "NR21",             // Sound channel 2 length timer & duty cycle
		0xFF17 => "NR22",             // Sound channel 2 volume & envelope
		0xFF18 => "NR23",             // Sound channel 2 period low
		0xFF19 => "NR24",             // Sound channel 2 period high & control
		0xFF1A => "NR30",             // Sound channel 3 DAC enable
		0xFF1B => "NR31",             // Sound channel 3 length timer
		0xFF1C => "NR32",             // Sound channel 3 output level
		0xFF1D => "NR33",             // Sound channel 3 period low
		0xFF1E => "NR34",             // Sound channel 3 period high & control
		0xFF20 => "NR41",             // Sound channel 4 length timer
		0xFF21 => "NR42",             // Sound channel 4 volume & envelope
		0xFF22 => "NR43",             // Sound channel 4 frequency & randomness
		0xFF23 => "NR44",             // Sound channel 4 control
		0xFF24 => "NR50",             // Master volume & VIN panning
		0xFF25 => "NR51",             // Sound panning
		0xFF26 => "NR52",             // Sound on/off
		0xFF30..=0xFF3F => "WaveRAM", // Storage for one of the sound channels` waveform
		0xFF40 => "LCDC",             // LCD control
		0xFF41 => "STAT",             // LCD status
		0xFF42 => "SCY",              // Viewport Y position
		0xFF43 => "SCX",              // Viewport X position
		0xFF44 => "LY",               // LCD Y coordinate
		0xFF45 => "LYC",              // LY compare
		0xFF46 => "DMA",              // OAM DMA source address & start
		0xFF47 => "BGP",              // BG palette data
		0xFF48 => "OBP0",             // OBJ palette 0 data
		0xFF49 => "OBP1",             // OBJ palette 1 data
		0xFF4A => "WY",               // Window Y position
		0xFF4B => "WX",               // Window X position plus 7
		0xFF4D => "KEY1",             // Prepare speed switch
		0xFF4F => "VBK",              // VRAM bank
		0xFF51 => "HDMA1",            // VRAM DMA source high
		0xFF52 => "HDMA2",            // VRAM DMA source low
		0xFF53 => "HDMA3",            // VRAM DMA destination high
		0xFF54 => "HDMA4",            // VRAM DMA destination low
		0xFF55 => "HDMA5",            // VRAM DMA length/mode/start
		0xFF56 => "RP",               // Infrared communications port
		0xFF68 => "BCPS/BGPI",        // Background color palette spec / Background palette index
		0xFF69 => "BCPD/BGPD",        // Background color palette data / Background palette data
		0xFF6A => "OCPS/OBPI",        // OBJ color palette specification / OBJ palette index
		0xFF6B => "OCPD/OBPD",        // OBJ color palette data / OBJ palette data
		0xFF6C => "OPRI",             // Object priority mode
		0xFF70 => "SVBK",             // WRAM bank
		0xFF76 => "PCM12",            // Audio digital outputs 1 & 2
		0xFF77 => "PCM34",            // Audio digital outputs 3 & 4
		0xFFFF => "IE",               // Interrupt enable

		0xFF4C => "KEY0 (CGB mode enable)", // not listed on register summary page
		0xFF50 => "HIDE_BOOT_ROM_BANK",
		0xFF60 => "JOYC (undocumented in pandocs)",

		_ => "UNKNOWN_REGISTER",
	}
}

#[derive(Default)]
pub struct IoReg {
	// normal io registers
	pub p1_joyp: u8,
	pub div: u16,
	pub tima: u8,
	pub tma: u8,
	pub tac: u8,
	pub interrupt: u8,
	pub lcdc: u8,
	pub stat: u8,
	pub scy: u8,
	pub scx: u8,
	pub ly: u8,
	pub lyc: u8,
	pub bgp: u8,
	pub obp0: u8,
	pub obp1: u8,
	pub wy: u8,
	pub wx: u8,
	pub cgb_mode: bool,   //key0
	pub speed_switch: u8, //key1
	pub vbk: bool,
	pub hide_boot_rom: bool,
	pub ie: u8,

	pub audio_params: Arc<Mutex<AudioParams>>,

	// other
	pub joyc: bool, // not documented in pandocs

	// not io registers
	pub debug: bool,
	pub user_input_buttons: u8,
	pub user_input_joypad: u8,
	pub lx: u64,
}
impl IoReg {
	pub fn get(&self, addr: usize) -> u8 {
		let r = match addr {
			0xFF00 => {
				let target = self.p1_joyp & 0b11_0000;
				let buttons = 0xf & !self.user_input_buttons;
				let joypad = 0xf & !self.user_input_joypad;
				target
					| match target {
						0b11_0000 => 0xf,
						0b01_0000 => buttons,
						0b10_0000 => joypad,
						_ => buttons & joypad,
					}
			}
			0xFF04 => (self.div >> 8) as u8,
			0xFF05 => self.tima,
			0xFF0F => self.interrupt,
			0xFF40 => self.lcdc,

			// TODO: report PPU mode
			0xFF41 => (self.stat & 0b_0_11111_00) | (((self.lyc == self.ly) as u8) << 2),

			0xFF42 => self.scy,
			0xFF43 => self.scx,
			0xFF44 => self.ly,
			0xFF45 => self.lyc,
			0xFF4a => self.wy,
			0xFF4b => self.wx,
			0xFF4c => (self.cgb_mode as u8) << 2,
			0xFF4d => self.speed_switch,
			// 0xFF4F => u8::from(self.vbk) | 254,
			0xFF50 => self.hide_boot_rom as u8,
			// 0xFF60 => 0xff,
			0xFFFF => self.ie,
			_ => {
				println!("Read from unknown IO register {addr:#x?}");
				0xff
			}
		};
		if self.debug {
			println!("IO read from {}: {r:02x}", name_of(addr));
		}
		r
	}
	pub fn set(&mut self, addr: usize, data: u8) {
		if self.debug {
			println!("IO write to [{}] = {data:#x?}", name_of(addr));
		}
		match addr {
			0xFF00 => self.p1_joyp = data,
			0xFF04 => self.div = 0,
			0xFF05 => self.tima = data,
			0xFF06 => self.tma = data,
			0xFF07 => self.tac = data,
			0xFF0F => self.interrupt = data,
			0xFF10..=0xFF3F => self.audio_params.lock().unwrap().set(addr, data),
			0xFF40 => {
				if data & 0x80 == 0 {
					self.lx = 0;
					self.ly = 0;
				}
				self.lcdc = data;
			}
			0xFF41 => {
				// we dont support mode 0/1/2 int select
				assert!(data & 0b111000 == 0);
				self.stat = data;
			}
			0xFF42 => self.scy = data,
			0xFF43 => self.scx = data,
			0xFF45 => self.lyc = data,
			0xFF47 => self.bgp = data,
			0xFF48 => self.obp0 = data,
			0xFF49 => self.obp1 = data,
			0xFF4a => self.wy = data,
			0xFF4b => self.wx = data,
			0xFF4c => {
				if !self.hide_boot_rom {
					self.cgb_mode = data & 0b100 != 0;
				}
			}
			0xFF4d => todo!("Prepare speed switch"),
			// 0xFF4F => self.vbk = data & 1 != 0,
			0xFF50 => {
				if data & 1 == 1 {
					self.hide_boot_rom = true;
				}
			}
			0xFF60 => {
				let value = data & 1 != 0;
				// 1-bit register. 0 is the default (normal) value,
				// so if 0 is written we dont have to do anything.
				assert!(!value, "JOYC was enabled. Not implemented.");
			}
			0xFF7F => {} // Unmapped
			0xFFFF => self.ie = data,
			_ => println!("Write to unknown IO register [{addr:#x?}] = {data:#x?}"),
		};
	}
	pub fn advance_counter_div(&mut self, mcycles: u64) -> bool {
		// TODO: sys counter resets and stops incrementing in stop mode,
		// but TIMA needs to keep going.

		let mut do_interrupt = false;
		for _ in 0..mcycles {
			// add 4 T-cycles, not dots. Doubles in double-speed mode.
			self.div = self.div.overflowing_add(4).0;
			if self.tac & 0b100 != 0 {
				let tima_check = match self.tac & 0b11 {
					0 => 1023,
					1 => 15,
					2 => 63,
					_ => 255,
				};
				if self.div & tima_check == 0 {
					if self.tima == 0xff {
						self.tima = self.tma;
						self.interrupt |= INT_TIMER;
						do_interrupt = true;
					} else {
						self.tima += 1;
					}
				}
			}
		}
		do_interrupt
	}
}
