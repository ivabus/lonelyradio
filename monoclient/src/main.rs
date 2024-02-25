use byteorder::ByteOrder;
use clap::Parser;
use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use std::io::Read;
use std::net::TcpStream;

// How many samples to cache before playing (half a second)
const BUFFER_SIZE: usize = 44100;

enum Channel {
	Right,
	Left,
	Stereo,
}

#[derive(Parser)]
struct Args {
	/// Remote address
	address: String,
	/// L, R or S for Left, Right or Stereo
	channel: String,
	#[arg(short)]
	/// Play only on specified channel, with it if channel = Right => L=0 and R=R, without L=R and R=R. No effect on Stereo
	single: bool,
}

fn main() {
	let args = Args::parse();
	let mut stream = TcpStream::connect(args.address).unwrap();

	let channel = match args.channel.to_ascii_lowercase().as_str() {
		"l" => Channel::Left,
		"r" => Channel::Right,
		"s" => Channel::Stereo,
		_ => panic!("Wrong channel specified"),
	};
	let (_stream, stream_handle) = OutputStream::try_default().unwrap();
	let sink = Sink::try_new(&stream_handle).unwrap();
	let mut buffer = [0u8; 4];
	let mut samples = vec![];
	while stream.read_exact(&mut buffer).is_ok() {
		let sample_l = byteorder::LittleEndian::read_i16(&buffer[..2]) as f32 / 32768.0;
		let sample_r = byteorder::LittleEndian::read_i16(&buffer[2..]) as f32 / 32768.0;
		// Left channel
		samples.push(match channel {
			Channel::Left | Channel::Stereo => sample_l,
			Channel::Right => {
				if args.single {
					0f32
				} else {
					sample_r
				}
			}
		});
		// Right channel
		samples.push(match channel {
			Channel::Right | Channel::Stereo => sample_r,
			Channel::Left => {
				if args.single {
					0f32
				} else {
					sample_l
				}
			}
		});
		if samples.len() >= BUFFER_SIZE {
			// What is next three lines?
			// Sink's thread is detached from main thread, so we need to somehow synchronize with it
			// Why we should synchronize with it?
			// Let's say, that if we don't synchronize with it, we would have
			// a lot (no upper limit, actualy) of buffered sound, waiting for playing in sink
			while sink.len() >= 3
			// buffered >= 1.5 sec
			{
				std::thread::sleep(std::time::Duration::from_secs_f32(0.25))
			}
			sink.append(SamplesBuffer::new(2, 44100, samples));
			samples = vec![];
		}
	}
}
