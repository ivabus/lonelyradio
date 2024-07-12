mod decode;
mod encode;

use std::io::Cursor;
use std::io::Read;
use std::net::TcpStream;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Local;
use clap::Parser;
use encode::encode;
use futures_util::pin_mut;
use futures_util::StreamExt;
use image::io::Reader as ImageReader;
use lofty::Accessor;
use lofty::TaggedFileExt;
use lonelyradio_types::Encoder;
use lonelyradio_types::ServerCapabilities;
use lonelyradio_types::Settings;
use lonelyradio_types::{FragmentMetadata, Message, TrackMetadata};
use rand::prelude::*;
use std::io::Write;
use tokio::net::TcpListener;
use tokio_stream::Stream;
use walkdir::DirEntry;

use crate::decode::decode_file_stream;
use crate::decode::get_meta;

#[derive(Parser)]
struct Args {
	/// Directory with audio files
	dir: PathBuf,

	/// Address:port to bind
	#[arg(short, default_value = "0.0.0.0:5894")]
	address: String,

	/// Enable "public" log (without sensitive information)
	#[arg(short, long)]
	public_log: bool,

	/// Process all samples to -1 or 1
	#[arg(short, long)]
	war: bool,

	/// Resample all tracks, which samplerate exceeds N
	#[arg(short, long, default_value = "96000")]
	max_samplerate: u32,

	/// Disable all audio processing (disable resampling)
	#[arg(long)]
	no_resampling: bool,

	/// Size of artwork (-1 for no artwork, 0 for original, N for NxN)
	#[arg(long, default_value = "96000")]
	artwork: i32,
}

const SUPPORTED_ENCODERS: [Encoder; 3] = [Encoder::Pcm16, Encoder::PcmFloat, Encoder::Flac];

async fn stream_track(
	samples_stream: impl Stream<Item = Vec<f32>>,
	war: bool,
	md: TrackMetadata,
	s: &mut TcpStream,
) -> bool {
	pin_mut!(samples_stream);

	let _md = md.clone();

	if s.write_all(rmp_serde::encode::to_vec_named(&Message::T(_md)).unwrap().as_slice()).is_err() {
		return true;
	};

	// Why chunks?
	// flacenc is broken on low amount of samples (Symphonia's AIFF decoder returns
	// ~2304 samples per packet (on bo en's tracks), instead of usual ~8192 on any
	// other lossless decoder)
	while let Some(mut _samples) = samples_stream
		.as_mut()
		.chunks(match md.encoder {
			Encoder::Pcm16 => 1,
			Encoder::PcmFloat => 1,
			Encoder::Flac => 16,
		})
		.next()
		.await
	{
		let mut _samples = _samples.concat();
		if war {
			_samples.iter_mut().for_each(|sample| {
				*sample = sample.signum();
			});
		}
		match md.encoder {
			Encoder::Pcm16 => {
				let _md = Message::F(FragmentMetadata {
					length: _samples.len() as u64 * 2,
				});
				if s.write_all(rmp_serde::to_vec(&_md).unwrap().as_slice()).is_err() {
					return true;
				}
				if s.write_all(
					&encode(Encoder::Pcm16, _samples, md.sample_rate, md.channels).unwrap(),
				)
				.is_err()
				{
					return true;
				}
			}
			Encoder::PcmFloat => {
				let _md = Message::F(FragmentMetadata {
					length: _samples.len() as u64 * 4,
				});
				if s.write_all(rmp_serde::to_vec(&_md).unwrap().as_slice()).is_err() {
					return true;
				}
				if s.write_all(
					&encode(Encoder::PcmFloat, _samples, md.sample_rate, md.channels).unwrap(),
				)
				.is_err()
				{
					return true;
				}
			}
			Encoder::Flac => {
				let encoded = encode(Encoder::Flac, _samples, md.sample_rate, md.channels).unwrap();
				let _md = Message::F(FragmentMetadata {
					length: encoded.as_slice().len() as u64,
				});
				if s.write_all(rmp_serde::to_vec(&_md).unwrap().as_slice()).is_err() {
					return true;
				}
				if s.write_all(encoded.as_slice()).is_err() {
					return true;
				}
			}
		}
	}
	false
}

