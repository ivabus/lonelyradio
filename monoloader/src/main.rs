use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
	/// Remote address
	address: String,

	#[arg(long)]
	xor_key_file: Option<PathBuf>,
}

fn main() {
	let args = Args::parse();
	let (md, samples) = monolib::get_track(
		&args.address,
		args.xor_key_file.map(|key| std::fs::read(key).expect("Failed to read preshared key")),
	)
	.unwrap();
	println!(
		"Downloaded: {} - {} - {} ({} MB)",
		md.artist,
		md.album,
		md.title,
		samples.len() as f32 * 2.0 / 1024.0 / 1024.0
	);
	let spec = hound::WavSpec {
		channels: md.channels,
		sample_rate: md.sample_rate,
		bits_per_sample: 16,
		sample_format: hound::SampleFormat::Int,
	};
	let mut writer =
		hound::WavWriter::create(format!("{} - {}.wav", md.artist, md.title), spec).unwrap();
	let mut writer_i16 = writer.get_i16_writer(samples.len() as u32);
	samples.iter().for_each(|s| writer_i16.write_sample(*s));
	writer_i16.flush().unwrap();
}
