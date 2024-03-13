use std::i16;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Local;
use clap::Parser;
use rand::prelude::*;
use rubato::{
	Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
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

	/// Stream in f32le instead of s16le
	#[arg(short, long)]
	float: bool,

	/// Stream in custom sample rate
	#[arg(short, long, default_value = "44100")]
	sample_rate: u32,
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
			.filter(|arg0: &std::path::PathBuf| track_valid(arg0))
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

async fn stream_samples(
	args: &Args,
	track_samples: Vec<f32>,
	rate: u32,
	s: &mut TcpStream,
) -> bool {
	let params = SincInterpolationParameters {
		sinc_len: 64,
		f_cutoff: 0.96,
		interpolation: SincInterpolationType::Quadratic,
		oversampling_factor: 16,
		window: WindowFunction::Blackman,
	};
	let target_rate = args.sample_rate;
	let mut resampler = SincFixedIn::<f32>::new(
		target_rate as f64 / rate as f64,
		100.0,
		params,
		track_samples.len() / 2,
		2,
	)
	.unwrap();
	let start = std::time::Instant::now();
	let (left, right): (Vec<&f32>, Vec<&f32>) =
		(track_samples.iter().step_by(2).collect(), track_samples[1..].iter().step_by(2).collect());
	eprintln!("Splitted channels in {} ms", (std::time::Instant::now() - start).as_millis());
	let samples = if rate != target_rate {
		eprintln!("Resampling {} samples from {} to {}", track_samples.len(), rate, target_rate);
		let start = std::time::Instant::now();
		if rate > target_rate {
			let resampled_l =
				samplerate::convert(rate, target_rate, 1, samplerate::ConverterType::Linear, &left)
					.unwrap();
			let resampled_r = samplerate::convert(
				rate,
				target_rate,
				1,
				samplerate::ConverterType::Linear,
				right.as_slice(),
			)
			.unwrap();
			eprintln!("Resampled in {} ms", (std::time::Instant::now() - start).as_millis());
			vec![resampled_l, resampled_r]
		} else {
			match resampler.process(&[&left, &right], None) {
				Ok(s) => {
					eprintln!(
						"Resampled in {} ms",
						(std::time::Instant::now() - start).as_millis()
					);
					s
				}
				Err(e) => panic!("{}", e),
			}
		}
	} else {
		vec![left, right]
	};
	let (left, right) = (&samples[0], &samples[1]);
	for (sample_l, sample_r) in left.iter().zip(right) {
		if args.float {
			let result = s
				.write(
					&(if args.war {
						sample_l.signum()
					} else {
						*sample_l
					})
					.to_le_bytes(),
				)
				.await;
			match result {
				Err(_) | Ok(0) => return true,
				_ => (),
			};
			let result = s
				.write(
					&(if args.war {
						sample_r.signum()
					} else {
						*sample_r
					})
					.to_le_bytes(),
				)
				.await;
			match result {
				Err(_) | Ok(0) => return true,
				_ => (),
			};
		} else {
			let result = s
				.write(
					&(if args.war {
						sample_l.signum() as i16 * 32767
					} else {
						(sample_l * 32768_f32) as i16
					})
					.to_le_bytes(),
				)
				.await;
			match result {
				Err(_) | Ok(0) => return true,
				_ => (),
			};
			let result = s
				.write(
					&(if args.war {
						sample_r.signum() as i16 * 32767
					} else {
						(sample_r * 32768_f32) as i16
					})
					.to_le_bytes(),
				)
				.await;
			match result {
				Err(_) | Ok(0) => return true,
				_ => (),
			};
		}
	}
	eprintln!("Exiting");
	false
}

async fn stream(mut s: TcpStream, tracklist: Arc<Vec<PathBuf>>) {
	let args = Args::parse();

	loop {
		s.writable().await.unwrap();
		let track = tracklist.choose(&mut thread_rng()).unwrap();

		if args.public_log {
			println!("[{}] {}", Local::now().to_rfc3339(), track.to_str().unwrap(),);
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
		let mut track_samples = vec![];
		let mut track_rate = 0;

		loop {
			let packet = match format.next_packet() {
				Ok(packet) => packet,
				_ => {
					break;
				}
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
					track_rate = rate;
					let mut byte_buf =
						SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
					byte_buf.copy_interleaved_ref(decoded);
					track_samples.append(&mut byte_buf.samples_mut().to_vec());
					continue;
				}
				_ => {
					// Handling any error as track skip
					continue;
				}
			}
		}

		if !track_samples.is_empty() {
			if stream_samples(&args, track_samples, track_rate, &mut s).await {
				return;
			}
		}
	}
}
