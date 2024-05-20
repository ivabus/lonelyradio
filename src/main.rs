mod decode;
mod writer;

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Local;
use clap::Parser;
use flacenc::component::BitRepr;
use flacenc::error::Verify;
use flacenc::source::MemSource;
use futures_util::pin_mut;
use futures_util::StreamExt;
use lofty::Accessor;
use lofty::TaggedFileExt;
use lonelyradio_types::{FragmentMetadata, Message, TrackMetadata};
use once_cell::sync::Lazy;
use rand::prelude::*;
use std::io::Write;
use tokio::net::TcpListener;
use tokio_stream::Stream;
use walkdir::DirEntry;
use writer::Writer;

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

	/// Use FLAC compression
	#[arg(short, long)]
	flac: bool,

	/// Enable XOR "encryption"
	#[arg(long)]
	xor_key_file: Option<PathBuf>,
}

static KEY: Lazy<Option<Arc<Vec<u8>>>> = Lazy::new(|| {
	let args = Args::parse();
	if let Some(path) = args.xor_key_file {
		let key = std::fs::read(path).expect("Failed to read preshared key");
		Some(Arc::new(key))
	} else {
		None
	}
});

async fn stream_track(
	samples_stream: impl Stream<Item = Vec<i16>>,
	war: bool,
	md: TrackMetadata,
	s: &mut Writer,
) -> bool {
	pin_mut!(samples_stream);

	let _md = md.clone();

	if s.write_all(rmp_serde::to_vec(&Message::T(_md)).unwrap().as_slice()).is_err() {
		return true;
	};

	// Why chunks?
	// flacenc is broken on low amount of samples (Symphonia's AIFF decoder returns ~2304
	// samples per packet (on bo en's tracks), instead of usual ~8192 on any other lossless decoder)
	while let Some(mut _samples) = samples_stream
		.as_mut()
		.chunks(if md.flac && md.track_length_secs > 1 {
			2
		} else {
			1
		})
		.next()
		.await
	{
		let mut _samples = _samples.concat();
		if war {
			_samples.iter_mut().for_each(|sample| {
				*sample = sample.signum() * 32767;
			});
		}
		if !md.flac {
			let _md = Message::F(FragmentMetadata {
				length: _samples.len() as u64,
			});
			if s.write_all(rmp_serde::to_vec(&_md).unwrap().as_slice()).is_err() {
				return true;
			}

			// Launching lonelyradio on the router moment
			if cfg!(target_endian = "big") {
				_samples.iter_mut().for_each(|sample| {
					*sample = sample.to_le();
				});
			}

			// Sowwy about that
			let (_, samples, _) = unsafe { _samples.align_to::<u8>() };

			if s.write_all(samples).is_err() {
				return true;
			}
		} else {
			let encoded = flacenc::encode_with_fixed_block_size(
				&flacenc::config::Encoder::default().into_verified().unwrap(),
				MemSource::from_samples(
					// I'm crying (It's just a burning memory)
					&_samples.iter().map(|x| *x as i32).collect::<Vec<i32>>(),
					md.channels as usize,
					16,
					md.sample_rate as usize,
				),
				256,
			);
			if encoded.is_err() {
				return true;
			}

			let mut sink = flacenc::bitsink::ByteSink::new();
			encoded.unwrap().write(&mut sink).unwrap();

			let _md = Message::F(FragmentMetadata {
				length: sink.as_slice().len() as u64,
			});
			if s.write_all(rmp_serde::to_vec(&_md).unwrap().as_slice()).is_err() {
				return true;
			}
			if s.write_all(sink.as_slice()).is_err() {
				return true;
			}
		}
	}
	false
}

#[tokio::main]
async fn main() {
	let args = Args::parse();
	let listener = TcpListener::bind(Args::parse().address).await.unwrap();
	let tracklist = Arc::new(
		walkdir::WalkDir::new(Args::parse().dir)
			.into_iter()
			.filter_entry(is_not_hidden)
			.filter_map(|v| v.ok())
			.map(|x| x.into_path())
			.filter(|x| track_valid(x))
			.collect::<Vec<PathBuf>>(),
	);
	loop {
		let (socket, _) = listener.accept().await.unwrap();
		let s = socket.into_std().unwrap();
		s.set_nonblocking(false).unwrap();
		let s = if args.xor_key_file.is_some() {
			Writer::XorEncrypted(
				s,
				match &*KEY {
					Some(a) => a.clone(),
					_ => {
						unreachable!()
					}
				},
				0,
			)
		} else {
			Writer::Unencrypted(s)
		};
		tokio::spawn(stream(s, tracklist.clone()));
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

async fn stream(mut s: Writer, tracklist: Arc<Vec<PathBuf>>) {
	let args = Args::parse();

	loop {
		let track = tracklist.choose(&mut thread_rng()).unwrap().clone();

		let mut title = String::new();
		let mut artist = String::new();
		let mut album = String::new();
		let mut file = std::fs::File::open(&track).unwrap();
		let tagged = match lofty::read_from(&mut file) {
			Ok(f) => f,
			_ => continue,
		};
		if let Some(id3v2) = tagged.primary_tag() {
			title =
				id3v2.title().unwrap_or(track.file_stem().unwrap().to_string_lossy()).to_string();
			album = id3v2.album().unwrap_or("[No tag]".into()).to_string();
			artist = id3v2.artist().unwrap_or("[No tag]".into()).to_string();
		};
		let track_message = format!("{} - {} - {}", &artist, &album, &title);
		eprintln!(
			"[{}] {} to {}:{}{}",
			Local::now().to_rfc3339(),
			track_message,
			s.peer_addr().unwrap().ip(),
			s.peer_addr().unwrap().port(),
			if args.war {
				" with WAR.rs"
			} else {
				""
			}
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
				flac: args.flac,
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
