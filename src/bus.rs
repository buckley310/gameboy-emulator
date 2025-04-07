use crate::ioreg::IoReg;

const ROM_BANK_SIZE: usize = 0x4000;
const VRAM_BANK_SIZE: usize = 0x2000;
const WRAM_BANK_SIZE: usize = 0x1000;
const EXRAM_BANK_SIZE: usize = 0x2000;

const BOOT_ROM: [u8; 0x100] = [
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0, 0, 0,
	////////////////////////////////////////////////////////////////

	// enable DMG compat mode if DMG rom is present
	0xfa, 0x43, 0x01, // ld a, ($0143)
	0xe6, 0x80, // and a, $80
	0x28, 0x05, // jr Z, $05
	0xfa, 0x43, 0x01, // ld a, ($0143)
	0xe0, 0x4c, // ldh ($4c), a
	//

	// set up stack
	0x31, 0xfe, 0xff, // LD SP, $FFFE
	//

	// turn on LCD
	0x3e, 0x91, // ld a, $91
	0xe0, 0x40, // ldh ($ff40), a
	//

	// unmap boot rom
	0x3e, 0x01, // ld a, $01
	0xe0, 0x50, // ldh ($ff50), a
];

#[derive(Debug)]
pub enum MBC {
	MBC0,
	MBC1,
}
impl MBC {
	fn from(n: u8) -> MBC {
		match n {
			0 => MBC::MBC0,
			1 => MBC::MBC1,
			3 => MBC::MBC1,
			x => panic!("Unknown MBC: {x:#x}"),
		}
	}
}

pub struct Bus {
	pub rom: Vec<[u8; ROM_BANK_SIZE]>,
	pub rom_bank: usize,

	pub vram: [[u8; VRAM_BANK_SIZE]; 2],

	pub exram: Vec<[u8; EXRAM_BANK_SIZE]>,
	pub exram_bank: usize,
	pub exram_enable: bool,

	pub bank_mode: bool,

	pub wram: [[u8; WRAM_BANK_SIZE]; 8],
	pub wram_bank: usize,

	pub oam: [u8; 0xA0],
	pub io: IoReg,
	pub hram: [u8; 0x7F],

	pub mbc: MBC,

