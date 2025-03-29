use crate::GB;

use sdl2::{
	event::Event,
	keyboard::Keycode,
	pixels::{Color, PixelFormatEnum},
	rect::Rect,
	render::Canvas,
	video::Window,
	EventPump,
};

const BTN_PIN_A: u8 = 1 << 0;
const BTN_PIN_B: u8 = 1 << 1;
const BTN_PIN_SELECT: u8 = 1 << 2;
const BTN_PIN_START: u8 = 1 << 3;
const BTN_PIN_RIGHT: u8 = 1 << 0;
const BTN_PIN_LEFT: u8 = 1 << 1;
const BTN_PIN_UP: u8 = 1 << 2;
const BTN_PIN_DOWN: u8 = 1 << 3;

const BTN_KEYCODE_A: Keycode = Keycode::R;
const BTN_KEYCODE_B: Keycode = Keycode::E;
const BTN_KEYCODE_SELECT: Keycode = Keycode::BACKSPACE;
const BTN_KEYCODE_START: Keycode = Keycode::RETURN;
const BTN_KEYCODE_RIGHT: Keycode = Keycode::L;
const BTN_KEYCODE_LEFT: Keycode = Keycode::J;
const BTN_KEYCODE_UP: Keycode = Keycode::I;
const BTN_KEYCODE_DOWN: Keycode = Keycode::K;

const PADDING: i32 = 10;

struct Layout {
	x: i32,
	y: i32,
}
impl Layout {
	fn default() -> Layout {
		Layout {
			x: PADDING,
			y: PADDING,
		}
	}
	fn stack(&mut self, _w: i32, h: i32) -> (i32, i32) {
		let r = (self.x, self.y);
		self.y += h + PADDING;
		r
	}
	fn end(&mut self, w: i32, _h: i32) -> (i32, i32) {
		let r = (self.x, self.y);
		self.y = PADDING;
		self.x += w + PADDING;
		r
	}
}

fn color_dmg(n: u8, palette: u8) -> (u8, u8, u8) {
	let pcolor = (palette >> (n * 2)) & 0b11;
	let c = 0x55 * (3 - pcolor);
	(c, c, c)
}

