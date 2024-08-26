use clap::Parser;
use crossterm::cursor::MoveToColumn;
use crossterm::event::{poll, read, Event};
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use monolib::lonelyradio_types::{Encoder, Settings};
use std::io::stdout;
use std::sync::OnceLock;
use std::time::Instant;

static VERBOSE: OnceLock<bool> = OnceLock::new();

#[derive(Parser)]
struct Args {
	/// Remote address
	address: String,

	#[arg(short, long)]
	verbose: bool,

	#[arg(short, long, default_value = "")]
	playlist: String,

	#[arg(short, long)]
	list: bool,
}

const HELP: &str = r#"Keybinds:
  Up   - Volume up
  Down - Volume down
  Q    - Quit monoclient
  H    - Show this help"#;

macro_rules! verbose {
    ($($arg:tt)*) => {{
    	if *VERBOSE.get().unwrap() {
	    	crossterm::execute!(stdout(), Clear(ClearType::CurrentLine), MoveToColumn(0)).unwrap();
	        println!("{}", format_args!($($arg)*));
	        crossterm::execute!(stdout(), Clear(ClearType::CurrentLine), MoveToColumn(0)).unwrap();
	     }
    }};
}
fn main() {
	let args = Args::parse();
	VERBOSE.set(args.verbose).unwrap();
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
	std::thread::spawn(move || {
		monolib::run(
			&args.address,
			Settings {
				encoder: Encoder::Flac,
				cover: -1,
			},
			&args.playlist,
		)
	});
	while monolib::get_metadata().is_none() {}
	let mut md = monolib::get_metadata().unwrap();
	let mut next_md = md.clone();
	verbose!("md: {:?}", md);
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
		if next_md != md
			&& md.track_length_secs as f64 <= (Instant::now() - track_start).as_secs_f64()
		{
			md = next_md.clone();
			verbose!("md: {:?}", md);
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
