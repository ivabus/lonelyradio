use clap::Parser;
use crossterm::cursor::MoveToColumn;
use crossterm::event::{poll, read, Event};
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use std::io::{stdout, IsTerminal};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
struct Args {
	/// Remote address
	address: String,

	/// Do not use backspace control char
	#[arg(short)]
	no_backspace: bool,

	#[arg(long)]
	xor_key_file: Option<PathBuf>,
}

#[cfg(target_os = "linux")]
async fn mpris() {
	use mpris_server::{Metadata, Time, Volume};

	let player = mpris_server::Player::builder("org.mpris.MediaPlayer2.monoclient")
		.identity("monoclient")
		.can_pause(true)
		.build()
		.await;
	let player = match player {
		Ok(p) => p,
		Err(_) => {
			return;
		}
	};
	player.connect_play_pause(|x| {
		monolib::toggle();
		async_std::task::block_on(x.set_playback_status(
			if monolib::get_state() == monolib::State::Playing {
				mpris_server::PlaybackStatus::Playing
			} else {
				mpris_server::PlaybackStatus::Paused
			},
		))
		.unwrap();
	});
	player.connect_set_volume(|_, v: Volume| monolib::set_volume(((v * 255.0) % 256.0) as u8));
	let mut md = monolib::get_metadata().unwrap();
	let mut vol = monolib::get_volume();
	player
		.set_metadata(
			Metadata::builder()
				.artist(vec![&md.artist])
				.album(&md.album)
				.title(&md.title)
				.length(Time::from_secs(md.track_length_secs as i64))
				.build(),
		)
		.await
		.unwrap();
	async_std::task::spawn_local(player.run());
	player.set_can_play(false).await.unwrap();
	player.set_playback_status(mpris_server::PlaybackStatus::Playing).await.unwrap();
	loop {
		let nmd = monolib::get_metadata().unwrap();
		let nvol = monolib::get_volume();
		if nmd != md {
			md = nmd;
			player
				.set_metadata(
					Metadata::builder()
						.artist(vec![&md.artist])
						.album(&md.album)
						.title(&md.title)
						.length(Time::from_secs(md.track_length_secs as i64))
						.build(),
				)
				.await
				.unwrap();
		}
		if nvol != vol {
			vol = nvol;
			player.set_volume(vol as f64 / 255.0).await.unwrap();
		}
		std::thread::sleep(std::time::Duration::from_secs(1))
	}
}

#[cfg(not(target_os = "linux"))]
async fn mpris() {}

#[async_std::main]
async fn main() {
	let mut args = Args::parse();
	args.no_backspace |= !std::io::stdout().is_terminal();
	std::thread::spawn(move || {
		monolib::run(
			&args.address,
			args.xor_key_file.map(|key| std::fs::read(key).expect("Failed to read preshared key")),
		)
	});
	while monolib::get_metadata().is_none() {}
	let mut md = monolib::get_metadata().unwrap();
	async_std::task::spawn_local(mpris());
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
				match event.code {
					crossterm::event::KeyCode::Up => {
						monolib::set_volume(monolib::get_volume().saturating_add(16));
					}
					crossterm::event::KeyCode::Down => {
						monolib::set_volume(monolib::get_volume().saturating_sub(16));
					}
					crossterm::event::KeyCode::Char('q') => {
						crossterm::terminal::disable_raw_mode().unwrap();
						std::process::exit(0)
					}
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
				md.track_length_secs % 60
			);
			track_start = Instant::now();
			seconds_past = 0;
			track_length = md.track_length_secs as f64 + md.track_length_frac as f64
		} else if next_md == md {
			next_md = monolib::get_metadata().unwrap();
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
		//std::thread::sleep(Duration::from_secs_f32(0.25))
	}
}
