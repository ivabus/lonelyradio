use std::io::{Cursor, Read};

use byteorder::{LittleEndian, ReadBytesExt};
use lonelyradio_types::{Encoder, FragmentMetadata, TrackMetadata};
use symphonia_core::{
	audio::SampleBuffer,
	codecs::{Decoder, CODEC_TYPE_ALAC},
	formats::Packet,
};

pub(crate) fn decode(
	mut stream: impl ReadBytesExt,
	md: &TrackMetadata,
	fmd: &FragmentMetadata,
) -> anyhow::Result<Vec<f32>> {
	let mut samples = vec![];
	match md.encoder {
		Encoder::Pcm16 => {
			let mut samples_i16 = vec![0; fmd.length as usize / 2];
			stream.read_i16_into::<LittleEndian>(&mut samples_i16)?;
			samples
				.append(&mut samples_i16.iter().map(|sample| *sample as f32 / 32767.0).collect());
		}
		Encoder::PcmFloat => {
			let mut samples_f32 = vec![0f32; fmd.length as usize / 4];
			stream.read_f32_into::<LittleEndian>(&mut samples_f32)?;
			samples.append(&mut samples_f32);
		}
		Encoder::Flac => {
			#[cfg(feature = "alac")]
			{
				let take = std::io::Read::by_ref(&mut stream).take(fmd.length);
				let mut reader = claxon::FlacReader::new(take)?;
				samples.append(
					&mut reader
						.samples()
						.map(|x| x.unwrap_or(0) as f32 / 32768.0 / 256.0)
						.collect::<Vec<f32>>(),
				);
			}

			#[cfg(not(feature = "flac"))]
			{
				unimplemented!("flac decoding is disabled in library")
			}
		}
		Encoder::Alac => {
			#[cfg(feature = "alac")]
			{
				let mut buf = vec![];
				std::io::Read::by_ref(&mut stream).take(fmd.length).read_to_end(&mut buf)?;
				let mut reader = symphonia_codec_alac::AlacDecoder::try_new(
					symphonia_core::codecs::CodecParameters::default()
						.for_codec(CODEC_TYPE_ALAC)
						.with_extra_data(fmd.magic_cookie.clone().unwrap().into_boxed_slice()),
					&symphonia_core::codecs::DecoderOptions {
						verify: false,
					},
				)?;
				let decoded = reader.decode(&Packet::new_from_slice(0, 0, 0, &buf))?;
				let mut byte_buf =
					SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
				byte_buf.copy_interleaved_ref(decoded);
				samples.extend(byte_buf.samples());
			}
			#[cfg(not(feature = "alac"))]
			{
				unimplemented!("alac decoding is disabled in library")
			}
		}
		Encoder::Vorbis => {
			#[cfg(feature = "vorbis")]
			{
				let mut buf = vec![];
				std::io::Read::by_ref(&mut stream).take(fmd.length).read_to_end(&mut buf)?;
				let mut decoder = vorbis_rs::VorbisDecoder::new(Cursor::new(buf))?;
				let mut interleaved = vec![];

				while let Some(decoded_block) = decoder.decode_audio_block()? {
					let s = decoded_block.samples();
					interleaved.resize(s[0].len() * s.len(), 0f32);
					for (ind, channel) in s.iter().enumerate() {
						for (samind, sample) in channel.iter().enumerate() {
							interleaved[ind + samind * md.channels as usize] = *sample;
						}
					}
					samples.extend(interleaved);
					interleaved = vec![];
				}
			}
			#[cfg(not(feature = "vorbis"))]
			{
				unimplemented!("vorbis decoding is disabled in library")
			}
		}
		Encoder::Aac | Encoder::Opus | Encoder::WavPack => unimplemented!(),
	};
	Ok(samples)
}
