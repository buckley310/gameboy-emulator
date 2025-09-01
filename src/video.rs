use crate::GB;

pub struct Sprite {
	y: usize,
	x: usize,
	itile: usize,
	prio: bool,
	y_flip: bool,
	x_flip: bool,
	dmg_palette: bool,
}
impl Sprite {
	pub fn new(data: (u8, u8, u8, u8)) -> Sprite {
		Sprite {
			y: data.0 as usize,
			x: data.1 as usize,
			itile: data.2 as usize,
			prio: data.3 & 0b_1000_0000 != 0,
			y_flip: data.3 & 0b_0100_0000 != 0,
			x_flip: data.3 & 0b_0010_0000 != 0,
			dmg_palette: data.3 & 0b_0001_0000 != 0,
		}
	}
}

pub fn oam_scan(gb: &GB) -> Vec<Sprite> {
	let sprite_h = match gb.bus.io.lcdc & 0b100 {
		0 => 8,
		_ => 16,
	};
	let mut sprites = vec![];
	for oam_ofs in (0..(40 * 4)).step_by(4) {
		if gb.bus.oam[oam_ofs] <= gb.bus.io.ly + 16
			&& gb.bus.oam[oam_ofs] + sprite_h > gb.bus.io.ly + 16
		{
			sprites.push(Sprite::new((
				gb.bus.oam[oam_ofs + 0],
				gb.bus.oam[oam_ofs + 1],
				gb.bus.oam[oam_ofs + 2],
				gb.bus.oam[oam_ofs + 3],
			)));
			if sprites.len() == 10 {
				break;
			}
		}
	}
	sprites.sort_by_key(|x| x.x);
	sprites
}

pub fn color_dmg(n: u8, palette: u8) -> (u8, u8, u8) {
	let pcolor = (palette >> (n * 2)) & 0b11;
	let c = 0x55 * (3 - pcolor);
	(c, c, c)
}

pub fn render_dot(gb: &mut GB, lx: u64, sprites: &Vec<Sprite>) {
	if lx < 80 {
		return;
	}
	let lcd_x = lx as usize - 80;
	let lcd_y = gb.bus.io.ly as usize;
	if lcd_x >= 160 {
		return;
	}
	if gb.bus.io.ly > 143 {
		return;
	}

	let window_enable = gb.bus.io.lcdc & 0b_0010_0000 != 0;

	let wx = gb.bus.io.wx as usize;
	let wy = gb.bus.io.wy as usize;

	let map_pallete_index = if window_enable && wy <= lcd_y && wx <= lcd_x + 7 {
		let win_y = lcd_y - wy;
		let win_x = lcd_x + 7 - wx;

		let tile_map_area = match gb.bus.io.lcdc & 0b_0100_0000 {
			0 => 0x1800,
			_ => 0x1C00,
		};

		let mut itile = gb.bus.vram[tile_map_area + ((win_x >> 3) + (win_y >> 3) * 32)] as usize;

		if gb.bus.io.lcdc & 0b10000 == 0 && itile & 0x80 == 0 {
			itile |= 0x100;
		}

		let tile_data = &gb.bus.vram[itile * 16..itile * 16 + 16];

		let tile_x = win_x & 0b111;
		let tile_y = win_y & 0b111;
		let b1 = (tile_data[tile_y * 2 + 0] >> (7 - tile_x)) & 1;
		let b2 = (tile_data[tile_y * 2 + 1] >> (7 - tile_x)) & 1;
		let pallete_index = b1 | (b2 << 1);

		let (r, g, b) = color_dmg(pallete_index, gb.bus.io.bgp);

		gb.framebuffer[0 + 3 * (lcd_x + 160 * lcd_y)] = r;
		gb.framebuffer[1 + 3 * (lcd_x + 160 * lcd_y)] = g;
		gb.framebuffer[2 + 3 * (lcd_x + 160 * lcd_y)] = b;

		pallete_index
	} else {
		let bg_x = 0xff & ((gb.bus.io.scx as usize) + lcd_x);
		let bg_y = 0xff & ((gb.bus.io.scy as usize) + lcd_y);

		let tile_map_area = match gb.bus.io.lcdc & 0b1000 {
			0 => 0x1800,
			_ => 0x1C00,
		};
		let mut itile = gb.bus.vram[tile_map_area + ((bg_x >> 3) + (bg_y >> 3) * 32)] as usize;

		if gb.bus.io.lcdc & 0b10000 == 0 && itile & 0x80 == 0 {
			itile |= 0x100;
		}

		let tile_data = &gb.bus.vram[itile * 16..itile * 16 + 16];

		let tile_x = bg_x & 0b111;
		let tile_y = bg_y & 0b111;
		let b1 = (tile_data[tile_y * 2 + 0] >> (7 - tile_x)) & 1;
		let b2 = (tile_data[tile_y * 2 + 1] >> (7 - tile_x)) & 1;
		let pallete_index = b1 | (b2 << 1);

		// TODO: BG transparency

		let (r, g, b) = color_dmg(pallete_index, gb.bus.io.bgp);
		gb.framebuffer[0 + 3 * (lcd_x + 160 * lcd_y)] = r;
		gb.framebuffer[1 + 3 * (lcd_x + 160 * lcd_y)] = g;
		gb.framebuffer[2 + 3 * (lcd_x + 160 * lcd_y)] = b;

		pallete_index
	};
	let sprite_h = match gb.bus.io.lcdc & 0b100 {
		0 => 8,
		_ => 16,
	};
	for sprite in sprites {
		if sprite.x <= lcd_x + 8 && sprite.x > lcd_x {
			let mut s_x = lcd_x + 8 - sprite.x;
			let mut s_y = lcd_y + 16 - sprite.y;

			if sprite.y_flip {
				s_y = sprite_h - 1 - s_y
			}
			if sprite.x_flip {
				s_x = 8 - 1 - s_x
			}

			let tile_data = &gb.bus.vram[sprite.itile * 16..sprite.itile * 16 + 32];

			let b1 = (tile_data[s_y * 2 + 0] >> (7 - s_x)) & 1;
			let b2 = (tile_data[s_y * 2 + 1] >> (7 - s_x)) & 1;
			let pallete_index = b1 | (b2 << 1);

			// sprite is transparent here
			if b1 | b2 == 0 {
				continue;
			}

			// sprite's (low-)priority flag is set, and background/window is non-zero
			if sprite.prio && map_pallete_index != 0 {
				continue;
			}

			let palette = match sprite.dmg_palette {
				false => gb.bus.io.obp0,
				true => gb.bus.io.obp1,
			};

			let (r, g, b) = color_dmg(pallete_index, palette);

			gb.framebuffer[0 + 3 * (lcd_x + 160 * lcd_y)] = r;
			gb.framebuffer[1 + 3 * (lcd_x + 160 * lcd_y)] = g;
			gb.framebuffer[2 + 3 * (lcd_x + 160 * lcd_y)] = b;

			break; // First sprite in the list wins
		}
	}
}
