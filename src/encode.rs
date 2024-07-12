use flacenc::{component::BitRepr, error::Verify, source::MemSource};
use lonelyradio_types::Encoder;

pub fn encode(
	codec: Encoder,
	mut samples: Vec<f32>,
	sample_rate: u32,
	channels: u16,
) -> Option<Vec<u8>> {
	match codec {
		Encoder::Pcm16 => {
			let mut samples = samples.iter_mut().map(|x| (*x * 32768.0) as i16).collect::<Vec<_>>();
			// Launching lonelyradio on the router moment
			if cfg!(target_endian = "big") {
				samples.iter_mut().for_each(|sample| {
					*sample = sample.to_le();
				});
			}
			// Sowwy about that
			let (_, samples, _) = unsafe { samples.align_to::<u8>() };
			Some(samples.to_vec())
		}
		Encoder::PcmFloat => {
			// Launching lonelyradio on the router moment
			// Sowwy about that
			let samples = samples.iter().map(|x| x.to_bits()).collect::<Vec<u32>>();
			let (_, samples, _) = unsafe { samples.align_to::<u8>() };
			Some(samples.to_vec())
		}
		Encoder::Flac => {
			let encoded = flacenc::encode_with_fixed_block_size(
				&flacenc::config::Encoder::default().into_verified().unwrap(),
				MemSource::from_samples(
					// I'm crying (It's just a burning memory)
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
			Some(sink.as_slice().to_vec())
		}
	}
}
