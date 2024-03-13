use byteorder::ByteOrder;
use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use std::ffi::CStr;
use std::io::Read;
use std::net::TcpStream;
use std::os::raw::c_char;
use std::time::Instant;

// How many samples to cache before playing in samples (both channels) SHOULD BE EVEN
const BUFFER_SIZE: usize = 4410;
// How many buffers to cache
const CACHE_SIZE: usize = 20;

static mut SINK: Option<Box<Sink>> = None;
static mut STATE: State = State::NotStarted;

#[derive(PartialEq)]
enum State {
	NotStarted,
	Resetting,
	Playing,
	Paused,
}

#[no_mangle]
pub extern "C" fn start(server: *const c_char) {
	let serv = unsafe { CStr::from_ptr(server) };
	unsafe {
		run(match serv.to_str() {
			Ok(s) => s,
			_ => "",
		})
	}
}

#[no_mangle]
pub extern "C" fn toggle() {
	unsafe {
		if STATE == State::Playing {
			STATE = State::Paused;
			if let Some(sink) = &SINK {
				sink.pause();
			}
		} else if STATE == State::Paused {
			STATE = State::Playing;
			if let Some(sink) = &SINK {
				sink.play();
			}
		}
	}
}

#[no_mangle]
pub extern "C" fn reset() {
	unsafe {
		STATE = State::Resetting;
		if let Some(sink) = &SINK {
			sink.pause();
		}
		// Blocking main thread
		while STATE == State::Resetting {
			std::thread::sleep(std::time::Duration::from_secs_f32(0.01))
		}
	}
}

unsafe fn _reset() {
	if let Some(sink) = &SINK {
		sink.pause();
		sink.clear();
	}
	SINK = None;
	STATE = State::NotStarted;
}

// Reset - true, not - false
unsafe fn watching_sleep(dur: f32) -> bool {
	let start = Instant::now();
	while Instant::now() < start + std::time::Duration::from_secs_f32(dur) {
		std::thread::sleep(std::time::Duration::from_secs_f32(0.0001));
		if STATE == State::Resetting {
			return true;
		}
	}
	false
}

unsafe fn run(server: &str) {
	if STATE == State::Playing || STATE == State::Paused {
		return;
	}
	STATE = State::Playing;
	let mut stream = TcpStream::connect(server).unwrap();
	println!("Connected to {} from {}", stream.peer_addr().unwrap(), stream.local_addr().unwrap());
	let (_stream, stream_handle) = OutputStream::try_default().unwrap();
	match &SINK {
		None => {
			let sink = Sink::try_new(&stream_handle).unwrap();
			SINK = Some(Box::new(sink));
		}
		Some(s) => {
			if s.is_paused() {
				s.play()
			}
		}
	}
	let mut buffer = [0u8; 4];
	let mut samples = [0f32; BUFFER_SIZE];
	let mut index = 0usize;
	while stream.read_exact(&mut buffer).is_ok() {
		while STATE == State::Paused {
			std::thread::sleep(std::time::Duration::from_secs_f32(0.25))
		}
		if STATE == State::Resetting {
			_reset();
			return;
		}
		let sample_l = byteorder::LittleEndian::read_i16(&buffer[..2]) as f32 / 32768.0;
		let sample_r = byteorder::LittleEndian::read_i16(&buffer[2..]) as f32 / 32768.0;
		// Left channel
		samples[index] = sample_l;
		index += 1;
		// Right channel
		samples[index] = sample_r;
		index += 1;
		if index == BUFFER_SIZE {
			// Sink's thread is detached from main thread, so we need to synchronize with it
			// Why we should synchronize with it?
			// Let's say, that if we don't synchronize with it, we would have
			// a lot (no upper limit, actualy) of buffered sound, waiting for playing in sink
			if let Some(sink) = &SINK {
				while sink.len() >= CACHE_SIZE {
					// Sleeping exactly one buffer and watching for reset signal
					if watching_sleep(
						if sink.len() > 2 {
							sink.len() as f32 - 2.0
						} else {
							0.5
						} * BUFFER_SIZE as f32 / 44100.0
							/ 2.0,
					) {
						_reset();
						return;
					}
				}
				sink.append(SamplesBuffer::new(2, 44100, samples.as_slice()));
				index = 0;
			}
		}
	}
}
