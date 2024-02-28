use std::path::PathBuf;
use std::sync::Arc;

use chrono::Local;
use clap::Parser;
use rand::prelude::*;
use samplerate::ConverterType;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::io::MediaSourceStream;
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

#[tokio::main]
async fn main() {
	let listener = TcpListener::bind(Args::parse().address).await.unwrap();
	let tracklist = Arc::new(
		walkdir::WalkDir::new(Args::parse().dir)
			.into_iter()
			.filter_entry(is_not_hidden)
			.filter_map(|v| v.ok())
			.map(|x| x.into_path())
			.filter(track_valid)
			.into_iter()
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

fn track_valid(track: &PathBuf) -> bool {
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

	'track: loop {
		let track = tracklist.choose(&mut thread_rng()).unwrap();
		eprintln!(
			"[{}] {} to {}:{}{}",
			Local::now().to_rfc3339(),
			track.to_str().unwrap(),
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
					let samples = if rate != 44100 {
						samplerate::convert(
							rate,
							44100,
							2,
							ConverterType::Linear,
							byte_buf.samples(),
						)
						.unwrap()
					} else {
						byte_buf.samples().to_vec()
					};
					for sample in samples {
						let result = s
							.write(
								&(if args.war {
									sample.signum() as i16 * 32767
								} else {
									(sample * 32768_f32) as i16
								})
								.to_le_bytes(),
							)
							.await;
						match result {
							Err(_) | Ok(0) => {
								return;
							}
							_ => (),
						};
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
