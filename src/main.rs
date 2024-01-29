use chrono::Local;
use clap::Parser;
use rand::prelude::*;
use rand::seq::SliceRandom;
use samplerate::ConverterType;
use std::path::PathBuf;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use walkdir::DirEntry;

#[derive(Parser)]
struct Args {
	dir: PathBuf,
	#[arg(short, default_value = "0.0.0.0:5894")]
	address: String,
}

#[tokio::main]
async fn main() {
	let listener = TcpListener::bind(Args::parse().address).await.unwrap();
	loop {
		let (socket, _) = listener.accept().await.unwrap();
		tokio::spawn(stream(socket));
	}
}
fn is_not_hidden(entry: &DirEntry) -> bool {
	entry.file_name().to_str().map(|s| entry.depth() == 0 || !s.starts_with('.')).unwrap_or(false)
}

// Recursively finding music file
fn pick_track(tracklist: &Vec<PathBuf>) -> &PathBuf {
	let mut track = tracklist.choose(&mut thread_rng()).unwrap();
	while !track.metadata().unwrap().is_file() {
		track = pick_track(tracklist)
	}
	// Skipping "images" (covers)
	while "jpgjpegpngwebp"
		.contains(&track.extension().unwrap().to_str().unwrap().to_ascii_lowercase())
	{
		track = pick_track(tracklist)
	}
	track
}

async fn stream(mut s: TcpStream) {
	let tracklist = walkdir::WalkDir::new(Args::parse().dir)
		.into_iter()
		.filter_entry(is_not_hidden)
		.filter_map(|v| v.ok())
		.map(|x| x.into_path())
		.collect::<Vec<PathBuf>>();
	'track: loop {
		let track = pick_track(&tracklist);
		println!(
			"[{}] {} to {}",
			Local::now().to_rfc3339(),
			track.to_str().unwrap(),
			s.peer_addr().unwrap().port()
		);
		let file = Box::new(std::fs::File::open(track).unwrap());
		let mut hint = Hint::new();
		hint.with_extension(track.extension().unwrap().to_str().unwrap());
		let mss = MediaSourceStream::new(file, Default::default());

		let meta_opts: MetadataOptions = Default::default();
		let fmt_opts: FormatOptions = Default::default();

		let probed = symphonia::default::get_probe()
			.format(&hint, mss, &fmt_opts, &meta_opts)
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

		loop {
			let packet = match format.next_packet() {
				Ok(packet) => packet,
				_ => continue 'track,
			};

			while !format.metadata().is_latest() {
				format.metadata().pop();
			}

			if packet.track_id() != track_id {
				continue;
			}

			match decoder.decode(&packet) {
				Ok(decoded) => {
					let rate = decoded.spec().rate;
					let mut byte_buf =
						SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
					byte_buf.copy_interleaved_ref(decoded);
					let samples = samplerate::convert(
						rate,
						44100,
						2,
						ConverterType::Linear,
						byte_buf.samples(),
					)
					.unwrap();
					for sample in samples {
						let result = s.write(&((sample * 32768_f32) as i16).to_le_bytes()).await;
						if result.is_err() {
							// Socket error -> stop
							return;
						} else {
							if result.unwrap() == 0 {
								// If socket cannot accept data -> stop
								return;
							}
						}
					}
					continue;
				}
				_ => {
					// Handling any error as track skip
					continue 'track;
				}
			}
		}
	}
}
