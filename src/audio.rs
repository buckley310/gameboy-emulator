use sdl2::audio::{AudioDevice, AudioSpecDesired};
use std::sync::{Arc, Mutex};

const AUDIO_FREQ: i32 = 48_000;

#[derive(Default)]
struct Channel {
	sweep: u8,     // NRx0
	length: u8,    // NRx1
	volume: u8,    // NRx2
	frequency: u8, // NRx3
	control: u8,   // NRx4
	trigger: bool,
}

#[derive(Default)]
pub struct AudioParams {
	channels: [Channel; 4],
	nr50: u8,
	nr51: u8,
	nr52: u8,
	wave_ram: [u8; 16],
}
impl AudioParams {
	pub fn set(&mut self, addr: usize, data: u8) {
		match addr {
			0xFF10 => self.channels[0].sweep = data,
			0xFF11 => self.channels[0].length = data,
			0xFF12 => self.channels[0].volume = data,
			0xFF13 => self.channels[0].frequency = data,
			0xFF14 => {
				self.channels[0].control = data;
				if data & (1 << 7) != 0 {
					self.channels[0].trigger = true;
				}
			}
			0xFF15 => self.channels[1].sweep = data,
			0xFF16 => self.channels[1].length = data,
			0xFF17 => self.channels[1].volume = data,
			0xFF18 => self.channels[1].frequency = data,
			0xFF19 => {
				self.channels[1].control = data;
				if data & (1 << 7) != 0 {
					self.channels[1].trigger = true;
				}
			}
			0xFF1A => self.channels[2].sweep = data,
			0xFF1B => self.channels[2].length = data,
			0xFF1C => self.channels[2].volume = data,
			0xFF1D => self.channels[2].frequency = data,
			0xFF1E => {
				self.channels[2].control = data;
				if data & (1 << 7) != 0 {
					self.channels[2].trigger = true;
				}
			}
			0xFF1F => self.channels[3].sweep = data,
			0xFF20 => self.channels[3].length = data,
			0xFF21 => self.channels[3].volume = data,
			0xFF22 => self.channels[3].frequency = data,
			0xFF23 => {
				self.channels[3].control = data;
				if data & (1 << 7) != 0 {
					self.channels[3].trigger = true;
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
	audio_params: Arc<Mutex<AudioParams>>,

	c2_freq: u64,
	c2_freq_low: u64,
	c2_phase: u64,
	c2_volume: u8,
	c2_env: u64,
	c2_env_high: u64,
	c2_env_direction: bool,
}
impl PcmGenerator {
	fn new(audio_params: Arc<Mutex<AudioParams>>) -> Self {
		PcmGenerator {
			audio_params,

			c2_freq: 0,
			c2_freq_low: 0,
			c2_phase: 0,
			c2_volume: 0,
			c2_env: 0,
			c2_env_high: 0,
			c2_env_direction: false,
		}
	}
}
impl sdl2::audio::AudioCallback for PcmGenerator {
	type Channel = i32;
	fn callback(&mut self, out: &mut [i32]) {
		let mut audio_params = self.audio_params.lock().unwrap();
		if audio_params.channels[1].trigger {
			audio_params.channels[1].trigger = false;
			self.c2_env_high =
				(audio_params.channels[1].volume as u64 & 7) * (AUDIO_FREQ as u64) / 64;
			self.c2_env = self.c2_env_high;
			self.c2_env_direction = audio_params.channels[1].volume & 8 != 0;
			self.c2_volume = audio_params.channels[1].volume & 0xF0;
			let period = (audio_params.channels[1].frequency as u64)
				| ((audio_params.channels[1].control as u64 & 0b111) << 8);
			self.c2_freq = (AUDIO_FREQ as u64) * ((1 << 11) - period) / (1 << 17);
			self.c2_freq_low = match audio_params.channels[1].length >> 6 {
				0 => self.c2_freq >> 3,
				1 => self.c2_freq >> 2,
				2 => self.c2_freq >> 1,
				3 => (self.c2_freq >> 2) * 3,
				_ => panic!(),
			};
		}
		for x in out.iter_mut() {
			if self.c2_env_high != 0 {
				self.c2_env = self.c2_env.saturating_sub(1);
				if self.c2_env == 0 {
					self.c2_env = self.c2_env_high;
					self.c2_volume = match self.c2_env_direction {
						true => self.c2_volume.saturating_add(0x10),
						false => self.c2_volume.saturating_sub(0x10),
					};
				}
			}
			if self.c2_phase == 0 {
				self.c2_phase = self.c2_freq;
			} else {
				self.c2_phase -= 1;
			}
			*x = if self.c2_phase < self.c2_freq_low {
				-((self.c2_volume as i32) << 18)
			} else {
				(self.c2_volume as i32) << 18
			};
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
