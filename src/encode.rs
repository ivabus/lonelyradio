use lonelyradio_types::Encoder;

// Return: 0 - encoded bytes, 1 - magic cookie (for alac only)
#[allow(unused_variables)]
pub fn encode(
	codec: Encoder,
	mut samples: Vec<f32>,
	sample_rate: u32,
	channels: u16,
) -> Option<(Vec<u8>, Option<Vec<u8>>)> {
	match codec {
		Encoder::Pcm16 => {
			#[allow(unused_mut)]
			let mut samples = samples.iter_mut().map(|x| (*x * 32768.0) as i16).collect::<Vec<_>>();
			// Launching lonelyradio on the router moment
			#[cfg(target_endian = "big")]
			{
				samples.iter_mut().for_each(|sample| {
					*sample = sample.to_le();
				});
			}
			// Sowwy about that
			let (_, samples, _) = unsafe { samples.align_to::<u8>() };
			Some((samples.to_vec(), None))
		}
		Encoder::PcmFloat => {
			// Sowwy about that
			let (_, samples, _) = unsafe { samples.align_to::<u8>() };
			Some((samples.to_vec(), None))
		}
		Encoder::Flac => {
			#[cfg(feature = "flac")]
			{
				use flacenc::{component::BitRepr, error::Verify, source::MemSource};

				let encoded = flacenc::encode_with_fixed_block_size(
					&flacenc::config::Encoder::default().into_verified().unwrap(),
					MemSource::from_samples(
						&samples
							.iter()
							.map(|x| (*x as f64 * 32768.0 * 256.0) as i32)
							.collect::<Vec<i32>>(),
						channels as usize,
						24,
						sample_rate as usize,
					),
					256,
				)
				.unwrap();

				let mut sink = flacenc::bitsink::ByteSink::new();
				encoded.write(&mut sink).unwrap();
				Some((sink.as_slice().to_vec(), None))
			}

			#[cfg(not(feature = "flac"))]
			{
				unimplemented!()
			}
		}
		Encoder::Alac => {
			#[cfg(feature = "alac")]
			{
				use alac_encoder::{AlacEncoder, FormatDescription};

				let samples = samples.iter_mut().map(|x| (*x * 32768.0) as i16).collect::<Vec<_>>();
				let (_, samples, _) = unsafe { samples.align_to::<u8>() };

				let input_format =
					FormatDescription::pcm::<i16>(sample_rate as f64, channels as u32);
				let output_format = FormatDescription::alac(
					sample_rate as f64,
					samples.len() as u32,
					channels as u32,
				);

				// Initialize the encoder
				let mut encoder = AlacEncoder::new(&output_format);

				// Allocate a buffer for the encoder to write chunks to.
				let mut output = vec![0u8; output_format.max_packet_size()];
				let size = encoder.encode(&input_format, samples, &mut output);

				// Here you can do whatever you want with the result:
				Some((Vec::from(&output[0..size]), Some(encoder.magic_cookie())))
			}
			#[cfg(not(feature = "alac"))]
			{
				unimplemented!()
			}
		}
		Encoder::Vorbis => {
			#[cfg(feature = "vorbis")]
			{
				use std::num::{NonZeroU32, NonZeroU8};
				let out: Vec<u8> = vec![];
				let mut encoder = vorbis_rs::VorbisEncoderBuilder::new(
					NonZeroU32::new(sample_rate).unwrap(),
					NonZeroU8::new(channels as u8).unwrap(),
					out,
				)
				.unwrap()
				.bitrate_management_strategy(
					vorbis_rs::VorbisBitrateManagementStrategy::ConstrainedAbr {
						// I will think about clients asking about bitrate later, now it's just
						// "enough" 128 kib/s
						maximum_bitrate: NonZeroU32::new(192 * 1024).unwrap(),
					},
				)
				.build()
				.unwrap();
				let mut samples_channels = vec![];
				for i in 0..channels as usize {
					samples_channels.push(
						samples[i..]
							.iter()
							.step_by(channels as usize)
							.copied()
							.collect::<Vec<f32>>(),
					);
				}
				encoder.encode_audio_block(samples_channels).unwrap();
				Some((encoder.finish().unwrap(), None))
			}

			#[cfg(not(feature = "vorbis"))]
			{
				unimplemented!()
			}
		}
		Encoder::Aac | Encoder::Opus | Encoder::WavPack => unimplemented!(),
	}
}
