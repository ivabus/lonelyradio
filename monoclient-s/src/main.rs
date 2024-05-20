use std::time::Duration;

use monolib::State;
use slint::Weak;

slint::slint! {
	import { AboutSlint, Button, VerticalBox, GroupBox, Slider } from "std-widgets.slint";
export component MainWindow inherits Window {
	max-height: self.preferred-height;
	callback play;
	callback stop;
	callback next;
	callback change_volume(float);
	callback text_edited;

	in-out property <string> addr: address.text;
	in-out property <string> mtitle: "";
	in-out property <string> malbum: "";
	in-out property <string> martist: "";
	in-out property <float> volume: svolume.value;
	in-out property <bool> start_enabled: false;
	in-out property <bool> playing: false;
	in-out property <bool> paused: false;

	title: "monoclient-s";
	min-width: 192px;
	max-width: 768px;
	VerticalBox {
		alignment: center;
		GroupBox{
			max-width: 768px;
			address := TextInput {
				text: "";
				horizontal-alignment: center;
				height: 1.25rem;

				accepted => {
					self.clear_focus()
				}

				edited => {
					text_edited()
				}
			}
		}
		VerticalLayout {
			max-width: 512px;
			VerticalLayout {
				spacing: 4px;
				Button {
					max-width: 256px;
					text: playing ? (paused ? "Play" : "Pause") : "Start";
					enabled: start_enabled || playing;
					clicked => {
						play()
					}
				}
				HorizontalLayout {
					spacing: 4px;
					max-width: 256px;
					Button {
						text: "Stop";
						enabled: playing && !paused;
						clicked => {
							stop()
						}
					}
					Button {
						text: "Next";
						enabled: playing && !paused;
						clicked => {
							next()
						}
					}
				}
				svolume := Slider {
					value: 255;
					maximum: 255;
					changed(f) => {
						change_volume(f)
					}
				}
			}
			tartist := Text {
				height: 1.25rem;
				font-weight: 600;
				text: martist;
				overflow: elide;
			}
			talbum := Text {
				height: 1.25rem;
				text: malbum;
				overflow: elide;
			}
			ttitle := Text {
				height: 1.25rem;
				text: mtitle;
				overflow: elide;
			}
		}
	}
}
}

fn start_playback(window_weak: Weak<MainWindow>) {
	let window = window_weak.upgrade().unwrap();
	let addr = window.get_addr().to_string();
	let handle = std::thread::spawn(move || monolib::run(&addr, None));
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
		window.set_start_enabled(addr.contains(':'));
	});

	let window_weak = window.as_weak();
	window.on_play(move || match monolib::get_state() {
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
	});

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
