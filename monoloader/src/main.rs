use clap::Parser;
use monolib::lonelyradio_types::Settings;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
	/// Remote address
	address: String,

	#[arg(long)]
	xor_key_file: Option<PathBuf>,

	#[arg(short, long, default_value = "")]
	playlist: String,

	#[arg(short, long)]
	list: bool,
}

fn main() {
	let args = Args::parse();

	if args.list {
		println!(
			"Available playlists: {}",
			match monolib::list_playlists(&args.address) {
				Some(s) => format!("{:?}", s),
				None => String::from("None"),
			}
		);
		return;
	}

	let (md, samples) = monolib::get_track(
		&args.address,
		Settings {
			encoder: monolib::lonelyradio_types::Encoder::Flac,
			cover: -1,
		},
		&args.playlist,
	)
	.unwrap();
	println!(
		"Downloaded: {} - {} - {} ({:?}, {} MiB)",
		md.artist,
		md.album,
		md.title,
		md.encoder,
		samples.len() as f32 / 256.0 / 1024.0
	);
	let spec = hound::WavSpec {
		channels: md.channels,
		sample_rate: md.sample_rate,
		bits_per_sample: 32,
		sample_format: hound::SampleFormat::Float,
	};
	let mut writer =
		hound::WavWriter::create(format!("{} - {}.wav", md.artist, md.title), spec).unwrap();
	samples.iter().for_each(|s| writer.write_sample(*s).unwrap());
	writer.flush().unwrap();
}