	pub debug_bank_switch: bool,
}
impl std::default::Default for Bus {
	fn default() -> Bus {
		Bus {
			rom: vec![],
			rom_bank: 0, // 0 treated as 1 during bank access
			vram: [[0; VRAM_BANK_SIZE]; 2],
			exram: vec![],
			exram_bank: 0,
			exram_enable: false,
			bank_mode: false,
			wram: [[0; WRAM_BANK_SIZE]; 8],
			wram_bank: 0,
			hram: [0; 0x7f],
			oam: [0; 0xA0],
			io: IoReg::default(),
			mbc: MBC::MBC0,
			debug_bank_switch: false,
		}
	}
}
impl Bus {
	pub fn load_rom(&mut self, mut rom: &[u8]) {
		self.rom = vec![];

		self.mbc = MBC::from(rom[0x147]);

		assert!(rom[0x148] < 9);
		if rom[0x148] > 4 {
			println!(">=1MiB cart! ram/rom addressing might be weird.");
		}
		let cart_rom_banks = 2 << rom[0x148];

		let cart_ram_banks = match rom[0x149] {
			0 => 0,
			2 => 1,
			3 => 4,
			4 => 16,
			5 => 8,
			_ => panic!(),
		};

		println!(
			"{:?}, ROM BANKS:{}, RAM BANKS:{}, cart size:{}",
			self.mbc,
			cart_rom_banks,
			cart_ram_banks,
			rom.len(),
		);

		match self.mbc {
			MBC::MBC0 => {
				assert!(cart_rom_banks == 2);
				assert!(cart_ram_banks == 0);
				assert!(rom.len() == ROM_BANK_SIZE * cart_rom_banks);
				while rom.len() > 0 {
					self.rom.push((&rom[..ROM_BANK_SIZE]).try_into().unwrap());
					rom = &rom[ROM_BANK_SIZE..];
				}
			}
			MBC::MBC1 => {
				assert!(cart_ram_banks <= 4);
				assert!(rom.len() == ROM_BANK_SIZE * cart_rom_banks);
				while rom.len() > 0 {
					self.rom.push((&rom[..ROM_BANK_SIZE]).try_into().unwrap());
					rom = &rom[ROM_BANK_SIZE..];
				}
				for _ in 0..cart_ram_banks {
					self.exram.push([0; EXRAM_BANK_SIZE]);
				}
			}
		}
	}
	pub fn peek(&self, addr16: u16) -> u8 {
		let addr = addr16 as usize;
		match addr16 {
			// ROM
			0x0000..=0x7FFF => {
				if addr < 0x100 && !self.io.hide_boot_rom {
					return BOOT_ROM[addr];
				}
				match self.mbc {
					MBC::MBC0 => match addr {
						0x0000..=0x3FFF => self.rom[0][addr],
						_ => self.rom[1][addr - 0x4000],
					},
					MBC::MBC1 => {
						// TODO:  we currently do not support MBC1 multi-carts
						let bank_advanced_ofs = if self.bank_mode {
							self.exram_bank << 5
						} else {
							0
						};
						match addr {
							0x0000..=0x3FFF => self.rom[bank_advanced_ofs][addr],
							_ => self.rom[bank_advanced_ofs + self.rom_bank.max(1)][addr - 0x4000],
						}
					}
				}
			}
			// vram bank 0/1
			0x8000..=0x9FFF => self.vram[usize::from(self.io.vbk)][addr - 0x8000], // TODO: banks
			// external ram bank N
			0xA000..=0xBFFF => match self.mbc {
				MBC::MBC0 => 0xFF,
				MBC::MBC1 => {
					if self.exram_bank >= self.exram.len() {
						0xFF
					} else if !self.exram_enable {
						0xFF
					} else {
						self.exram[self.exram_bank][addr - 0xA000]
					}
				}
			},
			// WRAM bank 0
			0xC000..=0xCFFF => self.wram[0][addr - 0xC000],
			// WRAM bank 1-7
			0xD000..=0xDFFF => self.wram[self.wram_bank.max(1)][addr - 0xD000],
			// Echo RAM
			0xE000..=0xFDFF => self.peek(addr16 - 0x2000),
			// OAM
			0xFE00..=0xFE9F => self.oam[addr - 0xFE00],
			// Not Usable
			0xFEA0..=0xFEFF => 0,
			// IO registers
			0xFF00..=0xFF7F => self.io.get(addr),
			// HRAM
			0xFF80..=0xFFFE => self.hram[addr - 0xFF80],
			// IE
			0xFFFF => self.io.get(addr),
		}
	}
	pub fn poke(&mut self, addr16: u16, data: u8) {
		let addr = addr16 as usize;
		match addr16 {
			// ROM
			0x0000..=0x7FFF => {
				assert!(addr > 0xff || self.io.hide_boot_rom);
				match self.mbc {
					MBC::MBC0 => println!("attempted to write to ROM with MBC 0"),
					MBC::MBC1 => match addr {
						0x0000..=0x1FFF => self.exram_enable = data & 0xF == 0xA,
						0x2000..=0x3FFF => {
							if self.debug_bank_switch {
								println!("ROM BANK {} {:02x}", "  ".repeat(data as usize), data);
							}
							self.rom_bank = data as usize & 0x1F
						}
						// TODO: could be upper bits of rom bank number on some carts:
						0x4000..=0x5FFF => {
							self.exram_bank = data as usize & 3;
							if self.debug_bank_switch {
								println!("EXRAM BANK {:02x}", data);
							}
						}
						_ => {
							self.bank_mode = data & 1 != 0;
							if self.debug_bank_switch {
								println!("BANK MODE: advanced={}", self.bank_mode);
							}
						}
					},
				}
			}

			// vram bank 0/1
			0x8000..=0x9FFF => self.vram[usize::from(self.io.vbk)][addr - 0x8000] = data,

			// external ram bank N
			0xA000..=0xBFFF => match self.mbc {
				MBC::MBC0 => {}
				MBC::MBC1 => {
					if self.exram_enable {
						self.exram[self.exram_bank][addr - 0xA000] = data;
					}
				}
			},

			// WRAM bank 0
			0xC000..=0xCFFF => self.wram[0][addr - 0xC000] = data,

			// WRAM bank 1-7
			0xD000..=0xDFFF => self.wram[self.wram_bank.max(1)][addr - 0xD000] = data,

			// Echo RAM
			0xE000..=0xFDFF => {}

			// OAM
			0xFE00..=0xFE9F => self.oam[addr & 0xff] = data,

			// Not Usable
			0xFEA0..=0xFEFF => {}

			// IO registers
			0xFF46 => {
				let ofs = (data as u16) << 8;
				let x = (ofs..(ofs + 0xA0))
					.map(|i| self.peek(i))
					.collect::<Vec<u8>>();
				self.oam.copy_from_slice(&x);
			}
			0xFF00..=0xFF7F => self.io.set(addr, data),

			// HRAM
			0xFF80..=0xFFFE => self.hram[addr - 0xFF80] = data,

			// IE
			0xFFFF => self.io.set(addr, data),
		}
	}
	pub fn peek16(&self, addr: u16) -> u16 {
		(self.peek(addr) as u16) | ((self.peek(addr + 1) as u16) << 8)
	}
	pub fn poke16(&mut self, addr: u16, data: u16) {
		self.poke(addr, (data & 0xff) as u8);
		self.poke(addr + 1, ((data & 0xff00) >> 8) as u8);
	}
}
