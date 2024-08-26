//! A library implementing the lonely radio audio streaming protocol
//!
//! Example usage (play for 10 seconds):
//! ```rust
//! extern crate monolib;
//! use std::thread::{sleep, spawn};
//! use std::time::Duration;
//! use monolib::lonelyradio_types::{Settings, Encoder};
//!
//! spawn(|| monolib::run("someserver:someport", Settings {encoder: Encoder::Flac, cover: -1}, "my_playlist"));
//! while monolib::get_metadata().is_none() {}
//! let seconds = md.length / md.sample_rate as u64 / 2;
//! println!("Playing: {} - {} - {} ({}:{:02})", md.artist, md.album, md.title, seconds / 60, seconds % 60);
//! sleep(Duration::from_secs(10));
//! monolib::stop();
//! ```

/// Functions, providing C-like API
pub mod c;

pub use lonelyradio_types;

use anyhow::{bail, Context};
use decode::decode;
use lonelyradio_types::{
	Encoder, PlayMessage, Request, RequestResult, ServerCapabilities, Settings, TrackMetadata,
};
use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use std::io::Write;
use std::net::TcpStream;
use std::sync::atomic::AtomicU8;
use std::sync::RwLock;
use std::time::Instant;

mod decode;

const CACHE_SIZE_PCM: usize = 32;
const CACHE_SIZE_COMPRESSED: usize = 4;

const SUPPORTED_DECODERS: &[Encoder] = &[
	Encoder::Pcm16,
	Encoder::PcmFloat,
	#[cfg(feature = "flac")]
	Encoder::Flac,
	#[cfg(feature = "alac")]
	Encoder::Alac,
	#[cfg(feature = "vorbis")]
	Encoder::Vorbis,
];

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
		sink.pause();
		sink.clear()
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
pub fn get_track(
	server: &str,
	mut settings: Settings,
	playlist: &str,
) -> anyhow::Result<(TrackMetadata, Vec<f32>)> {
	let mut connection = TcpStream::connect(server)?;
	connection.write_all(lonelyradio_types::HELLO_MAGIC)?;
	let capabilities: ServerCapabilities = rmp_serde::from_read(&mut connection)?;
	if !capabilities.encoders.contains(&settings.encoder) {
		settings.encoder = Encoder::Pcm16
	}

	let request = if playlist.is_empty() {
		Request::Play(settings)
	} else {
		Request::PlayPlaylist(playlist.to_string(), settings)
	};
	connection.write_all(&rmp_serde::to_vec_named(&request).unwrap())?;

	let response: RequestResult = rmp_serde::from_read(&connection)?;
	if let RequestResult::Error(e) = response {
		bail!("{e:?}")
	}

	let mut samples = vec![];
	let mut md: Option<TrackMetadata> = None;

	loop {
		let recv_md: PlayMessage = rmp_serde::from_read(&mut connection)?;
		match recv_md {
			PlayMessage::T(tmd) => {
				if md.is_some() {
					break;
				}
				md = Some(tmd);
			}
			PlayMessage::F(fmd) => {
				samples.extend(decode(&mut connection, md.as_ref().unwrap(), &fmd)?)
			}
		}
	}

	if let Some(md) = md {
		Ok((md, samples))
	} else {
		bail!("No metadata")
	}
}

pub fn list_playlists(server: &str) -> Option<Vec<String>> {
	let mut connection = TcpStream::connect(server).ok()?;
	connection.write_all(lonelyradio_types::HELLO_MAGIC).ok()?;
	let _: ServerCapabilities = rmp_serde::from_read(&mut connection).ok()?;
	connection.write_all(&rmp_serde::to_vec_named(&Request::ListPlaylist).ok()?).ok()?;
	let res: RequestResult = rmp_serde::from_read(connection).ok()?;
	match res {
		RequestResult::Playlist(plist) => Some(plist.playlists),
		_ => None,
	}
}

/// Starts playing at "server:port"
pub fn run(server: &str, settings: Settings, playlist: &str) {
	let result = _run(server, settings, playlist);
	if let Err(e) = result {
		println!("{:?}", e);
		*STATE.write().unwrap() = State::NotStarted;
	}
}

pub(crate) fn _run(server: &str, mut settings: Settings, playlist: &str) -> anyhow::Result<()> {
	if !SUPPORTED_DECODERS.contains(&settings.encoder) {
		eprintln!(
			"monolib was built without support for {:?}, falling back to Pcm16",
			settings.encoder
		);
		settings.encoder = Encoder::Pcm16
	}
	let mut state = STATE.write().unwrap();
	if *state == State::Playing || *state == State::Paused {
		return Ok(());
	}
	*state = State::Playing;
	drop(state);

	let mut connection = TcpStream::connect(server).context("failed to connect to the server")?;
	connection.write_all(lonelyradio_types::HELLO_MAGIC)?;
	let capabilities: ServerCapabilities = rmp_serde::from_read(&mut connection)?;
	if !capabilities.encoders.contains(&settings.encoder) {
		settings.encoder = Encoder::Pcm16
	}

	let request = if playlist.is_empty() {
		Request::Play(settings)
	} else {
		Request::PlayPlaylist(playlist.to_string(), settings)
	};
	connection.write_all(&rmp_serde::to_vec_named(&request).unwrap())?;

	let response: RequestResult = rmp_serde::from_read(&connection).unwrap();
	if let RequestResult::Error(e) = response {
		bail!("{:?}", e)
	}
	let mut stream = connection;

	let mut sink = SINK.write().unwrap();
	let (_stream, stream_handle) =
		OutputStream::try_default().context("failed to determine audio device")?;

	// Can't reuse old sink for some reason
	let audio_sink = Sink::try_new(&stream_handle).context("failed to create audio sink")?;
	*sink = Some(audio_sink);
	drop(sink);

	let mut samples = Vec::with_capacity(8192);
	loop {
		let recv_md: PlayMessage =
			rmp_serde::from_read(&mut stream).expect("Failed to parse message");
		match recv_md {
			PlayMessage::T(tmd) => {
				// No metadata shift
				if watching_sleep_until_end() {
					_stop();
					return Ok(());
				}
				let mut md = MD.write().unwrap();
				*md = Some(tmd.clone());

				drop(md);
			}
			PlayMessage::F(fmd) => {
				while *STATE.read().unwrap() == State::Paused {
					std::thread::sleep(std::time::Duration::from_secs_f32(0.25))
				}
				if *STATE.read().unwrap() == State::Resetting {
					_stop();
					return Ok(());
				}

				samples.extend(decode(&mut stream, &MD.read().unwrap().clone().unwrap(), &fmd)?);

				// Synchronizing with sink
				let sink = SINK.read().unwrap();
				let _md = MD.read().unwrap();
				let md = _md.as_ref().unwrap().clone();
				drop(_md);
				if let Some(sink) = sink.as_ref() {
					while (sink.len() >= CACHE_SIZE_PCM
						&& md.encoder == Encoder::Pcm16
						&& md.encoder == Encoder::PcmFloat)
						|| (sink.len() >= CACHE_SIZE_COMPRESSED
							&& md.encoder != Encoder::Pcm16
							&& md.encoder != Encoder::PcmFloat)
					{
						// Sleeping exactly one buffer and watching for reset signal
						if watching_sleep(
							if sink.len() > 2 {
								sink.len() as f32 - 2.0
							} else {
								0.25
							} * samples.len() as f32 / md.sample_rate as f32
								/ 4.0,
						) {
							_stop();
							return Ok(());
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
