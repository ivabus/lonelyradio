use std::path::{Path, PathBuf};

use async_stream::stream;
use clap::Parser;
use futures_util::Stream;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

use crate::Args;

pub async fn get_meta(file_path: &Path, encoder_wants: u32) -> (u16, u32, Time) {
	let file = Box::new(std::fs::File::open(file_path).unwrap());
	let mut hint = Hint::new();
	hint.with_extension(file_path.extension().unwrap().to_str().unwrap());

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
	let mut channels = 2u16;
	let mut sample_rate = 0;
	let track_length =
		track.codec_params.time_base.unwrap().calc_time(track.codec_params.n_frames.unwrap());
	loop {
		let packet = match format.next_packet() {
			Ok(packet) => packet,
			_ => break,
		};

		if packet.track_id() != track_id {
			continue;
		}

		match decoder.decode(&packet) {
			Ok(decoded) => {
				channels = decoded.spec().channels.count().try_into().unwrap();
				sample_rate = decoded.spec().rate;
				break;
			}
			_ => {
				// Handling any error as track skip
				continue;
			}
		}
	}
	let args = Args::parse();

	(
		channels,
		if args.no_resampling && encoder_wants == 0 {
			sample_rate
		} else {
			get_resampling_rate(
				&sample_rate,
				&if encoder_wants != 0 {
					args.max_samplerate.min(encoder_wants)
				} else {
					args.max_samplerate
				},
			)
		},
		track_length,
	)
}

/// Getting samples
pub fn decode_file_stream(file_path: PathBuf, encoder_wants: u32) -> impl Stream<Item = Vec<f32>> {
	let args = Args::parse();
	let file = Box::new(std::fs::File::open(&file_path).unwrap());
	let mut hint = Hint::new();
	hint.with_extension(file_path.extension().unwrap().to_str().unwrap());

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
	stream! {
		loop {
			let packet = match format.next_packet() {
				Ok(packet) => packet,
				_ => break,
			};

			if packet.track_id() != track_id {
				continue;
			}

			match decoder.decode(&packet) {
				Ok(decoded) => {
					let output_rate = get_resampling_rate(&decoded.spec().rate, &if encoder_wants != 0 {
						args.max_samplerate.min(encoder_wants)
					} else {
						args.max_samplerate
					});
					if decoded.spec().rate > output_rate && (!args.no_resampling || encoder_wants != 0) {
						let spec = *decoded.spec();
						let mut byte_buf =
							SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
						byte_buf.copy_interleaved_ref(decoded);
						// About Samplerate struct:
						// We are downsampling, not upsampling, so we should be fine
						yield (if output_rate == spec.rate {
							byte_buf.samples().to_vec()
						} else {
							samplerate::convert(
								spec.rate,
								output_rate,
								spec.channels.count(),
								samplerate::ConverterType::Linear,
								byte_buf.samples(),
							)
							.unwrap()
						});
					} else {
						let mut byte_buf =
							SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
						byte_buf.copy_interleaved_ref(decoded);
						yield byte_buf.samples().to_vec();
					}
					continue;
				}
				_ => {
					// Handling any error as track skip
					continue;
				}
			}
		}
	}
}

fn get_resampling_rate(in_rate: &u32, max_samplerate: &u32) -> u32 {
	if in_rate < max_samplerate {
		*in_rate
	} else if in_rate % 44100 == 0 {
		max_samplerate - (max_samplerate % 44100)
	} else if in_rate % 48000 == 0 {
		max_samplerate - (max_samplerate % 48000)
	} else {
		*max_samplerate
	}
}
