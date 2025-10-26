use crate::{DOTS_HZ, GB};
use raylib::prelude::*;

const AUDIO_FREQ: u16 = 48_000;
const VOLUME_DIAL: f32 = 0.25;
const AUDIO_BUFFER_SIZE: usize = 0x1000;

#[derive(Default)]
struct Channel {
	// 	sweep: u8,     // NRx0
	// 	length: u8,    // NRx1
	// 	volume: u8,    // NRx2
	// 	frequency: u8, // NRx3
	// 	control: u8,   // NRx4
	nr: [u8; 5],
	trigger: bool,
}
impl Channel {
	// fn get_trigger_enable(&self) {}
	fn get_length_enable(&self) -> bool {
		self.nr[4] & 0b_0100_0000 != 0
	}
	fn get_pulse_length(&self) -> u8 {
		self.nr[1] & 0b_11_1111
	}
	fn set_pulse_length(&mut self, n: u8) {
		self.nr[1] &= 0b_1100_0000;
		self.nr[1] |= 0b_0011_1111 & n;
	}
	fn get_noise_length(&self) -> u8 {
		self.nr[1] & 0b_11_1111
	}
	fn set_noise_length(&mut self, n: u8) {
		self.nr[1] &= 0b_1100_0000;
		self.nr[1] |= 0b_0011_1111 & n;
	}
	fn get_wave_length(&self) -> u8 {
		self.nr[1]
	}
	fn set_wave_length(&mut self, n: u8) {
		self.nr[1] = n;
	}
	// fn get_noise_length(&self) {}
	// fn get_wave_length(&self) {}
	// fn get_pulse_freq_sweep_pace(&self) {}
	// fn get_pulse_freq_sweep_dir(&self) {}
	// fn get_pulse_freq_sweep_step(&self) {}
	fn get_period(&self) -> usize {
		(self.nr[3] as usize) | ((self.nr[4] as usize & 0b111) << 8)
	}
	fn get_pulse_duty_cycle(&self) -> u8 {
		self.nr[1] >> 6
	}
	fn get_pulse_volume(&self) -> u8 {
		self.nr[2] >> 4
	}
	fn get_pulse_env_dir(&self) -> bool {
		self.nr[2] & 0b1000 != 0
	}
	fn get_pulse_env_pace(&self) -> u8 {
		self.nr[2] & 3
	}
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
			0xFF10 => self.channels[0].nr[0] = data,
			0xFF11 => self.channels[0].nr[1] = data,
			0xFF12 => self.channels[0].nr[2] = data,
			0xFF13 => self.channels[0].nr[3] = data,
			0xFF14 => {
				self.channels[0].nr[4] = data;
				if data & (1 << 7) != 0 {
					self.channels[0].trigger = true;
				}
			}
			0xFF15 => self.channels[1].nr[0] = data,
			0xFF16 => self.channels[1].nr[1] = data,
			0xFF17 => self.channels[1].nr[2] = data,
			0xFF18 => self.channels[1].nr[3] = data,
			0xFF19 => {
				self.channels[1].nr[4] = data;
				if data & (1 << 7) != 0 {
					self.channels[1].trigger = true;
				}
			}
			0xFF1A => self.channels[2].nr[0] = data,
			0xFF1B => self.channels[2].nr[1] = data,
			0xFF1C => self.channels[2].nr[2] = data,
			0xFF1D => self.channels[2].nr[3] = data,
			0xFF1E => {
				self.channels[2].nr[4] = data;
				if data & (1 << 7) != 0 {
					self.channels[2].trigger = true;
				}
			}
			0xFF1F => self.channels[3].nr[0] = data,
			0xFF20 => self.channels[3].nr[1] = data,
			0xFF21 => self.channels[3].nr[2] = data,
			0xFF22 => self.channels[3].nr[3] = data,
			0xFF23 => {
				self.channels[3].nr[4] = data;
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

pub fn init_audio() -> RaylibAudio {
	let s = RaylibAudio::init_audio_device().expect("audio init failed");
	s.set_audio_stream_buffer_size_default(AUDIO_BUFFER_SIZE as i32);
	s
}

pub struct APU<'a> {
	next_sample: u64,
	sample_number: u64,

	audio_buffer: Box<[i16; AUDIO_BUFFER_SIZE]>,
	audio_buffer_ofs: usize,

	pulse1_enabled: bool,
	pulse1_period_div: usize,
	pulse1_current_sample: u8,
	pulse1_internal_volume: u8,
	pulse1_internal_env_counter: u8,
	pulse1_internal_env_pace: u8,
	pulse1_internal_env_dir: bool,

	pulse2_enabled: bool,
	pulse2_period_div: usize,
	pulse2_current_sample: u8,
	pulse2_internal_volume: u8,
	pulse2_internal_env_counter: u8,
	pulse2_internal_env_pace: u8,
	pulse2_internal_env_dir: bool,

	// Keep the stream around. It closes if it goes out of scope.
	#[allow(dead_code)]
	audio_stream: AudioStream<'a>,
}
impl<'a> APU<'a> {
	pub fn new(device: &'a RaylibAudio) -> Self {
		let stream = device.new_audio_stream(AUDIO_FREQ as u32, 16, 1);

		stream.set_volume(VOLUME_DIAL);
		stream.play();

		Self {
			next_sample: 0,
			sample_number: 0,

			audio_buffer: Box::new([0; AUDIO_BUFFER_SIZE]),
			audio_buffer_ofs: 0,

			pulse1_enabled: false,
			pulse1_period_div: 0,
			pulse1_current_sample: 0,
			pulse1_internal_volume: 0,
			pulse1_internal_env_counter: 0,
			pulse1_internal_env_pace: 0,
			pulse1_internal_env_dir: false,

			pulse2_enabled: false,
			pulse2_period_div: 0,
			pulse2_current_sample: 0,
			pulse2_internal_volume: 0,
			pulse2_internal_env_counter: 0,
			pulse2_internal_env_pace: 0,
			pulse2_internal_env_dir: false,

			audio_stream: stream,
		}
	}
	pub fn tick(&mut self, gb: &mut GB, dots: u64) {
		if gb.bus.io.audio_params.channels[0].trigger {
			gb.bus.io.audio_params.channels[0].trigger = false;
			self.pulse1_enabled = true;
			self.pulse1_internal_volume = gb.bus.io.audio_params.channels[0].get_pulse_volume();
			self.pulse1_internal_env_dir = gb.bus.io.audio_params.channels[0].get_pulse_env_dir();
			self.pulse1_internal_env_pace = gb.bus.io.audio_params.channels[0].get_pulse_env_pace();
			self.pulse1_internal_env_counter = self.pulse1_internal_env_pace;
		}
		if gb.bus.io.audio_params.channels[1].trigger {
			gb.bus.io.audio_params.channels[1].trigger = false;
			self.pulse2_enabled = true;
			self.pulse2_internal_volume = gb.bus.io.audio_params.channels[1].get_pulse_volume();
			self.pulse2_internal_env_dir = gb.bus.io.audio_params.channels[1].get_pulse_env_dir();
			self.pulse2_internal_env_pace = gb.bus.io.audio_params.channels[1].get_pulse_env_pace();
			self.pulse2_internal_env_counter = self.pulse2_internal_env_pace;
		}
		if gb.bus.io.audio_params.channels[2].trigger {
			gb.bus.io.audio_params.channels[2].trigger = false;
		}
		if gb.bus.io.audio_params.channels[3].trigger {
			gb.bus.io.audio_params.channels[3].trigger = false;
		}

		// TODO: there should be a DIV-APU register, but for now, just read dots instead

		// every 4 dots
		if dots & 0b11 == 0 {
			self.pulse1_period_div += 1;
			if self.pulse1_period_div >= 0x800 {
				self.pulse1_current_sample += 1;
				self.pulse1_current_sample &= 7;
				self.pulse1_period_div = gb.bus.io.audio_params.channels[0].get_period();
			}
			self.pulse2_period_div += 1;
			if self.pulse2_period_div >= 0x800 {
				self.pulse2_current_sample += 1;
				self.pulse2_current_sample &= 7;
				self.pulse2_period_div = gb.bus.io.audio_params.channels[1].get_period();
			}
		}

		// 64 hz
		// TODO: verify this math
		if dots & ((DOTS_HZ as u64 >> 6) - 1) == 0 {
			if self.pulse1_internal_env_pace != 0 && self.pulse1_internal_env_counter == 0 {
				self.pulse1_internal_env_counter = self.pulse1_internal_env_pace;
				self.pulse1_internal_volume = match self.pulse1_internal_env_dir {
					true => (self.pulse1_internal_volume + 1).min(0xf),
					false => self.pulse1_internal_volume.saturating_sub(1),
				}
			} else {
				self.pulse1_internal_env_counter =
					self.pulse1_internal_env_counter.saturating_sub(1);
			}
			if self.pulse2_internal_env_pace != 0 && self.pulse2_internal_env_counter == 0 {
				self.pulse2_internal_env_counter = self.pulse2_internal_env_pace;
				self.pulse2_internal_volume = match self.pulse2_internal_env_dir {
					true => (self.pulse2_internal_volume + 1).min(0xf),
					false => self.pulse2_internal_volume.saturating_sub(1),
				}
			} else {
				self.pulse2_internal_env_counter =
					self.pulse2_internal_env_counter.saturating_sub(1);
			}
		}

		// 128 hz
		// TODO: verify this math
		if dots & ((DOTS_HZ as u64 >> 7) - 1) == 0 {
			// TODO: ch1 frequency sweep
		}

		// 256 hz
		// TODO: verify this math
		if dots & ((DOTS_HZ as u64 >> 8) - 1) == 0 {
			if gb.bus.io.audio_params.channels[0].get_length_enable() {
				let len = gb.bus.io.audio_params.channels[0].get_pulse_length();
				if len == 63 {
					self.pulse1_enabled = false;
				}
				gb.bus.io.audio_params.channels[0].set_pulse_length(len.saturating_add(1));
			}
			if gb.bus.io.audio_params.channels[1].get_length_enable() {
				let len = gb.bus.io.audio_params.channels[1].get_pulse_length();
				if len == 63 {
					self.pulse2_enabled = false;
				}
				gb.bus.io.audio_params.channels[1].set_pulse_length(len.saturating_add(1));
			}
			if gb.bus.io.audio_params.channels[2].get_length_enable() {
				// length timer max == 256
			}
			if gb.bus.io.audio_params.channels[3].get_length_enable() {
				// length timer max == 64
			}
		}

		if dots > self.next_sample {
			self.sample_number += 1;

			self.next_sample = (
				// TODO: check if u128 is needed, or if u64 is enough.
				// Will { sample_number*DOTS_HZ } ever overflow a u64?
				self.sample_number as u128 * DOTS_HZ as u128 / AUDIO_FREQ as u128
			) as u64;

			let c1 = if self.pulse1_enabled {
				let c1_is_low = match gb.bus.io.audio_params.channels[0].get_pulse_duty_cycle() {
					0 => self.pulse1_current_sample == 0,
					1 => self.pulse1_current_sample <= 1,
					2 => self.pulse1_current_sample <= 3,
					_ => self.pulse1_current_sample <= 5,
				};
				let out = if c1_is_low { i16::MIN } else { i16::MAX };
				(out / 16) * self.pulse1_internal_volume as i16
			} else {
				0
			};

			let c2 = if self.pulse2_enabled {
				let c2_is_low = match gb.bus.io.audio_params.channels[1].get_pulse_duty_cycle() {
					0 => self.pulse2_current_sample == 0,
					1 => self.pulse2_current_sample <= 1,
					2 => self.pulse2_current_sample <= 3,
					_ => self.pulse2_current_sample <= 5,
				};
				let out = if c2_is_low { i16::MIN } else { i16::MAX };
				(out / 16) * self.pulse2_internal_volume as i16
			} else {
				0
			};

			let c3 = { 0 };

			let c4 = { 0 };

			self.audio_buffer[self.audio_buffer_ofs] = c1 / 4 + c2 / 4 + c3 / 4 + c4 / 4;
			if self.audio_buffer_ofs < AUDIO_BUFFER_SIZE - 1 {
				self.audio_buffer_ofs += 1;
			} else if self.audio_stream.is_processed() {
				self.audio_stream
					.update(self.audio_buffer.as_slice())
					.unwrap();
				self.audio_buffer_ofs = 0;
			}
		}
	}
}
