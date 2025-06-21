use crate::cart::Cartridge;
use crate::ioreg::IoReg;

const VRAM_BANK_SIZE: usize = 0x2000;
const WRAM_BANK_SIZE: usize = 0x1000;

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

pub struct Bus {
	pub vram: [[u8; VRAM_BANK_SIZE]; 2],

	pub wram: [[u8; WRAM_BANK_SIZE]; 8],
	pub wram_bank: usize,

	pub oam: [u8; 0xA0],
	pub io: IoReg,
	pub hram: [u8; 0x7F],

	pub cart: Cartridge,
}
impl std::default::Default for Bus {
	fn default() -> Bus {
		Bus {
			vram: [[0; VRAM_BANK_SIZE]; 2],
			wram: [[0; WRAM_BANK_SIZE]; 8],
			wram_bank: 0,
			hram: [0; 0x7f],
			oam: [0; 0xA0],
			io: IoReg::default(),
			cart: Cartridge::default(),
		}
	}
}
impl Bus {
	pub fn peek(&self, addr16: u16) -> u8 {
		let addr = addr16 as usize;
		if addr < 0x100 && !self.io.hide_boot_rom {
			return BOOT_ROM[addr];
		}
		match addr16 {
			// Cartridge (ROM/EXRAM)
			0x0000..=0x7FFF | 0xA000..=0xBFFF => self.cart.peek(addr16),
			// vram bank 0/1
			0x8000..=0x9FFF => self.vram[usize::from(self.io.vbk)][addr - 0x8000], // TODO: banks
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
		assert!(addr16 > 0xff || self.io.hide_boot_rom);
		let addr = addr16 as usize;
		match addr16 {
			// Cartridge (ROM/EXRAM)
			0x0000..=0x7FFF | 0xA000..=0xBFFF => self.cart.poke(addr16, data),
			// vram bank 0/1
			0x8000..=0x9FFF => self.vram[usize::from(self.io.vbk)][addr - 0x8000] = data,
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
