use byteorder::ByteOrder;
use clap::Parser;
use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use serde::Deserialize;
use std::io::{Read, Write};
use std::net::TcpStream;

// How many samples to cache before playing in samples (both channels) SHOULD BE EVEN
const BUFFER_SIZE: usize = 2400;
// How many buffers to cache
const CACHE_SIZE: usize = 100;

enum Channel {
	Right,
	Left,
	Stereo,
}

#[derive(Deserialize, Debug)]
struct SentMetadata {
	// In bytes, we need to read next track metadata
	lenght: u64,
	title: String,
	album: String,
	artist: String,
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

	/// Do not erase previously played track from stdout
	#[arg(short)]
	no_backspace: bool,
}

fn main() {
	let args = Args::parse();
	let mut stream = TcpStream::connect(args.address).unwrap();
	println!("Connected to {} from {}", stream.peer_addr().unwrap(), stream.local_addr().unwrap());

	let channel = match args.channel.to_ascii_lowercase().as_str() {
		"l" => Channel::Left,
		"r" => Channel::Right,
		"s" => Channel::Stereo,
		_ => panic!("Wrong channel specified"),
	};
	let (_stream, stream_handle) = OutputStream::try_default().unwrap();
	let sink = Sink::try_new(&stream_handle).unwrap();
	let mut buffer = [0u8; 4];
	let mut samples = [0f32; BUFFER_SIZE];
	let mut latest_msg_len = 0;
	print!("Playing: ");
	loop {
		let mut index = 0usize;

		let md: SentMetadata = rmp_serde::from_read(&stream).unwrap();
		let seconds = md.lenght / (2 * 44100);
		let message = format!(
			"{} - {} - {} ({}:{:02})",
			md.artist,
			md.album,
			md.title,
			seconds / 60,
			seconds % 60
		);
		if latest_msg_len != 0 {
			if args.no_backspace {
				print!("\nPlaying: ");
			} else {
				print!("{}", "\u{8}".repeat(latest_msg_len));
				print!("{}", " ".repeat(latest_msg_len));
				print!("{}", "\u{8}".repeat(latest_msg_len));
			}
		}
		print!("{}", message);
		std::io::stdout().flush().unwrap();
		latest_msg_len = message.chars().count();

		for _ in 0..md.lenght / 2 {
			if stream.read_exact(&mut buffer).is_err() {
				return;
			};
			let sample_l = byteorder::LittleEndian::read_i16(&buffer[..2]) as f32 / 32768.0;
			let sample_r = byteorder::LittleEndian::read_i16(&buffer[2..]) as f32 / 32768.0;
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
				// Sink's thread is detached from main thread, so we need to synchronize with it
				// Why we should synchronize with it?
				// Let's say, that if we don't synchronize with it, we would have
				// a lot (no upper limit, actualy) of buffered sound, waiting for playing in sink
				while sink.len() >= CACHE_SIZE {
					// Sleeping exactly one buffer
					std::thread::sleep(std::time::Duration::from_secs_f32(
						(if sink.len() >= 2 {
							sink.len() - 2
						} else {
							1
						} as f32) * BUFFER_SIZE as f32
							/ 44100.0 / 2.0,
					))
				}
				sink.append(SamplesBuffer::new(2, 44100, samples.as_slice()));
				index = 0;
			}
		}
		while sink.len() != 0 {
			std::thread::sleep(std::time::Duration::from_secs_f32(0.01))
		}
	}
}
