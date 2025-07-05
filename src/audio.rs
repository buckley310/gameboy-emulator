use sdl2::audio::{AudioDevice, AudioSpecDesired};
use std::sync::{Arc, Mutex};

const AUDIO_FREQ: i32 = 48_000;

#[derive(Default)]
pub struct AudioParams {
	nr10: u8,
	nr11: u8,
	nr12: u8,
	nr13: u8,
	nr14: u8,
	trigger1: bool,

	nr21: u8,
	nr22: u8,
	nr23: u8,
	nr24: u8,
	trigger2: bool,

	nr30: u8,
	nr31: u8,
	nr32: u8,
	nr33: u8,
	nr34: u8,
	trigger3: bool,

	nr41: u8,
	nr42: u8,
	nr43: u8,
	nr44: u8,
	trigger4: bool,

	nr50: u8,
	nr51: u8,
	nr52: u8,
	wave_ram: [u8; 16],
}
impl AudioParams {
	pub fn set(&mut self, addr: usize, data: u8) {
		match addr {
			0xFF10 => self.nr10 = data,
			0xFF11 => self.nr11 = data,
			0xFF12 => self.nr12 = data,
			0xFF13 => self.nr13 = data,
			0xFF14 => {
				self.nr14 = data;
				if data & (1 << 7) != 0 {
					self.trigger1 = true;
				}
			}
			0xFF16 => self.nr21 = data,
			0xFF17 => self.nr22 = data,
			0xFF18 => self.nr23 = data,
			0xFF19 => {
				self.nr24 = data;
				if data & (1 << 7) != 0 {
					self.trigger2 = true;
				}
			}
			0xFF1A => self.nr30 = data,
			0xFF1B => self.nr31 = data,
			0xFF1C => self.nr32 = data,
			0xFF1D => self.nr33 = data,
			0xFF1E => {
				self.nr34 = data;
				if data & (1 << 7) != 0 {
					self.trigger3 = true;
				}
			}
			0xFF20 => self.nr41 = data,
			0xFF21 => self.nr42 = data,
			0xFF22 => self.nr43 = data,
			0xFF23 => {
				self.nr44 = data;
				if data & (1 << 7) != 0 {
					self.trigger4 = true;
				}
			}
			0xFF24 => self.nr50 = data,
			0xFF25 => self.nr51 = data,
			0xFF26 => self.nr52 = data,
			0xFF30..=0xFF3F => self.wave_ram[addr - 0xFF30] = data,
			_ => println!("invalid audio write! {addr:#x}"),
		}
	}
}

pub struct PcmGenerator {
	phase: i32,
	audio_params: Arc<Mutex<AudioParams>>,
}
impl PcmGenerator {
	fn new(audio_params: Arc<Mutex<AudioParams>>) -> Self {
		PcmGenerator {
			phase: 0,
			audio_params,
		}
	}
}
impl sdl2::audio::AudioCallback for PcmGenerator {
	type Channel = i32;
	fn callback(&mut self, out: &mut [i32]) {
		// very incomplete, only reads channel 2
		let audio_params = self.audio_params.lock().unwrap();
		let vol = (audio_params.nr22 as i32) << 18;
		for x in out.iter_mut() {
			*x = if self.phase < 0x800 { vol } else { -vol };
			self.phase += audio_params.nr23 as i32;
			self.phase &= 0xfff;
		}
	}
}

pub struct APU {
	pub audio_params: Arc<Mutex<AudioParams>>,
	pub device: AudioDevice<PcmGenerator>,
}
impl std::default::Default for APU {
	fn default() -> Self {
		let audio_params = Arc::new(Mutex::new(AudioParams::default()));
		let sdl_context = sdl2::init().unwrap();
		let audio_subsystem = sdl_context.audio().unwrap();
		let desired_spec = AudioSpecDesired {
			freq: Some(AUDIO_FREQ),
			channels: Some(1),
			samples: Some((AUDIO_FREQ / 100) as u16),
		};
		let device = audio_subsystem
			.open_playback(None, &desired_spec, |_| {
				PcmGenerator::new(audio_params.clone())
			})
			.unwrap();
		Self {
			audio_params,
			device,
		}
	}
}
