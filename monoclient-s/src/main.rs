use std::time::Duration;

use monolib::lonelyradio_types;
use monolib::State;
use slint::{
	Image, ModelRc, Rgb8Pixel, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel, Weak,
};

slint::include_modules!();

#[allow(dead_code)]
fn start_playback(window_weak: Weak<MainWindow>) {
	let window = window_weak.upgrade().unwrap();
	let addr = window.get_addr().to_string();
	let playlist = window.get_selected_playlist();
	let encoder = monolib::SUPPORTED_DECODERS[window.get_selected_encoder() as usize];
	let handle = std::thread::spawn(move || {
		monolib::run(
			&addr,
			lonelyradio_types::Settings {
				encoder,
				cover: 2048,
			},
			if playlist == "All tracks" {
				""
			} else {
				&playlist
			},
		)
	});
	std::thread::sleep(Duration::from_millis(166));
	if handle.is_finished() {
		window.set_playing(false);
		return;
	}
	window.set_playing(true);
	window.set_paused(false);
	while monolib::get_metadata().is_none() {}
	monolib::set_volume(window.get_volume() as u8);
}

pub fn main() {
	let window = MainWindow::new().unwrap();

	let window_weak = window.as_weak();
	window.on_text_edited(move || {
		let window = window_weak.upgrade().unwrap();
		let addr = window.get_addr().to_string();
		if addr.contains(':') {
			window.set_start_enabled(true);
		} else {
			window.set_start_enabled(false);
		}
	});

	let window_weak = window.as_weak();
	window.on_play(move || {
		match monolib::get_state() {
			State::NotStarted => start_playback(window_weak.clone()),
			State::Paused => {
				let window = window_weak.upgrade().unwrap();
				window.set_paused(false);
				monolib::toggle();
			}
			State::Resetting => {}
			State::Playing => {
				let window = window_weak.upgrade().unwrap();
				window.set_paused(true);
				monolib::toggle()
			}
		}
		let window = window_weak.upgrade().unwrap();

		let playlists = match monolib::list_playlists(&window.get_addr()) {
			Some(v) => [vec!["All tracks".to_string()], v].concat(),
			None => vec!["All tracks".to_string()],
		};
		window.set_playlists(ModelRc::new(VecModel::from(
			playlists.iter().map(SharedString::from).collect::<Vec<_>>(),
		)));
	});

	window.set_supported_encoders(ModelRc::new(VecModel::from(
		monolib::SUPPORTED_DECODERS
			.iter()
			.map(|x| x.to_string())
			.map(SharedString::from)
			.collect::<Vec<_>>(),
	)));

	let window_weak = window.as_weak();
	window.on_next(move || {
		monolib::stop();
		start_playback(window_weak.clone())
	});
	let window_weak = window.as_weak();
	window.on_stop(move || {
		let window = window_weak.upgrade().unwrap();
		window.set_playing(false);
		window.set_martist("".into());
		window.set_malbum("".into());
		window.set_mtitle("".into());
		window.set_cover(Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::new(1, 1)));
		monolib::stop();
	});
	window.on_change_volume(move |vol| monolib::set_volume(vol as u8));

	let window_weak = window.as_weak();
	std::thread::spawn(move || loop {
		let window = window_weak.clone();
		while monolib::get_metadata().is_none() {
			std::thread::sleep(Duration::from_millis(25))
		}
		let md = monolib::get_metadata().unwrap();
		let _md = md.clone();
		if let Some(jpeg) = md.cover {
			let mut decoder = zune_jpeg::JpegDecoder::new(jpeg);
			decoder.decode_headers().unwrap();
			let (w, h) = decoder.dimensions().unwrap();
			let decoded = decoder.decode().unwrap();
			let mut pixel_buffer = SharedPixelBuffer::<Rgb8Pixel>::new(w as u32, h as u32);
			pixel_buffer.make_mut_bytes().copy_from_slice(&decoded);
			window
				.upgrade_in_event_loop(|win| {
					let image = Image::from_rgb8(pixel_buffer);
					win.set_cover(image);
				})
				.unwrap();
		} else {
			window
				.upgrade_in_event_loop(|win| {
					win.set_cover(Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::new(1, 1)));
				})
				.unwrap();
		}
		slint::invoke_from_event_loop(move || {
			let window = window.unwrap();
			window.set_martist(md.artist.clone().into());
			window.set_malbum(md.album.clone().into());
			window.set_mtitle(md.title.clone().into());
		})
		.unwrap();
		while monolib::get_metadata() == Some(_md.clone()) {
			std::thread::sleep(Duration::from_millis(100))
		}
	});
	window.run().unwrap();
}
