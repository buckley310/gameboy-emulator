use crate::ioreg::AUDIO_PARAMS_SIZE;
use sdl2::audio::{AudioDevice, AudioSpecDesired};
use std::sync::{Arc, Mutex};

static AUDIO_FREQ: i32 = 48_000;

pub struct PcmGenerator {
	callback_i: u64,
	phase: i32,
	audio_params: Arc<Mutex<[(u8, bool); AUDIO_PARAMS_SIZE]>>,
}
impl PcmGenerator {
	fn new(audio_params: Arc<Mutex<[(u8, bool); AUDIO_PARAMS_SIZE]>>) -> Self {
		PcmGenerator {
			callback_i: 0,
			phase: 0,
			audio_params,
		}
	}
}
impl sdl2::audio::AudioCallback for PcmGenerator {
	type Channel = i32;
	fn callback(&mut self, out: &mut [i32]) {
		println!("snd {} {}", out.len(), self.callback_i);
		self.callback_i += 1;

		// very incomplete, only reads channel 2
		let audio_params = self.audio_params.lock().unwrap();
		let vol = (audio_params[0x7].0 as i32) << 18;
		for x in out.iter_mut() {
			*x = if self.phase < 0x800 { vol } else { -vol };
			self.phase += audio_params[0x8].0 as i32;
			self.phase &= 0xfff;
		}
	}
}

pub struct APU {
	pub audio_params: Arc<Mutex<[(u8, bool); AUDIO_PARAMS_SIZE]>>,
	pub device: AudioDevice<PcmGenerator>,
}
impl std::default::Default for APU {
	fn default() -> Self {
		let audio_params = Arc::new(Mutex::new([(0, false); AUDIO_PARAMS_SIZE]));
		let sdl_context = sdl2::init().unwrap();
		let audio_subsystem = sdl_context.audio().unwrap();
		let desired_spec = AudioSpecDesired {
			freq: Some(AUDIO_FREQ),
			channels: Some(1),
			samples: None, // TODO: manual sample count?
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
