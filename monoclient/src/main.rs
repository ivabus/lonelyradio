use byteorder::ByteOrder;
use clap::Parser;
use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use serde::Deserialize;
use std::io::{IsTerminal, Read, Write};
use std::net::TcpStream;

// How many samples to cache before playing in samples (both channels) SHOULD BE EVEN
const BUFFER_SIZE: usize = 4800;
// How many buffers to cache
const CACHE_SIZE: usize = 10;

#[derive(Deserialize, Debug)]
struct SentMetadata {
	// In bytes, we need to read next track metadata
	lenght: u64,
	sample_rate: u32,
	title: String,
	album: String,
	artist: String,
}

#[derive(Parser)]
struct Args {
	/// Remote address
	address: String,

	/// Do not use backspace control char
	#[arg(short)]
	no_backspace: bool,
}

fn delete_chars(n: usize) {
	print!("{}{}{}", "\u{8}".repeat(n), " ".repeat(n), "\u{8}".repeat(n));
	std::io::stdout().flush().expect("Failed to flush stdout")
}

fn main() {
	let mut args = Args::parse();
	args.no_backspace |= !std::io::stdout().is_terminal();
	let mut stream = TcpStream::connect(&args.address)
		.unwrap_or_else(|err| panic!("Failed to connect to {}: {}", args.address, err.to_string()));
	println!("Connected to {} from {}", stream.peer_addr().unwrap(), stream.local_addr().unwrap());
	let (_stream, stream_handle) = OutputStream::try_default().unwrap();
	let sink = Sink::try_new(&stream_handle).unwrap();
	let mut buffer = [0u8; 2];
	let mut samples = [0f32; BUFFER_SIZE];
	let mut latest_msg_len = 0;
	print!("Playing: ");
	loop {
		let mut index = 0usize;

		let md: SentMetadata =
			rmp_serde::from_read(&stream).expect("Failed to parse track metadata");
		let seconds = md.lenght / (2 * md.sample_rate as u64);
		let total_lenght = format!("{}:{:02}", seconds / 60, seconds % 60);
		let message = format!("{} - {} - {} ", md.artist, md.album, md.title);
		if latest_msg_len != 0 {
			if args.no_backspace {
				print!("\nPlaying: ");
			} else {
				delete_chars(latest_msg_len)
			}
		}
		print!("{}", message);
		let mut prev_timestamp_len = 0;
		if args.no_backspace {
			print!("({})", &total_lenght)
		} else {
			print!("(0:00 / {})", &total_lenght);
			// (0:00/ + :00 + minutes len
			prev_timestamp_len = 12 + format!("{}", seconds / 60).len();
		}
		std::io::stdout().flush().expect("Failed to flush stdout");
		latest_msg_len = message.chars().count();
		let mut second = 0;
		for sample_index in 0..md.lenght {
			if (sample_index / (md.sample_rate as u64 * 2)) > second {
				second += 1;
				if !args.no_backspace {
					delete_chars(prev_timestamp_len);
					let current_timestamp =
						format!("({}:{:02} / {})", second / 60, second % 60, &total_lenght);
					print!("{}", &current_timestamp);
					std::io::stdout().flush().expect("Failed to flush stdout");
					prev_timestamp_len = current_timestamp.len()
				}
			}
			if stream.read_exact(&mut buffer).is_err() {
				return;
			};

			samples[index] = byteorder::LittleEndian::read_i16(&buffer[..2]) as f32 / 32768.0;
			index += 1;

			if index == BUFFER_SIZE {
				// Sink's thread is detached from main thread, so we need to synchronize with it
				// Why we should synchronize with it?
				// Let's say, that if we don't synchronize with it, we would have
				// a lot (no upper limit, actualy) of buffered sound, waiting for playing in sink
				while sink.len() >= CACHE_SIZE {
					// Sleeping exactly one buffer
					std::thread::sleep(std::time::Duration::from_secs_f32(
						BUFFER_SIZE as f32 / md.sample_rate as f32 / 2.0,
					))
				}
				sink.append(SamplesBuffer::new(2, md.sample_rate, samples.as_slice()));
				index = 0;
			}
		}
		sink.sleep_until_end()
	}
}
