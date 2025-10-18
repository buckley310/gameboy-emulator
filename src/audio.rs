use raylib::prelude::*;
use std::sync::{Arc, Mutex};

const AUDIO_FREQ: u16 = 48_000;
const MASTER_VOLUME: f32 = 0.2;

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

	c2_freq: usize,
	c2_freq_low: usize,
	c2_phase: usize,
	c2_volume: u8,
	c2_env: usize,
	c2_env_high: usize,
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
	fn callback(&mut self, buf: &mut [u8]) {
		fn from_ticks(tick_hz: usize, n: usize) -> usize {
			(AUDIO_FREQ as usize) * n / tick_hz
		}
		let mut audio_params = self.audio_params.lock().unwrap();
		if audio_params.channels[1].trigger {
			audio_params.channels[1].trigger = false;
			self.c2_env_high = from_ticks(64, audio_params.channels[1].volume as usize & 7);
			self.c2_env = self.c2_env_high;
			self.c2_env_direction = audio_params.channels[1].volume & 8 != 0;
			self.c2_volume = audio_params.channels[1].volume & 0xF0;
			let period = (audio_params.channels[1].frequency as usize)
				| ((audio_params.channels[1].control as usize & 0b111) << 8);
			self.c2_freq = from_ticks(1 << 17, (1 << 11) - period);
			self.c2_freq_low = match audio_params.channels[1].length >> 6 {
				0 => self.c2_freq >> 3,
				1 => self.c2_freq >> 2,
				2 => self.c2_freq >> 1,
				3 => (self.c2_freq >> 2) * 3,
				_ => panic!(),
			};
		}
		for out in buf.iter_mut() {
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
			self.c2_phase = self.c2_phase.saturating_sub(1);
			if self.c2_phase == 0 {
				self.c2_phase = self.c2_freq;
			}
			let magnitude = (self.c2_volume as f32) / 256.0 * MASTER_VOLUME;
			*out = if self.c2_phase < self.c2_freq_low {
				128 - (magnitude * 127.0) as u8
			} else {
				128 + (magnitude * 127.0) as u8
			};
		}
	}
}

pub fn init_audio() -> RaylibAudio {
	RaylibAudio::init_audio_device().expect("audio init failed")
}

pub struct APU<'a> {
	pub audio_params: Arc<Mutex<AudioParams>>,

	// Keep the stream around. It closes if it goes out of scope.
	#[allow(dead_code)]
	audio_stream: AudioStream<'a>,
}
impl<'a> APU<'a> {
	pub fn new(device: &'a RaylibAudio) -> Self {
		let audio_params = Arc::new(Mutex::new(AudioParams::default()));
		let stream = device.new_audio_stream(AUDIO_FREQ as u32, 8, 1);

		let mut pcm_generator = PcmGenerator::new(audio_params.clone());

		audio_stream_callback::set_audio_stream_callback(&stream, move |buf: &mut [u8]| {
			pcm_generator.callback(buf);
		})
		.expect("error activating audio callback");

		stream.play();

		Self {
			audio_params,
			audio_stream: stream,
		}
	}
}
