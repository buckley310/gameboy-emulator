use crate::{GB, bus};
use raylib::prelude::*;

const CONTROLS: &[(bool, u8, KeyboardKey)] = &[
	// (is_joypad, io_pin, keycode)
	(false, 1, KeyboardKey::KEY_R),         // A
	(false, 2, KeyboardKey::KEY_E),         // B
	(false, 4, KeyboardKey::KEY_BACKSPACE), // SELECT
	(false, 8, KeyboardKey::KEY_ENTER),     // START
	(true, 1, KeyboardKey::KEY_L),          // RIGHT
	(true, 2, KeyboardKey::KEY_J),          // LEFT
	(true, 4, KeyboardKey::KEY_I),          // UP
	(true, 8, KeyboardKey::KEY_K),          // DOWN
];

const VRAM_WIDTH: i32 = 32;
const VRAM_HEIGHT: i32 = bus::VRAM_SIZE as i32 / VRAM_WIDTH;

// 384 tiles. display 16 wide. each tile is 8x8
const TILE_VIEWER_WIDTH: i32 = 8 * 16;
const TILE_VIEWER_HEIGHT: i32 = 8 * 384 / 16;

const PADDING: i32 = 10;

struct Layout {
	x: i32,
	y: i32,
	col_w: i32,
}
impl Layout {
	fn default() -> Layout {
		Layout {
			x: PADDING,
			y: PADDING,
			col_w: 0,
		}
	}
	fn stack(&mut self, w: i32, h: i32, scale: i32) -> Vector2 {
		let (x, y) = (self.x, self.y);
		self.col_w = self.col_w.max(w * scale);
		self.y += h * scale + PADDING;
		Vector2 {
			x: x as f32,
			y: y as f32,
		}
	}
	fn next(&mut self, w: i32, h: i32, scale: i32) -> Vector2 {
		self.y = h * scale + 2 * PADDING;
		self.x += self.col_w + PADDING;
		self.col_w = w * scale;
		Vector2 {
			x: self.x as f32,
			y: PADDING as f32,
		}
	}
}

fn color_dmg(n: u8, palette: u8) -> (u8, u8, u8) {
	let pcolor = (palette >> (n * 2)) & 0b11;
	let c = 0x55 * (3 - pcolor);
	(c, c, c)
}

fn blank_tex(rl: &mut (RaylibHandle, RaylibThread), w: i32, h: i32) -> Texture2D {
	let mut img = Image::gen_image_color(w, h, Color::BLACK);
	img.set_format(PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8);
	rl.0.load_texture_from_image(&rl.1, &img).unwrap()
}

struct GbTextures {
	fb: Texture2D,
	mem: Texture2D,
	bg: Texture2D,
	win: Texture2D,
	tile: Texture2D,
	vram: Texture2D,
}

pub struct UI {
	rl: (RaylibHandle, RaylibThread),
	tex: GbTextures,
	frame_number: u64,
	verbose: bool,
}
impl UI {
	pub fn new(verbose: bool) -> UI {
		let (w, h) = match verbose {
			true => (1920, 1080),
			false => (160 * 3 + PADDING * 2, 144 * 3 + (PADDING * 2)),
		};
		let mut rl = raylib::init().size(w, h).build();
		let tex = GbTextures {
			fb: blank_tex(&mut rl, 160, 144),
			mem: blank_tex(&mut rl, 256, 256),
			bg: blank_tex(&mut rl, 256, 256),
			win: blank_tex(&mut rl, 256, 256),
			tile: blank_tex(&mut rl, TILE_VIEWER_WIDTH, TILE_VIEWER_HEIGHT),
			vram: blank_tex(&mut rl, VRAM_WIDTH, VRAM_HEIGHT),
		};
		UI {
			rl,
			tex,
			frame_number: 0,
			verbose,
		}
	}
	pub fn draw(&mut self, gb: &mut GB, play: &mut bool) {
		if self.rl.0.window_should_close() {
			*play = false
		}

		for (is_joypad, io_pin, keycode) in CONTROLS {
			let target = if *is_joypad {
				&mut gb.bus.io.user_input_joypad
			} else {
				&mut gb.bus.io.user_input_buttons
			};
			if self.rl.0.is_key_down(*keycode) {
				*target |= io_pin
			} else {
				*target &= !io_pin
			}
		}

		self.tex.fb.update_texture(&gb.framebuffer).unwrap();
		if self.verbose {
			self.tex.mem.update_texture(&mem_dump(&gb.bus)).unwrap();
			self.tex.bg.update_texture(&bg_map(&gb.bus)).unwrap();
			self.tex.win.update_texture(&window_map(&gb.bus)).unwrap();
			self.tex.tile.update_texture(&tile_dump(&gb.bus)).unwrap();
			self.tex.vram.update_texture(&vram_dump(&gb.bus)).unwrap();
		}

		let mut l = Layout::default();
		let mut d = self.rl.0.begin_drawing(&self.rl.1);
		d.clear_background(Color::GRAY);
		if self.verbose {
			d.draw_texture_ex(
				&self.tex.bg,
				l.stack(self.tex.bg.width, self.tex.bg.height, 2),
				0.0,
				2.0,
				Color::WHITE,
			);
			d.draw_texture_ex(
				&self.tex.win,
				l.stack(self.tex.win.width, self.tex.win.height, 2),
				0.0,
				2.0,
				Color::WHITE,
			);
			d.draw_texture_ex(
				&self.tex.mem,
				l.next(self.tex.mem.width, self.tex.mem.height, 2),
				0.0,
				2.0,
				Color::WHITE,
			);
		}
		d.draw_texture_ex(
			&self.tex.fb,
			l.stack(self.tex.fb.width, self.tex.fb.height, 3),
			0.0,
			3.0,
			Color::WHITE,
		);
		if self.verbose {
			d.draw_texture_ex(
				&self.tex.tile,
				l.next(self.tex.tile.width, self.tex.tile.height, 3),
				0.0,
				3.0,
				Color::WHITE,
			);
			d.draw_texture_ex(
				&self.tex.vram,
				l.next(self.tex.vram.width, self.tex.vram.height, 3),
				0.0,
				3.0,
				Color::WHITE,
			);
		}
		self.frame_number += 1;
	}
}