pub struct UI {
	canvas: Canvas<Window>,
	event_pump: EventPump,
	frame_number: u64,
}
impl UI {
	pub fn default() -> UI {
		let sdl_context = sdl2::init().unwrap();
		let video_subsystem = sdl_context.video().unwrap();

		let window = video_subsystem
			.window("gb", 1920, 1080)
			.position_centered()
			.build()
			.unwrap();

		let canvas = sdl2::render::CanvasBuilder::new(window.clone())
			// .accelerated()
			// .present_vsync()
			.build()
			.unwrap();

		UI {
			canvas,
			frame_number: 0,
			event_pump: sdl_context.event_pump().unwrap(),
		}
	}
	pub fn draw(&mut self, gb: &mut GB, play: &mut bool) -> bool {
		let mut signal_debug = false;

		for event in self.event_pump.poll_iter() {
			match event {
				Event::Quit { .. } => {
					*play = false;
				}
				Event::KeyDown {
					keycode: Some(Keycode::D),
					..
				} => {
					signal_debug = true;
				}
				Event::KeyDown {
					keycode: Some(keycode),
					..
				} => match keycode {
					BTN_KEYCODE_START => gb.bus.io.user_input_buttons |= BTN_PIN_START,
					BTN_KEYCODE_SELECT => gb.bus.io.user_input_buttons |= BTN_PIN_SELECT,
					BTN_KEYCODE_A => gb.bus.io.user_input_buttons |= BTN_PIN_A,
					BTN_KEYCODE_B => gb.bus.io.user_input_buttons |= BTN_PIN_B,
					BTN_KEYCODE_RIGHT => gb.bus.io.user_input_joypad |= BTN_PIN_RIGHT,
					BTN_KEYCODE_DOWN => gb.bus.io.user_input_joypad |= BTN_PIN_DOWN,
					BTN_KEYCODE_LEFT => gb.bus.io.user_input_joypad |= BTN_PIN_LEFT,
					BTN_KEYCODE_UP => gb.bus.io.user_input_joypad |= BTN_PIN_UP,
					_ => {}
				},
				Event::KeyUp {
					keycode: Some(keycode),
					..
				} => match keycode {
					BTN_KEYCODE_START => gb.bus.io.user_input_buttons &= !BTN_PIN_START,
					BTN_KEYCODE_SELECT => gb.bus.io.user_input_buttons &= !BTN_PIN_SELECT,
					BTN_KEYCODE_A => gb.bus.io.user_input_buttons &= !BTN_PIN_A,
					BTN_KEYCODE_B => gb.bus.io.user_input_buttons &= !BTN_PIN_B,
					BTN_KEYCODE_RIGHT => gb.bus.io.user_input_joypad &= !BTN_PIN_RIGHT,
					BTN_KEYCODE_DOWN => gb.bus.io.user_input_joypad &= !BTN_PIN_DOWN,
					BTN_KEYCODE_LEFT => gb.bus.io.user_input_joypad &= !BTN_PIN_LEFT,
					BTN_KEYCODE_UP => gb.bus.io.user_input_joypad &= !BTN_PIN_UP,
					_ => {}
				},
				_ => {}
			}
		}

		self.canvas.set_draw_color(Color::RGB(0x80, 0x80, 0x80));
		self.canvas.clear();

		let mut l = Layout::default();

		{
			let mut img = mem_dump(&gb.bus);

			let w = img.1;
			let h = img.2;

			let (x, y) = l.end(w as i32 * 2, h as i32 * 2);

			let surf = sdl2::surface::Surface::from_data(
				img.0.as_mut(),
				w,
				h,
				w * 3,
				PixelFormatEnum::RGB24,
			)
			.unwrap();

			self.canvas
				.copy(
					&surf.as_texture(&self.canvas.texture_creator()).unwrap(),
					None,
					Rect::new(x, y, w * 2, h * 2),
				)
				.unwrap();
		}

		let bg = {
			let mut img = bg_map(&gb.bus);
			let r = img.0.clone();

			let w = img.1;
			let h = img.2;

			let (x, y) = l.stack(w as i32 * 2, h as i32 * 2);

			let surf = sdl2::surface::Surface::from_data(
				img.0.as_mut(),
				w,
				h,
				w * 3,
				PixelFormatEnum::RGB24,
			)
			.unwrap();

			self.canvas
				.copy(
					&surf.as_texture(&self.canvas.texture_creator()).unwrap(),
					None,
					Rect::new(x, y, w * 2, h * 2),
				)
				.unwrap();

			r
		};

		{
			let mut img = window_map(&gb.bus);

			let w = img.1;
			let h = img.2;

			let (x, y) = l.end(w as i32 * 2, h as i32 * 2);

			let surf = sdl2::surface::Surface::from_data(
				img.0.as_mut(),
				w,
				h,
				w * 3,
				PixelFormatEnum::RGB24,
			)
			.unwrap();

			self.canvas
				.copy(
					&surf.as_texture(&self.canvas.texture_creator()).unwrap(),
					None,
					Rect::new(x, y, w * 2, h * 2),
				)
				.unwrap();
		}

		{
			let mut img = vram_dump(&gb.bus);

			let w = img.1;
			let h = img.2;

			let (x, y) = l.end(w as i32 * 3, h as i32 * 3);

			let surf = sdl2::surface::Surface::from_data(
				img.0.as_mut(),
				w,
				h,
				w * 3,
				PixelFormatEnum::RGB24,
			)
			.unwrap();

			self.canvas
				.copy(
					&surf.as_texture(&self.canvas.texture_creator()).unwrap(),
					None,
					Rect::new(x, y, w * 3, h * 3),
				)
				.unwrap();
		}

		{
			let mut img = tile_dump(&gb.bus);

			let w = img.1;
			let h = img.2;

			let (x, y) = l.end(w as i32 * 3, h as i32 * 3);

			let surf = sdl2::surface::Surface::from_data(
				img.0.as_mut(),
				w,
				h,
				w * 3,
				PixelFormatEnum::RGB24,
			)
			.unwrap();

			self.canvas
				.copy(
					&surf.as_texture(&self.canvas.texture_creator()).unwrap(),
					None,
					Rect::new(x, y, w * 3, h * 3),
				)
				.unwrap();
		}

		{
			let w = 160;
			let h = 144;

			let (x, y) = l.stack(w as i32 * 3, h as i32 * 3);

			let mut framebuffer = gb.framebuffer.clone();
			let surf = sdl2::surface::Surface::from_data(
				framebuffer.as_mut(),
				w,
				h,
				w * 3,
				PixelFormatEnum::RGB24,
			)
			.unwrap();

			self.canvas
				.copy(
					&surf.as_texture(&self.canvas.texture_creator()).unwrap(),
					None,
					Rect::new(x, y, w * 3, h * 3),
				)
				.unwrap();
		}
		{
			let mut img = render_screen_old(bg.as_ref(), &gb.bus);
			render_sprites(img.0.as_mut(), &gb.bus);

			let w = img.1;
			let h = img.2;

			let (x, y) = l.end(w as i32 * 3, h as i32 * 3);

			let surf = sdl2::surface::Surface::from_data(
				img.0.as_mut(),
				w,
				h,
				w * 3,
				PixelFormatEnum::RGB24,
			)
			.unwrap();

			self.canvas
				.copy(
					&surf.as_texture(&self.canvas.texture_creator()).unwrap(),
					None,
					Rect::new(x, y, w * 3, h * 3),
				)
				.unwrap();
		}

		self.canvas.present();

		self.frame_number += 1;

		signal_debug
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

fn window_map(mem: &crate::bus::Bus) -> (Box<[u8]>, u32, u32) {
	let mut img = Box::new([0; 256 * 256 * 3]);

	for x in 0..32 {
		for y in 0..32 {
			// TODO: LCDC controls tile area 0x1800/0x1C00
			let mut itile = mem.vram[0][0x1C00 + (x + y * 32)] as usize;
			if mem.io.lcdc & 0b10000 == 0 && itile & 0x80 == 0 {
				itile |= 0x100;
			}
			draw_tile(
				itile,
				img.as_mut(),
				(x * 8) + (y * 8 * 256),
				256,
				&mem.vram[0],
				mem.io.bgp,
				false,
			);
		}
	}

	(img, 256, 256)
}

fn bg_map(mem: &crate::bus::Bus) -> (Box<[u8]>, u32, u32) {
	let mut img = Box::new([0; 256 * 256 * 3]);

	for x in 0..32 {
		for y in 0..32 {
			// TODO: LCDC controls tile area 0x1800/0x1C00
			let mut itile = mem.vram[0][0x1800 + (x + y * 32)] as usize;
			if mem.io.lcdc & 0b10000 == 0 && itile & 0x80 == 0 {
				itile |= 0x100;
			}
			draw_tile(
				itile,
				img.as_mut(),
				(x * 8) + (y * 8 * 256),
				256,
				&mem.vram[0],
				mem.io.bgp,
				false,
			);
		}
	}

	(img, 256, 256)
}

fn tile_dump(mem: &crate::bus::Bus) -> (Box<[u8]>, u32, u32) {
	const TILE_SIZE: usize = 8;
	const OUTPUT_WIDTH_IN_TILES: usize = 16;
	const OUTPUT_HEIGHT_IN_TILES: usize = 384 / OUTPUT_WIDTH_IN_TILES;
	const OUTPUT_WIDTH: usize = OUTPUT_WIDTH_IN_TILES * TILE_SIZE;
	const OUTPUT_HEIGHT: usize = OUTPUT_HEIGHT_IN_TILES * TILE_SIZE;

	let mut img = Box::new([0; OUTPUT_WIDTH * OUTPUT_HEIGHT * 3]);

	for itile in 0..384 {
		draw_tile(
			itile,
			img.as_mut(),
			(itile % OUTPUT_WIDTH_IN_TILES * 8) + (itile / OUTPUT_WIDTH_IN_TILES * 8 * 8 * 16),
			OUTPUT_WIDTH,
			&mem.vram[0],
			0b_11_10_01_00,
			false,
		);
	}

	(img, OUTPUT_WIDTH as u32, OUTPUT_HEIGHT as u32)
}

fn vram_dump(mem: &crate::bus::Bus) -> (Box<[u8]>, u32, u32) {
	const W: usize = 32;
	const H: usize = 0x2000 / W;

	let mut img = Box::new([0; W * H * 3]);

	for i in 0..(W * H) {
		let c = mem.vram[0][i];
		img[3 * i + 0] = c;
		img[3 * i + 1] = c;
		img[3 * i + 2] = c;
	}

	(img, W as u32, H as u32)
}

fn render_screen_old(bg: &[u8], mem: &crate::bus::Bus) -> (Box<[u8]>, u32, u32) {
	let mut img = Box::new([0; 160 * 144 * 3]);

	for x in 0..160 {
		for y in 0..144 {
			let fb_ofs = (x + mem.io.scx as usize) + (y + mem.io.scy as usize) * 256;
			img[3 * (x + 160 * y) + 0] = bg[3 * (fb_ofs & 0xffff) + 0];
			img[3 * (x + 160 * y) + 1] = bg[3 * (fb_ofs & 0xffff) + 1];
			img[3 * (x + 160 * y) + 2] = bg[3 * (fb_ofs & 0xffff) + 2];
		}
	}

	(img, 160, 144)
}

fn render_sprites(img: &mut [u8], mem: &crate::bus::Bus) {
	// let larger_sprites = mem.io.lcdc & 0b100 != 0;
	for oam_ofs in (0..(40 * 4)).step_by(4) {
		let y = mem.oam[oam_ofs + 0];
		let x = mem.oam[oam_ofs + 1];
		let itile = mem.oam[oam_ofs + 2] as usize;
		let flags = mem.oam[oam_ofs + 3];
		// let cgb_palette = flags & 0b111;
		let bank = (flags >> 3 & 1) as usize;
		let dmg_palette = flags >> 4 & 1 != 0;
		// let x_flip = flags >> 5 & 1 != 0;
		// let y_flip = flags >> 6 & 1 != 0;
		// let prio = flags >> 7 != 0;

		// TODO: handle partially off-screen sprites

		// the +8 here is only okay because we cant render 16-tall tiles
		if y >= 16 && x >= 8 && y <= 143 + 8 {
			let y = (y as usize) - 16;
			let x = (x as usize) - 8;
			let ofs = y * 160 + x;
			draw_tile(
				itile,
				img.as_mut(),
				ofs,
				160,
				&mem.vram[bank],
				match dmg_palette {
					false => mem.io.obp0,
					true => mem.io.obp1,
				},
				true,
			);
		}
	}
}

fn mem_dump(mem: &crate::bus::Bus) -> (Box<[u8]>, u32, u32) {
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
	(img, 128, 0x10000 / 128)
}
