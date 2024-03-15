use clap::Parser;
use std::io::{IsTerminal, Write};
use std::time::{Duration, Instant};

#[derive(Parser)]
struct Args {
	/// Remote address
	address: String,

	/// Do not use backspace control char
	#[arg(short)]
	no_backspace: bool,
}

fn delete_chars(n: usize, nb: bool) {
	if !nb {
		print!("{}{}{}", "\u{8}".repeat(n), " ".repeat(n), "\u{8}".repeat(n));
		std::io::stdout().flush().expect("Failed to flush stdout")
	} else {
		println!()
	}
}

fn flush() {
	std::io::stdout().flush().unwrap();
}

fn main() {
	let mut args = Args::parse();
	args.no_backspace |= !std::io::stdout().is_terminal();
	std::thread::spawn(move || monolib::run(&args.address));
	while monolib::get_metadata().is_none() {}
	let mut md = monolib::get_metadata().unwrap();
	let seconds = md.length / md.sample_rate as u64 / 2;
	let mut track_start = Instant::now();
	let mut seconds_past = 0;
	let mut msg_len = format!(
		"Playing: {} - {} - {} ({}:{:02})",
		md.artist,
		md.album,
		md.title,
		seconds / 60,
		seconds % 60
	)
	.len();
	print!(
		"Playing: {} - {} - {} ({}:{:02})",
		md.artist,
		md.album,
		md.title,
		seconds / 60,
		seconds % 60
	);
	flush();
	loop {
		if monolib::get_metadata().unwrap() != md {
			md = monolib::get_metadata().unwrap();
			let seconds = md.length / md.sample_rate as u64 / 2;
			delete_chars(msg_len, args.no_backspace);
			msg_len = format!(
				"Playing: {} - {} - {} ({}:{:02})",
				md.artist,
				md.album,
				md.title,
				seconds / 60,
				seconds % 60
			)
			.len();
			print!(
				"Playing: {} - {} - {} (0:00 / {}:{:02})",
				md.artist,
				md.album,
				md.title,
				seconds / 60,
				seconds % 60
			);
			flush();
			track_start = Instant::now();
			seconds_past = 0;
		}
		if (Instant::now() - track_start).as_secs() > seconds_past && !args.no_backspace {
			seconds_past = (Instant::now() - track_start).as_secs();
			msg_len = format!(
				"Playing: {} - {} - {} ({}:{:02} / {}:{:02})",
				md.artist,
				md.album,
				md.title,
				seconds_past / 60,
				seconds_past % 60,
				seconds / 60,
				seconds % 60
			)
			.len();
			delete_chars(msg_len, args.no_backspace);
			print!(
				"Playing: {} - {} - {} ({}:{:02} / {}:{:02})",
				md.artist,
				md.album,
				md.title,
				seconds_past / 60,
				seconds_past % 60,
				seconds / 60,
				seconds % 60
			);
			flush();
		}
		std::thread::sleep(Duration::from_secs_f32(0.05))
	}
}
