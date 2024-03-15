//! A library implementing the lonely radio audio streaming protocol
//!
//! Example usage (play for 10 seconds):
//! ```rust
//! extern crate monolib;
//! use std::thread::{sleep, spawn};
//! use std::time::Duration;
//!
//! spawn(|| monolib::run("someserver:someport"));
//! while monolib::get_metadata().is_none() {}
//! let seconds = md.length / md.sample_rate as u64 / 2;
//! println!("Playing: {} - {} - {} ({}:{:02})", md.artist, md.album, md.title, seconds / 60, seconds % 60);
//! sleep(Duration::from_secs(10));
//! monolib::stop();
//!```

/// Functions, providing C-like API
pub mod c;

use byteorder::ByteOrder;
use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use serde::Deserialize;
use std::io::Read;
use std::net::TcpStream;
use std::sync::RwLock;
use std::time::Instant;

// How many samples to cache before playing in samples (both channels) SHOULD BE EVEN
const BUFFER_SIZE: usize = 4800;
// How many buffers to cache
const CACHE_SIZE: usize = 160;

static SINK: RwLock<Option<Sink>> = RwLock::new(None);
static MD: RwLock<Option<Metadata>> = RwLock::new(None);
static STATE: RwLock<State> = RwLock::new(State::NotStarted);

/// Player state
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum State {
	NotStarted = 0,
	Resetting = 1,
	Playing = 2,
	Paused = 3,
}

/// Track metadata
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Metadata {
	/// In samples, length / (sample_rate * 2 (channels)) = length in seconds
	pub length: u64,
	pub sample_rate: u32,
	pub title: String,
	pub album: String,
	pub artist: String,
}

/// Play/pauses playback
pub fn toggle() {
	let mut state = crate::STATE.write().unwrap();
	if *state == State::Playing {
		*state = State::Paused;

		let sink = SINK.read().unwrap();
		if let Some(sink) = sink.as_ref() {
			sink.pause()
		}
	} else if *state == State::Paused {
		*state = State::Playing;

		let sink = SINK.read().unwrap();
		if let Some(sink) = sink.as_ref() {
			sink.play()
		}
	}
}

/// Stops playback
pub fn stop() {
	let mut state = STATE.write().unwrap();
	*state = State::Resetting;

	let sink = SINK.read().unwrap();
	if let Some(sink) = sink.as_ref() {
		sink.pause()
	}
	drop(sink);
	drop(state);
	// Blocking main thread
	while *STATE.read().unwrap() == State::Resetting {
		std::thread::sleep(std::time::Duration::from_secs_f32(0.01))
	}
}

pub fn get_state() -> State {
	*STATE.read().unwrap()
}

pub fn get_metadata() -> Option<Metadata> {
	MD.read().unwrap().clone()
}

fn _stop() {
	let sink = SINK.read().unwrap();
	if let Some(sink) = sink.as_ref() {
		sink.pause();
		sink.clear();
	}

	let mut md = MD.write().unwrap();
	if md.is_some() {
		*md = None;
	}

	*STATE.write().unwrap() = State::NotStarted;
}

// Reset - true, not - false
fn watching_sleep(dur: f32) -> bool {
	let start = Instant::now();
	while Instant::now() < start + std::time::Duration::from_secs_f32(dur) {
		std::thread::sleep(std::time::Duration::from_secs_f32(0.0001));
		if *STATE.read().unwrap() == State::Resetting {
			return true;
		}
	}
	false
}

/// Starts playing at "server:port"
pub fn run(server: &str) {
	let mut state = STATE.write().unwrap();
	if *state == State::Playing || *state == State::Paused {
		return;
	}
	*state = State::Playing;
	drop(state);

	let mut stream = TcpStream::connect(server).unwrap();
	let mut sink = SINK.write().unwrap();
	let (_stream, stream_handle) = OutputStream::try_default().unwrap();

	// Can't reuse old sink for some reason
	let audio_sink = Sink::try_new(&stream_handle).unwrap();
	*sink = Some(audio_sink);
	drop(sink);

	let mut buffer = [0u8; 2];
	let mut samples = [0f32; BUFFER_SIZE];
	loop {
		let mut index = 0usize;
		let recv_md: Metadata =
			rmp_serde::from_read(&stream).expect("Failed to parse track metadata");

		let mut md = MD.write().unwrap();
		*md = Some(recv_md.clone());
		drop(md);
		for _ in 0..recv_md.length {
			while *STATE.read().unwrap() == State::Paused {
				std::thread::sleep(std::time::Duration::from_secs_f32(0.25))
			}
			if *STATE.read().unwrap() == State::Resetting {
				_stop();
				return;
			}

			if stream.read_exact(&mut buffer).is_err() {
				return;
			};

			samples[index] = byteorder::LittleEndian::read_i16(&buffer[..2]) as f32 / 32768.0;
			index += 1;

			if index == BUFFER_SIZE {
				// Sink's thread is detached from main thread, so we need to synchronize with it
				// Why we should synchronize with it?
				// Let's say, that if we don't synchronize with it, we would have
				// a lot (no upper limit, actualy) of buffered sound, waiting for playing in sink
				let sink = SINK.read().unwrap();
				if let Some(sink) = sink.as_ref() {
					while sink.len() >= CACHE_SIZE {
						// Sleeping exactly one buffer and watching for reset signal
						if watching_sleep(
							if sink.len() > 2 {
								sink.len() as f32 - 2.0
							} else {
								0.5
							} * BUFFER_SIZE as f32 / recv_md.sample_rate as f32
								/ 2.0,
						) {
							_stop();
							return;
						}
					}
					sink.append(SamplesBuffer::new(2, recv_md.sample_rate, samples.as_slice()));
					index = 0;
				}
			}
		}
	}
}
