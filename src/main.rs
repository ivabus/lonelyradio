mod decode;
mod encode;

use std::collections::HashMap;
use std::io::Cursor;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Local;
use clap::Parser;
use encode::encode;
use futures_util::pin_mut;
use futures_util::StreamExt;
use image::ImageReader;
use lofty::Accessor;
use lofty::TaggedFileExt;
use lonelyradio_types::Encoder;
use lonelyradio_types::Request;
use lonelyradio_types::RequestResult;
use lonelyradio_types::ServerCapabilities;
use lonelyradio_types::Settings;
use lonelyradio_types::{FragmentMetadata, PlayMessage, TrackMetadata};
use rand::prelude::*;
use std::io::Write;
use tokio::net::TcpListener;
use tokio_stream::Stream;
use url::Url;
use walkdir::DirEntry;
use xspf::Playlist;

use crate::decode::decode_file_stream;
use crate::decode::get_meta;

#[derive(Parser)]
struct Args {
	/// Directory with audio files
	dir: PathBuf,

	/// Address:port to bind
	#[arg(short, default_value = "0.0.0.0:5894")]
	address: String,

	/// Resample all tracks, which samplerate exceeds N
	#[arg(short, long, default_value = "96000")]
	max_samplerate: u32,

	/// Disable all audio processing (disable resampling)
	#[arg(long)]
	no_resampling: bool,

	/// Size of artwork (-1 for no artwork, 0 for original, N for NxN)
	#[arg(long, default_value = "96000")]
	artwork: i32,

	#[arg(long)]
	playlist_dir: Option<PathBuf>,
}

const SUPPORTED_ENCODERS: &[Encoder] = &[
	Encoder::Pcm16,
	Encoder::PcmFloat,
	#[cfg(feature = "flac")]
	Encoder::Flac,
	#[cfg(feature = "alac")]
	Encoder::Alac,
	#[cfg(feature = "vorbis")]
	Encoder::Vorbis,
];

async fn stream_track(
	samples_stream: impl Stream<Item = Vec<f32>>,
	md: TrackMetadata,
	mut s: impl Write,
) -> bool {
	pin_mut!(samples_stream);

	let _md = md.clone();

	if s.write_all(rmp_serde::encode::to_vec_named(&PlayMessage::T(_md)).unwrap().as_slice())
		.is_err()
	{
		return true;
	};

	// Why chunks?
	// Different codecs have different quality on different audio lenghts
	while let Some(mut _samples) = samples_stream
		.as_mut()
		.chunks(match md.encoder {
			Encoder::Pcm16 => 1,
			Encoder::PcmFloat => 1,
			Encoder::Flac => 16,
			Encoder::Alac => 32,
			Encoder::Vorbis => 64,
			Encoder::Aac | Encoder::Opus | Encoder::WavPack => unimplemented!(),
		})
		.next()
		.await
	{
		let mut _samples = _samples.concat();

		match md.encoder {
			Encoder::Pcm16 => {
				let _md = PlayMessage::F(FragmentMetadata {
					length: _samples.len() as u64 * 2,
					magic_cookie: None,
				});
				if s.write_all(rmp_serde::to_vec(&_md).unwrap().as_slice()).is_err() {
					return true;
				}
				if s.write_all(
					&encode(Encoder::Pcm16, _samples, md.sample_rate, md.channels).unwrap().0,
				)
				.is_err()
				{
					return true;
				}
			}
			Encoder::PcmFloat => {
				let _md = PlayMessage::F(FragmentMetadata {
					length: _samples.len() as u64 * 4,
					magic_cookie: None,
				});
				if s.write_all(rmp_serde::to_vec(&_md).unwrap().as_slice()).is_err() {
					return true;
				}
				if s.write_all(
					&encode(Encoder::PcmFloat, _samples, md.sample_rate, md.channels).unwrap().0,
				)
				.is_err()
				{
					return true;
				}
			}
			Encoder::Flac | Encoder::Alac | Encoder::Vorbis => {
				let (encoded, magic_cookie) =
					encode(md.encoder, _samples, md.sample_rate, md.channels).unwrap();
				let _md = PlayMessage::F(FragmentMetadata {
					length: encoded.as_slice().len() as u64,
					magic_cookie,
				});
				if s.write_all(rmp_serde::to_vec(&_md).unwrap().as_slice()).is_err() {
					return true;
				}
				if s.write_all(encoded.as_slice()).is_err() {
					return true;
				}
			}
			Encoder::Aac | Encoder::Opus | Encoder::WavPack => unimplemented!(),
		}
	}
	false
}