fn draw_tile(
	itile: usize,
	img: &mut [u8],
	ofs: usize,
	output_width: usize,
	bank: &[u8],
	palette: u8,
	transparent: bool,
) {
	let data = &bank[itile * 16..itile * 16 + 16];
	for x in 0..8 {
		for y in 0..8 {
			let b1 = (data[y * 2 + 0] >> (7 - x)) & 1;
			let b2 = (data[y * 2 + 1] >> (7 - x)) & 1;
			if b1 | b2 != 0 || !transparent {
				let (r, g, b) = color_dmg(b1 | (b2 << 1), palette);
				img[0 + 3 * (ofs + x + output_width * y)] = r;
				img[1 + 3 * (ofs + x + output_width * y)] = g;
				img[2 + 3 * (ofs + x + output_width * y)] = b;
			}
		}
	}
}

fn window_map(mem: &crate::bus::Bus) -> Box<[u8]> {
	let mut img = Box::new([0; 256 * 256 * 3]);

	for x in 0..32 {
		for y in 0..32 {
			// TODO: LCDC controls tile area 0x1800/0x1C00
			let mut itile = mem.vram[0x1C00 + (x + y * 32)] as usize;
			if mem.io.lcdc & 0b10000 == 0 && itile & 0x80 == 0 {
				itile |= 0x100;
			}
			draw_tile(
				itile,
				img.as_mut(),
				(x * 8) + (y * 8 * 256),
				256,
				&mem.vram,
				mem.io.bgp,
				false,
			);
		}
	}

	img
}

fn bg_map(mem: &crate::bus::Bus) -> Box<[u8]> {
	let mut img = Box::new([0; 256 * 256 * 3]);

	for x in 0..32 {
		for y in 0..32 {
			// TODO: LCDC controls tile area 0x1800/0x1C00
			let mut itile = mem.vram[0x1800 + (x + y * 32)] as usize;
			if mem.io.lcdc & 0b10000 == 0 && itile & 0x80 == 0 {
				itile |= 0x100;
			}
			draw_tile(
				itile,
				img.as_mut(),
				(x * 8) + (y * 8 * 256),
				256,
				&mem.vram,
				mem.io.bgp,
				false,
			);
		}
	}

	img
}

fn tile_dump(mem: &crate::bus::Bus) -> Box<[u8]> {
	const OUTPUT_WIDTH_IN_TILES: i32 = TILE_VIEWER_WIDTH / 8;

	let mut img = Box::new([0; (TILE_VIEWER_WIDTH * TILE_VIEWER_HEIGHT * 3) as usize]);

	for itile in 0..384 {
		draw_tile(
			itile,
			img.as_mut(),
			(itile % OUTPUT_WIDTH_IN_TILES as usize * 8)
				+ (itile / OUTPUT_WIDTH_IN_TILES as usize * 8 * 8 * 16),
			TILE_VIEWER_WIDTH as usize,
			&mem.vram,
			0b_11_10_01_00,
			false,
		);
	}

	img
}

fn vram_dump(mem: &crate::bus::Bus) -> Box<[u8]> {
	let mut img = Box::new([0; bus::VRAM_SIZE * 3]);

	for i in 0..bus::VRAM_SIZE {
		let c = mem.vram[i];
		img[3 * i + 0] = c;
		img[3 * i + 1] = c;
		img[3 * i + 2] = c;
	}

	img
}

fn mem_dump(mem: &crate::bus::Bus) -> Box<[u8]> {
	let mut img = Box::new([0; 0x10000 * 3]);
	for i in 0x0000..=0xFFFF {
		let byte = match i {
			0xE000..=0xFDFF => 1, // Echo Ram
			0xFF00..=0xFF7F => 1, // IO Regs
			0xFFFF => 1,          // IE Reg (skip to avoid verbose io log)
			_ => mem.peek(i),
		}
		.reverse_bits();
		img[3 * (i as usize) + 0] = byte;
		img[3 * (i as usize) + 1] = byte;
		img[3 * (i as usize) + 2] = byte;
	}
	img
}
