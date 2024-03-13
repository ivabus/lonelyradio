use byteorder::ByteOrder;
use clap::Parser;
use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use std::io::Read;
use std::net::TcpStream;
use std::time::Instant;

// How many samples to cache before playing in samples (both channels) SHOULD BE EVEN
const BUFFER_SIZE: usize = 96000;
// How many buffers to cache
const CACHE_SIZE: usize = 20;

enum Channel {
	Right,
	Left,
	Stereo,
}

#[derive(Parser)]
struct Args {
	/// Remote address
	address: String,
	#[arg(short, long, default_value = "s")]
	/// L, R or S for Left, Right or Stereo
	channel: String,
	#[arg(short)]
	/// Play only on specified channel, with it if channel = Right => L=0 and R=R, without L=R and R=R. No effect on Stereo
	single: bool,

	/// More verbose
	#[arg(short)]
	verbose: bool,

	/// Stream in f32le instead of s16le
	#[arg(short, long)]
	float: bool,

	/// Stream in custom sample rate
	#[arg(short, long, default_value = "44100")]
	sample_rate: u32,
}

fn main() {
	let start = Instant::now();
	let args = Args::parse();
	let mut stream = TcpStream::connect(args.address).unwrap();
	if args.verbose {
		eprintln!(
			"Connected to {} from {}",
			stream.peer_addr().unwrap(),
			stream.local_addr().unwrap()
		)
	}

	let channel = match args.channel.to_ascii_lowercase().as_str() {
		"l" => Channel::Left,
		"r" => Channel::Right,
		"s" => Channel::Stereo,
		_ => panic!("Wrong channel specified"),
	};
	let (_stream, stream_handle) = OutputStream::try_default().unwrap();
	let sink = Sink::try_new(&stream_handle).unwrap();
	let mut buffer = vec![
		0u8;
		if args.float {
			8
		} else {
			4
		}
	];
	let mut samples = [0f32; BUFFER_SIZE];
	let mut index = 0usize;
	let mut first = true;
	while stream.read_exact(&mut buffer).is_ok() {
		let sample_l = if args.float {
			byteorder::LittleEndian::read_f32(&buffer[..4])
		} else {
			byteorder::LittleEndian::read_i16(&buffer[..2]) as f32 / 32768.0
		};
		let sample_r = if args.float {
			byteorder::LittleEndian::read_f32(&buffer[4..])
		} else {
			byteorder::LittleEndian::read_i16(&buffer[2..]) as f32 / 32768.0
		};
		// Left channel
		samples[index] = match channel {
			Channel::Left | Channel::Stereo => sample_l,
			Channel::Right => {
				if args.single {
					0f32
				} else {
					sample_r
				}
			}
		};
		index += 1;
		// Right channel
		samples[index] = match channel {
			Channel::Right | Channel::Stereo => sample_r,
			Channel::Left => {
				if args.single {
					0f32
				} else {
					sample_l
				}
			}
		};
		index += 1;
		if index == BUFFER_SIZE {
			let mut first_wait_iteration = true;
			// Sink's thread is detached from main thread, so we need to synchronize with it
			// Why we should synchronize with it?
			// Let's say, that if we don't synchronize with it, we would have
			// a lot (no upper limit, actualy) of buffered sound, waiting for playing in sink
			while sink.len() >= CACHE_SIZE {
				if args.verbose && first_wait_iteration {
					eprint!(".");
					first_wait_iteration = false;
				}
				// Sleeping exactly one buffer
				std::thread::sleep(std::time::Duration::from_secs_f32(
					(if sink.len() >= 2 {
						sink.len() - 2
					} else {
						1
					} as f32) * BUFFER_SIZE as f32
						/ args.sample_rate as f32
						/ 2.0,
				))
			}
			if first && args.verbose {
				eprintln!("Started playing in {} ms", (Instant::now() - start).as_millis());
				first = false;
			}
			sink.append(SamplesBuffer::new(2, args.sample_rate, samples.as_slice()));
			index = 0;
		}
	}
}