fn get_playlists(dir: impl AsRef<Path>) -> Option<HashMap<String, Arc<Vec<PathBuf>>>> {
	let mut map: HashMap<String, Arc<Vec<PathBuf>>> = HashMap::new();
	for playlist in walkdir::WalkDir::new(dir)
		.into_iter()
		.filter_entry(is_not_hidden)
		.filter_map(|v| v.ok())
		.map(|x| x.into_path())
		.filter(|x| x.is_file())
	{
		let mut name = playlist.file_name().unwrap().to_str().unwrap().to_string();
		let parsed = Playlist::read_file(playlist).unwrap();
		if let Some(ref n) = parsed.title {
			name = n.clone();
		}
		let tracklist = parsed
			.track_list
			.iter()
			.flat_map(|x| x.location.iter().flat_map(|l| Url::parse(l.as_str()).ok()))
			.filter(|x| x.scheme() == "file")
			.map(|x| x.to_file_path().unwrap())
			.filter(|x| track_valid(x))
			.collect();
		map.insert(name, Arc::new(tracklist));
	}
	Some(map)
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
	let playlists: Option<HashMap<String, Arc<Vec<PathBuf>>>> = match args.playlist_dir.as_ref() {
		None => None,
		Some(dir) => get_playlists(dir),
	};
	loop {
		let (socket, _) = listener.accept().await.unwrap();
		let mut s = socket.into_std().unwrap();
		s.set_nonblocking(false).unwrap();

		let mut hello = [0u8; 8];
		if s.read_exact(&mut hello).is_err() {
			continue;
		}

		if &hello != lonelyradio_types::HELLO_MAGIC {
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
		s.flush().unwrap();

		let request: Request = match rmp_serde::from_read(&s) {
			Ok(r) => r,
			Err(_) => {
				continue;
			}
		};

		match request {
			Request::Play(settings) => {
				if s.write_all(&rmp_serde::to_vec_named(&check_settings(&settings)).unwrap())
					.is_err()
				{
					continue;
				}
				tokio::spawn(stream(s, tracklist.clone(), settings));
			}
			Request::ListPlaylist => match playlists {
				None => {
					s.write_all(
						&rmp_serde::to_vec_named(&RequestResult::Playlist(
							lonelyradio_types::PlaylistResponce {
								playlists: vec![],
							},
						))
						.unwrap(),
					)
					.unwrap();
				}
				Some(ref playlists) => {
					s.write_all(
						&rmp_serde::to_vec_named(&RequestResult::Playlist(
							lonelyradio_types::PlaylistResponce {
								playlists: playlists.keys().cloned().collect(),
							},
						))
						.unwrap(),
					)
					.unwrap();
				}
			},

			Request::PlayPlaylist(playlist, settings) => {
				if playlists.is_none() || playlists.as_ref().unwrap().get(&playlist).is_none() {
					s.write_all(
						&rmp_serde::to_vec_named(&RequestResult::Error(
							lonelyradio_types::RequestError::NoSuchPlaylist,
						))
						.unwrap(),
					)
					.unwrap();
					continue;
				}
				if s.write_all(&rmp_serde::to_vec_named(&check_settings(&settings)).unwrap())
					.is_err()
				{
					continue;
				}
				let tracklist = playlists.as_ref().unwrap().get(&playlist).unwrap().clone();
				tokio::spawn(stream(s, tracklist, settings));
			}
		}
	}
}

fn check_settings(settings: &Settings) -> RequestResult {
	if settings.cover < -1 {
		return RequestResult::Error(lonelyradio_types::RequestError::WrongCoverSize);
	}
	if !SUPPORTED_ENCODERS.contains(&settings.encoder) {
		return RequestResult::Error(lonelyradio_types::RequestError::UnsupportedEncoder);
	}
	RequestResult::Ok
}

fn is_not_hidden(entry: &DirEntry) -> bool {
	entry.file_name().to_str().map(|s| entry.depth() == 0 || !s.starts_with('.')).unwrap_or(false)
}

fn track_valid(track: &Path) -> bool {
	if let Ok(meta) = track.metadata() {
		if !meta.is_file() {
			return false;
		}
	} else {
		return false;
	}
	if let Some(ext) = track.extension() {
		[
			"aac", "mp1", "mp2", "mp3", "wav", "wave", "webm", "mkv", "mp4", "m4a", "m4p", "m4b",
			"m4r", "m4v", "mov", "aiff", "aif", "aifc", "ogg", "ogv", "oga", "ogx", "ogm", "spx",
			"opus", "caf", "flac",
		]
		.contains(&ext.to_str().unwrap())
	} else {
		false
	}
}

async fn stream(mut s: impl Write, tracklist: Arc<Vec<PathBuf>>, settings: Settings) {
	let args = Args::parse();
	let encoder_wants = match settings.encoder {
		Encoder::Opus | Encoder::Vorbis | Encoder::Aac => 48000,
		Encoder::Flac => 96000,
		_ => 0,
	};
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
		println!("[{}] {} ({:?})", Local::now().to_rfc3339(), track_message, settings.encoder);

		let (channels, sample_rate, time) = get_meta(track.as_path(), encoder_wants).await;
		let stream = decode_file_stream(track, encoder_wants);
		let id = thread_rng().gen();
		if stream_track(
			stream,
			TrackMetadata {
				track_length_frac: time.frac as f32,
				track_length_secs: time.seconds,
				encoder: settings.encoder,
				cover: cover.join().unwrap(),
				id,
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
