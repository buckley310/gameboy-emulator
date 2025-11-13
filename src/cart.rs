use std::error::Error;

pub const ROM_BANK_SIZE: usize = 0x4000;
pub const EXRAM_BANK_SIZE: usize = 0x2000;

#[derive(Debug)]
pub enum MBCType {
	MBC0,
	MBC1,
}
impl MBCType {
	pub fn from_header(n: u8) -> MBCType {
		match n {
			0 => MBCType::MBC0,
			1 => MBCType::MBC1,
			3 => MBCType::MBC1,
			x => panic!("Unknown MBC: {x:#x}"),
		}
	}
}

pub struct Cartridge {
	pub rom: Vec<[u8; ROM_BANK_SIZE]>,
	pub rom_bank: usize,

	pub exram: Vec<[u8; EXRAM_BANK_SIZE]>,
	pub exram_bank: usize,
	pub exram_enable: bool,

	pub bank_mode: bool,

	pub mbc: MBCType,

	pub debug_bank_switch: bool,
}
impl std::default::Default for Cartridge {
	fn default() -> Self {
		Self {
			rom: vec![],
			rom_bank: 0, // 0 treated as 1 during bank access

			exram: vec![],
			exram_bank: 0,
			exram_enable: false,

			bank_mode: false,

			mbc: MBCType::MBC0,

			debug_bank_switch: false,
		}
	}
}
impl Cartridge {
	pub fn load_rom(&mut self, mut rom: &[u8]) -> Result<(), Box<dyn Error>> {
		self.rom = vec![];

		self.mbc = MBCType::from_header(rom[0x147]);

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
			MBCType::MBC0 => {
				assert!(cart_rom_banks == 2);
				assert!(cart_ram_banks == 0);
				assert!(rom.len() == ROM_BANK_SIZE * cart_rom_banks);
				while rom.len() > 0 {
					self.rom.push((&rom[..ROM_BANK_SIZE]).try_into()?);
					rom = &rom[ROM_BANK_SIZE..];
				}
			}
			MBCType::MBC1 => {
				assert!(cart_ram_banks <= 4);
				assert!(rom.len() == ROM_BANK_SIZE * cart_rom_banks);
				while rom.len() > 0 {
					self.rom.push((&rom[..ROM_BANK_SIZE]).try_into()?);
					rom = &rom[ROM_BANK_SIZE..];
				}
				for _ in 0..cart_ram_banks {
					self.exram.push([0; EXRAM_BANK_SIZE]);
				}
			}
		}
		Ok(())
	}
	pub fn peek(&self, addr16: u16) -> u8 {
		let addr = addr16 as usize;
		match addr16 {
			// ROM
			0x0000..=0x7FFF => {
				match self.mbc {
					MBCType::MBC0 => match addr {
						0x0000..=0x3FFF => self.rom[0][addr],
						_ => self.rom[1][addr - 0x4000],
					},
					MBCType::MBC1 => {
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
			// external ram bank N
			0xA000..=0xBFFF => match self.mbc {
				MBCType::MBC0 => 0xFF,
				MBCType::MBC1 => {
					if self.exram_bank >= self.exram.len() {
						0xFF
					} else if !self.exram_enable {
						0xFF
					} else {
						self.exram[self.exram_bank][addr - 0xA000]
					}
				}
			},
			_ => panic!("Cart out-of-bounds read"),
		}
	}
	pub fn poke(&mut self, addr16: u16, data: u8) {
		let addr = addr16 as usize;
		match addr16 {
			// ROM
			0x0000..=0x7FFF => {
				match self.mbc {
					MBCType::MBC0 => println!("attempted to write to ROM with MBC 0"),
					MBCType::MBC1 => match addr {
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
			// external ram bank N
			0xA000..=0xBFFF => match self.mbc {
				MBCType::MBC0 => {}
				MBCType::MBC1 => {
					if self.exram_enable {
						self.exram[self.exram_bank][addr - 0xA000] = data;
					}
				}
			},
			_ => panic!("Cart out-of-bounds write"),
		}
	}
}
