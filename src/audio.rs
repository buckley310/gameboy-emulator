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
	// fn get_trigger(&self) -> bool {
	// 	self.nr[4] & 0b1000_0000 != 0
	// }
	// fn get_length_enable(&self) -> bool {
	// 	todo!("check this");
	// 	self.nr[4] & 0b0100_0000 != 0
	// }
	fn get_period(&self) -> usize {
		(self.nr[3] as usize) | ((self.nr[4] as usize & 0b111) << 8)
	}
	fn get_pulse_duty_cycle(&self) -> u8 {
		self.nr[1] >> 6
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

struct AudioBuffer {
	head: usize,
	tail: usize,
	buf: Vec<i16>,
}
impl AudioBuffer {
	fn new() -> Self {
		let mut buf = Vec::new();
		buf.reserve_exact(0xffff + 1);
		for _ in 0..=0xffff {
			buf.push(0)
		}
		Self {
			head: 0,
			tail: 0,
			buf: buf,
		}
	}
	fn add(&mut self, val: i16) {
		let ofs = self.head;
		self.head += 1;
		self.buf[ofs & 0xffff] = val;
	}
	fn take(&mut self, size: usize) -> Vec<i16> {
		let mut dest = Vec::new();
		dest.reserve_exact(size);
		let ofs = self.tail & 0xffff;
		self.tail += size;
		if ofs + size <= 0xffff + 1 {
			for x in &self.buf[ofs..ofs + size] {
				dest.push(*x);
			}
		} else {
			for x in &self.buf[ofs..] {
				dest.push(*x);
			}
			for x in &self.buf[..size - dest.len()] {
				dest.push(*x);
			}
		}
		assert!(dest.len() == size);
		dest
	}
}

pub struct APU<'a> {
	ring: AudioBuffer,
	next_sample: u64,
	sample_number: u64,

	pulse2_period_div: usize,
	pulse2_current_sample: u8,

	// Keep the stream around. It closes if it goes out of scope.
	#[allow(dead_code)]
	audio_stream: AudioStream<'a>,
}
impl<'a> APU<'a> {
	pub fn new(device: &'a RaylibAudio) -> Self {
		let stream = device.new_audio_stream(AUDIO_FREQ as u32, 16, 1);
		let ring = AudioBuffer::new();

		stream.set_volume(VOLUME_DIAL);
		stream.play();

		Self {
			ring,
			next_sample: 0,
			sample_number: 0,
			pulse2_period_div: 0,
			pulse2_current_sample: 0,

			audio_stream: stream,
		}
	}
	pub fn tick(&mut self, gb: &mut GB, dots: u64) {
		// Pulse 2 Channel
		if dots & 0b11 == 0 {
			self.pulse2_period_div += 1;
			if self.pulse2_period_div >= 0x800 {
				self.pulse2_current_sample += 1;
				self.pulse2_current_sample &= 7;
				self.pulse2_period_div = gb.bus.io.audio_params.channels[1].get_period();
			}
		}

		if dots > self.next_sample {
			self.sample_number += 1;

			self.next_sample = (
				// TODO: check if u128 is needed, or if u64 is enough.
				// Will { sample_number*DOTS_HZ } ever overflow a u64?
				self.sample_number as u128 * DOTS_HZ as u128 / AUDIO_FREQ as u128
			) as u64;

			let c2 = {
				let c2_is_low = match gb.bus.io.audio_params.channels[1].get_pulse_duty_cycle() {
					0 => self.pulse2_current_sample == 0,
					1 => self.pulse2_current_sample <= 1,
					2 => self.pulse2_current_sample <= 3,
					_ => self.pulse2_current_sample <= 5,
				};
				if c2_is_low { i16::MIN } else { i16::MAX }
			};

			self.ring.add(c2 / 4);
			if self.audio_stream.is_processed() {
				self.audio_stream
					.update(&self.ring.take(AUDIO_BUFFER_SIZE))
					.unwrap();
			}
		}
	}
}