#[tokio::main]
async fn main() {
	let args = Args::parse();
	let listener = TcpListener::bind(args.address).await.unwrap();
	let tracklist = Arc::new(
		walkdir::WalkDir::new(args.dir)
			.into_iter()
			.filter_entry(is_not_hidden)
			.filter_map(|v| v.ok())
			.map(|x| x.into_path())
			.filter(|x| track_valid(x))
			.collect::<Vec<PathBuf>>(),
	);
	loop {
		let (socket, _) = listener.accept().await.unwrap();
		let mut s = socket.into_std().unwrap();
		s.set_nonblocking(false).unwrap();
		let mut hello = [0u8; 8];
		if s.read_exact(&mut hello).is_err() {
			continue;
		}
		if hello != lonelyradio_types::HELLO_MAGIC.to_le_bytes() {
			continue;
		}
		if s.write_all(
			&rmp_serde::to_vec_named(&ServerCapabilities {
				encoders: SUPPORTED_ENCODERS.to_vec(),
			})
			.unwrap(),
		)
		.is_err()
		{
			continue;
		};
		let settings: Settings = match rmp_serde::from_read(&s) {
			Ok(s) => s,
			_ => continue,
		};
		if settings.cover < -1 {
			continue;
		}
		tokio::spawn(stream(s, tracklist.clone(), settings));
	}
}

fn is_not_hidden(entry: &DirEntry) -> bool {
	entry.file_name().to_str().map(|s| entry.depth() == 0 || !s.starts_with('.')).unwrap_or(false)
}

fn track_valid(track: &Path) -> bool {
	if !track.metadata().unwrap().is_file() {
		return false;
	}
	// Skipping "images" (covers)
	if "jpgjpegpngwebp".contains(&track.extension().unwrap().to_str().unwrap().to_ascii_lowercase())
	{
		return false;
	}
	true
}

async fn stream(mut s: TcpStream, tracklist: Arc<Vec<PathBuf>>, settings: Settings) {
	let args = Args::parse();

	loop {
		let track = tracklist.choose(&mut thread_rng()).unwrap().clone();

		let mut title = String::new();
		let mut artist = String::new();
		let mut album = String::new();
		let mut cover = std::thread::spawn(|| None);
		let mut file = std::fs::File::open(&track).unwrap();
		let tagged = match lofty::read_from(&mut file) {
			Ok(f) => f,
			_ => continue,
		};
		if let Some(id3v2) = tagged.primary_tag() {
			title =
				id3v2.title().unwrap_or(track.file_stem().unwrap().to_string_lossy()).to_string();
			album = id3v2.album().unwrap_or("".into()).to_string();
			artist = id3v2.artist().unwrap_or("".into()).to_string();
			if !(id3v2.pictures().is_empty() || args.artwork == -1 || settings.cover == -1) {
				let pic = id3v2.pictures()[0].clone();
				cover = std::thread::spawn(move || {
					let dec = ImageReader::new(Cursor::new(pic.into_data()))
						.with_guessed_format()
						.ok()?
						.decode()
						.ok()?;
					let mut img = Vec::new();
					if args.artwork != 0 && settings.cover != 0 {
						let size = std::cmp::min(args.artwork as u32, settings.cover as u32);
						dec.resize(size, size, image::imageops::FilterType::Lanczos3)
					} else {
						dec
					}
					.to_rgb8()
					.write_to(&mut Cursor::new(&mut img), image::ImageFormat::Jpeg)
					.unwrap();
					Some(img)
				});
			};
		};
		let track_message = format!("{} - {} - {}", &artist, &album, &title);
		eprintln!(
			"[{}] {} to {}:{}{} ({:?})",
			Local::now().to_rfc3339(),
			track_message,
			s.peer_addr().unwrap().ip(),
			s.peer_addr().unwrap().port(),
			if args.war {
				" with WAR.rs"
			} else {
				""
			},
			settings.encoder
		);

		if args.public_log {
			println!(
				"[{}] {} to {}{}",
				Local::now().to_rfc3339(),
				track.to_str().unwrap(),
				s.peer_addr().unwrap().port(),
				if args.war {
					" with WAR.rs"
				} else {
					""
				}
			);
		}
		let (channels, sample_rate, time) = get_meta(track.as_path()).await;
		let stream = decode_file_stream(track);
		if stream_track(
			stream,
			args.war,
			TrackMetadata {
				track_length_frac: time.frac as f32,
				track_length_secs: time.seconds,
				encoder: settings.encoder,
				cover: cover.join().unwrap(),
				album,
				artist,
				title,
				sample_rate,
				channels,
			},
			&mut s,
		)
		.await
		{
			return;
		};
	}
}
