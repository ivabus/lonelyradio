use clap::Parser;
use crossterm::cursor::MoveToColumn;
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use std::io::{stdout, IsTerminal};
use std::time::{Duration, Instant};

#[derive(Parser)]
struct Args {
	/// Remote address
	address: String,

	/// Do not use backspace control char
	#[arg(short)]
	no_backspace: bool,
}

fn main() {
	let mut args = Args::parse();
	args.no_backspace |= !std::io::stdout().is_terminal();
	std::thread::spawn(move || monolib::run(&args.address));
	while monolib::get_metadata().is_none() {}
	let mut md = monolib::get_metadata().unwrap();
	let mut track_start = Instant::now();
	let mut seconds_past = 0;
	crossterm::execute!(
		stdout(),
		Print(format!(
			"Playing: {} - {} - {} ({}:{:02})",
			md.artist,
			md.album,
			md.title,
			md.track_length_secs / 60,
			md.track_length_secs % 60
		))
	)
	.unwrap();
	loop {
		if monolib::get_metadata().unwrap() != md {
			md = monolib::get_metadata().unwrap();
			crossterm::execute!(stdout(), Clear(ClearType::CurrentLine), MoveToColumn(0)).unwrap();
			print!(
				"Playing: {} - {} - {} (0:00 / {}:{:02})",
				md.artist,
				md.album,
				md.title,
				md.track_length_secs / 60,
				md.track_length_secs % 60
			);
			track_start = Instant::now();
			seconds_past = 0;
		}
		if (Instant::now() - track_start).as_secs() > seconds_past && !args.no_backspace {
			seconds_past = (Instant::now() - track_start).as_secs();
			crossterm::execute!(stdout(), Clear(ClearType::CurrentLine), MoveToColumn(0)).unwrap();
			crossterm::execute!(
				stdout(),
				Print(format!(
					"Playing: {} - {} - {} ({}:{:02} / {}:{:02})",
					md.artist,
					md.album,
					md.title,
					seconds_past / 60,
					seconds_past % 60,
					md.track_length_secs / 60,
					md.track_length_secs % 60
				))
			)
			.unwrap();
		}
		std::thread::sleep(Duration::from_secs_f32(0.25))
	}
}
