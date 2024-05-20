use clap::Parser;
use crossterm::cursor::MoveToColumn;
use crossterm::event::{poll, read, Event};
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use std::io::stdout;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
struct Args {
	/// Remote address
	address: String,

	#[arg(long)]
	xor_key_file: Option<PathBuf>,
}

const HELP: &str = r#"Keybinds:
  Up   - Volume up
  Down - Volume down
  Q    - Quit monoclient
  H    - Show this help"#;

fn main() {
	let args = Args::parse();
	std::thread::spawn(move || {
		monolib::run(
			&args.address,
			args.xor_key_file.map(|key| std::fs::read(key).expect("Failed to read preshared key")),
		)
	});
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
	let mut track_length = md.track_length_secs as f64 + md.track_length_frac as f64;
	let mut next_md = md.clone();
	crossterm::terminal::enable_raw_mode().unwrap();
	loop {
		if let Ok(true) = poll(std::time::Duration::from_micros(1)) {
			if let Event::Key(event) = read().unwrap() {
				match (event.code, event.modifiers) {
					(crossterm::event::KeyCode::Up, _) => {
						monolib::set_volume(monolib::get_volume().saturating_add(4));
					}
					(crossterm::event::KeyCode::Down, _) => {
						monolib::set_volume(monolib::get_volume().saturating_sub(4));
					}
					(crossterm::event::KeyCode::Char('q' | 'й'), _)
					| (
						crossterm::event::KeyCode::Char('c' | 'с'),
						crossterm::event::KeyModifiers::CONTROL,
					) => {
						crossterm::terminal::disable_raw_mode().unwrap();
						println!();
						std::process::exit(0)
					}
					(crossterm::event::KeyCode::Char('h' | 'р'), _) => {
						crossterm::terminal::disable_raw_mode().unwrap();
						crossterm::execute!(
							stdout(),
							Clear(ClearType::CurrentLine),
							MoveToColumn(0)
						)
						.unwrap();
						println!("{}", HELP);
						crossterm::terminal::enable_raw_mode().unwrap();
						seconds_past = (Instant::now() - track_start).as_secs();
						crossterm::execute!(
							stdout(),
							Print(format!(
								"Playing: {} - {} - {} ({}:{:02} / {}:{:02}) [{:.2}]",
								md.artist,
								md.album,
								md.title,
								seconds_past / 60,
								seconds_past % 60,
								md.track_length_secs / 60,
								md.track_length_secs % 60,
								monolib::get_volume() as f32 / 255.0
							))
						)
						.unwrap();
					}
					(crossterm::event::KeyCode::Char(' '), _) => monolib::toggle(),
					_ => {}
				}
			}
		}
		if monolib::get_metadata().unwrap() != md
			&& track_length <= (Instant::now() - track_start).as_secs_f64()
		{
			md = next_md.clone();
			crossterm::execute!(stdout(), Clear(ClearType::CurrentLine), MoveToColumn(0)).unwrap();
			print!(
				"Playing: {} - {} - {} (0:00 / {}:{:02})",
				md.artist,
				md.album,
				md.title,
				md.track_length_secs / 60,
				md.track_length_secs % 60,
			);
			track_start = Instant::now();
			seconds_past = 0;
			track_length = md.track_length_secs as f64 + md.track_length_frac as f64
		} else if next_md == md {
			next_md = monolib::get_metadata().unwrap();
		}
		if (Instant::now() - track_start).as_secs() > seconds_past {
			seconds_past = (Instant::now() - track_start).as_secs();
			crossterm::execute!(stdout(), Clear(ClearType::CurrentLine), MoveToColumn(0)).unwrap();
			crossterm::execute!(
				stdout(),
				Print(format!(
					"Playing: {} - {} - {} ({}:{:02} / {}:{:02}) [{:.2}]",
					md.artist,
					md.album,
					md.title,
					seconds_past / 60,
					seconds_past % 60,
					md.track_length_secs / 60,
					md.track_length_secs % 60,
					monolib::get_volume() as f32 / 255.0
				))
			)
			.unwrap();
		}
		std::thread::sleep(std::time::Duration::from_secs_f32(0.0125))
	}
}
