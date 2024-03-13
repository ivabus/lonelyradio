use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Local;
use clap::Parser;
use lofty::Accessor;
use lofty::TaggedFileExt;
use rand::prelude::*;
use samplerate::ConverterType;
use serde::Serialize;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::StandardTagKey;
use symphonia::core::probe::Hint;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use walkdir::DirEntry;

#[derive(Parser)]
struct Args {
	dir: PathBuf,
	#[arg(short, default_value = "0.0.0.0:5894")]
	address: String,

	#[arg(short, long)]
	public_log: bool,

	#[arg(short, long)]
	war: bool,
}

#[derive(Serialize)]
struct SentMetadata {
	// In bytes, we need to read next track metadata
	lenght: u64,
	title: String,
	album: String,
	artist: String,
}

async fn stream_samples(
	track_samples: Vec<f32>,
	rate: u32,
	war: bool,
	md: SentMetadata,
	s: &mut TcpStream,
) -> bool {
	let resampled = if rate != 44100 {
		samplerate::convert(rate, 44100, 2, ConverterType::Linear, track_samples.as_slice())
			.unwrap()
	} else {
		track_samples
	};
	let mut md = md;
	md.lenght = resampled.len() as u64;
	if s.write_all(rmp_serde::to_vec(&md).unwrap().as_slice()).await.is_err() {
		return true;
	}
	for sample in resampled {
		if s.write_all(
			&(if war {
				sample.signum() as i16 * 32767
			} else {
				(sample * 32768_f32) as i16
			}
			.to_le_bytes()),
		)
		.await
		.is_err()
		{
			return true;
		};
	}
	false
}

#[tokio::main]
async fn main() {
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
		tokio::spawn(stream(socket, tracklist.clone()));
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

async fn stream(mut s: TcpStream, tracklist: Arc<Vec<PathBuf>>) {
	let args = Args::parse();

	loop {
		let track = tracklist.choose(&mut thread_rng()).unwrap();

		let mut title = String::new();
		let mut artist = String::new();
		let mut album = String::new();
		let mut file = std::fs::File::open(track).unwrap();
		let tagged = lofty::read_from(&mut file).unwrap();
		if let Some(id3v2) = tagged.primary_tag() {
			title = id3v2.title().unwrap_or("[No tag]".into()).to_string();
			album = id3v2.album().unwrap_or("[No tag]".into()).to_string();
			artist = id3v2.artist().unwrap_or("[No tag]".into()).to_string()
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

		let file = Box::new(std::fs::File::open(track).unwrap());
		let mut hint = Hint::new();
		hint.with_extension(track.extension().unwrap().to_str().unwrap());

		let probed = symphonia::default::get_probe()
			.format(
				&hint,
				MediaSourceStream::new(file, Default::default()),
				&Default::default(),
				&Default::default(),
			)
			.expect("unsupported format");

		let mut format = probed.format;

		let track = format
			.tracks()
			.iter()
			.find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
			.expect("no supported audio tracks");

		let mut decoder = symphonia::default::get_codecs()
			.make(&track.codec_params, &Default::default())
			.expect("unsupported codec");

		let track_id = track.id;
		let mut track_rate = 0;
		let mut samples = vec![];
		loop {
			let packet = match format.next_packet() {
				Ok(packet) => packet,
				_ => break,
			};
			while !format.metadata().is_latest() {
				format.metadata().pop();
				if let Some(rev) = format.metadata().current() {
					for tag in rev.tags() {
						println!("Looped");
						match tag.std_key {
							Some(StandardTagKey::Album) => album = tag.value.to_string(),
							Some(StandardTagKey::Artist) => artist = tag.value.to_string(),
							Some(StandardTagKey::TrackTitle) => title = tag.value.to_string(),
							_ => {}
						}
						eprintln!("DBG: {} {} {}", &album, &artist, &title)
					}
				}
			}

			if packet.track_id() != track_id {
				continue;
			}

			match decoder.decode(&packet) {
				Ok(decoded) => {
					track_rate = decoded.spec().rate;
					let mut byte_buf =
						SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
					byte_buf.copy_interleaved_ref(decoded);
					samples.append(&mut byte_buf.samples_mut().to_vec());
					continue;
				}
				_ => {
					// Handling any error as track skip
					continue;
				}
			}
		}
		if !samples.is_empty() {
			if stream_samples(
				samples,
				track_rate,
				args.war,
				SentMetadata {
					lenght: 0,
					title,
					album,
					artist,
				},
				&mut s,
			)
			.await
			{
				break;
			}
		}
	}
}
