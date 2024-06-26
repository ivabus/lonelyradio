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
//! ```

/// Functions, providing C-like API
pub mod c;
mod reader;

use byteorder::{LittleEndian, ReadBytesExt};
use lonelyradio_types::{Message, TrackMetadata};
use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use std::error::Error;
use std::io::{BufReader, Read};
use std::net::TcpStream;
use std::sync::atomic::AtomicU8;
use std::sync::RwLock;
use std::time::Instant;

const CACHE_SIZE: usize = 128;

static SINK: RwLock<Option<Sink>> = RwLock::new(None);
static VOLUME: AtomicU8 = AtomicU8::new(255);
static MD: RwLock<Option<TrackMetadata>> = RwLock::new(None);
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
	if *state == State::NotStarted {
		return;
	}
	*state = State::Resetting;

	let sink = SINK.read().unwrap();
	if let Some(sink) = sink.as_ref() {
		sink.pause()
	}
	drop(sink);
	drop(state);

	// Blocking main thread
	while *STATE.read().unwrap() == State::Resetting {
		std::thread::sleep(std::time::Duration::from_secs_f32(0.1))
	}
}

pub fn get_state() -> State {
	*STATE.read().unwrap()
}

pub fn get_metadata() -> Option<TrackMetadata> {
	MD.read().unwrap().clone()
}

fn _stop() {
	let sink = SINK.read().unwrap();
	if let Some(sink) = sink.as_ref() {
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
		std::thread::sleep(std::time::Duration::from_secs_f32(0.01));
		if *STATE.read().unwrap() == State::Resetting {
			return true;
		}
	}
	false
}

fn watching_sleep_until_end() -> bool {
	while SINK.read().unwrap().as_ref().unwrap().len() != 0 {
		std::thread::sleep(std::time::Duration::from_secs_f32(0.01));
		if *STATE.read().unwrap() == State::Resetting {
			return true;
		}
	}
	false
}

pub fn get_volume() -> u8 {
	VOLUME.load(std::sync::atomic::Ordering::Acquire)
}

pub fn set_volume(volume: u8) {
	let sink = SINK.read().unwrap();
	if let Some(sink) = sink.as_ref() {
		sink.set_volume(get_volume() as f32 / 255.0)
	}
	VOLUME.store(volume, std::sync::atomic::Ordering::Relaxed)
}

/// Download track as samples
pub fn get_track(server: &str, xor_key: Option<Vec<u8>>) -> Option<(TrackMetadata, Vec<i16>)> {
	let mut stream = BufReader::new(match xor_key {
		Some(k) => reader::Reader::XorEncrypted(TcpStream::connect(server).unwrap(), k, 0),
		None => reader::Reader::Unencrypted(TcpStream::connect(server).unwrap()),
	});

	let mut samples = vec![];
	let mut md: Option<TrackMetadata> = None;
	loop {
		let recv_md: Message = rmp_serde::from_read(&mut stream).expect("Failed to parse message");
		match recv_md {
			Message::T(tmd) => {
				if md.is_some() {
					break;
				}
				md = Some(tmd);
			}
			Message::F(fmd) => {
				if !md.clone().unwrap().flac {
					let mut buf = vec![0; fmd.length as usize];
					stream.read_i16_into::<LittleEndian>(&mut buf).unwrap();
					samples.append(&mut buf);
				} else {
					let take = stream.by_ref().take(fmd.length);
					let mut reader = claxon::FlacReader::new(take).unwrap();
					samples.append(
						&mut reader.samples().map(|x| x.unwrap_or(0) as i16).collect::<Vec<i16>>(),
					);
				}
			}
		}
	}
	md.map(|md| (md, samples))
}

fn unwrap<T, E: Error>(thing: Result<T, E>) -> T {
	if thing.is_err() {
		*STATE.write().unwrap() = State::NotStarted;
	}
	thing.unwrap()
}

/// Starts playing at "server:port"
pub fn run(server: &str, xor_key: Option<Vec<u8>>) {
	let mut state = STATE.write().unwrap();
	if *state == State::Playing || *state == State::Paused {
		return;
	}
	*state = State::Playing;
	drop(state);

	let mut stream = BufReader::new(match xor_key {
		Some(k) => reader::Reader::XorEncrypted(unwrap(TcpStream::connect(server)), k, 0),
		None => reader::Reader::Unencrypted(unwrap(TcpStream::connect(server))),
	});

	let mut sink = SINK.write().unwrap();
	let (_stream, stream_handle) = unwrap(OutputStream::try_default());

	// Can't reuse old sink for some reason
	let audio_sink = Sink::try_new(&stream_handle).unwrap();
	*sink = Some(audio_sink);
	drop(sink);

	let mut samples = Vec::with_capacity(8192);
	loop {
		let recv_md: Message = rmp_serde::from_read(&mut stream).expect("Failed to parse message");
		match recv_md {
			Message::T(tmd) => {
				// No metadata shift
				if watching_sleep_until_end() {
					_stop();
					return;
				}
				let mut md = MD.write().unwrap();
				*md = Some(tmd.clone());
				drop(md);
			}
			Message::F(fmd) => {
				while *STATE.read().unwrap() == State::Paused {
					std::thread::sleep(std::time::Duration::from_secs_f32(0.25))
				}
				if *STATE.read().unwrap() == State::Resetting {
					_stop();
					return;
				}
				if !MD.read().unwrap().clone().unwrap().flac {
					let mut samples_i16 = vec![0; fmd.length as usize];
					if stream.read_i16_into::<LittleEndian>(&mut samples_i16).is_err() {
						return;
					};
					samples.append(
						&mut samples_i16.iter().map(|sample| *sample as f32 / 32767.0).collect(),
					);
				} else {
					let take = stream.by_ref().take(fmd.length);
					let mut reader = claxon::FlacReader::new(take).unwrap();
					samples.append(
						&mut reader
							.samples()
							.map(|x| x.unwrap_or(0) as f32 / 32767.0)
							.collect::<Vec<f32>>(),
					);
				}

				// Sink's thread is detached from main thread, so we need to synchronize with it
				// Why we should synchronize with it?
				// Let's say, that if we don't synchronize with it, we would have
				// a lot (no upper limit, actualy) of buffered sound, waiting for playing in
				// sink
				let sink = SINK.read().unwrap();
				let _md = MD.read().unwrap();
				let md = _md.as_ref().unwrap().clone();
				drop(_md);
				if let Some(sink) = sink.as_ref() {
					while sink.len() >= CACHE_SIZE {
						// Sleeping exactly one buffer and watching for reset signal
						if watching_sleep(
							if sink.len() > 2 {
								sink.len() as f32 - 2.0
							} else {
								0.25
							} * fmd.length as f32 / md.sample_rate as f32
								/ 4.0,
						) {
							_stop();
							return;
						}
					}
					sink.append(SamplesBuffer::new(
						md.channels,
						md.sample_rate,
						samples.as_slice(),
					));
					samples.clear();
				}
			}
		}
	}
}
