use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use lonelyradio_types::{FragmentMetadata, TrackMetadata};
use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::Duration;
use tokio::net::TcpListener;

static QUEUE: Lazy<Arc<RwLock<VecDeque<Vec<i16>>>>> =
	Lazy::new(|| Arc::new(RwLock::new(VecDeque::new())));

static START_INDEX: Mutex<usize> = Mutex::new(0);

#[tokio::main]
async fn main() {
	tokio::spawn(listen_mic());
	println!("Started buffering");
	let listener = TcpListener::bind("0.0.0.0:5894").await.unwrap();
	std::thread::sleep(Duration::from_secs(5));
	tokio::spawn(update_start());
	println!("Accepting connections");
	loop {
		let (socket, _) = listener.accept().await.unwrap();
		let socket = socket.into_std().unwrap();
		tokio::spawn(stream(socket));
	}
}

async fn update_start() {
	loop {
		std::thread::sleep(Duration::from_secs(1));
		*START_INDEX.lock().unwrap() = QUEUE.read().unwrap().len() - 5;
	}
}

async fn stream(mut s: std::net::TcpStream) {
	println!("Playing for {}", s.peer_addr().unwrap());
	let md = lonelyradio_types::Message::T(TrackMetadata {
		cover: None,
		encoder: lonelyradio_types::Encoder::Pcm,
		track_length_secs: 0,
		track_length_frac: 0.0,
		channels: 1,
		sample_rate: 44100,
		title: "microserve instance".to_string(),
		album: "".to_string(),
		artist: "".to_string(),
	});
	s.write_all(rmp_serde::to_vec(&md).unwrap().as_slice()).unwrap();
	let mut ind = *START_INDEX.lock().unwrap();
	dbg!(ind);
	loop {
		let front = QUEUE.read().unwrap()[ind].clone();
		ind += 1;
		let md = lonelyradio_types::Message::F(FragmentMetadata {
			length: front.len() as u64,
		});
		s.write_all(rmp_serde::to_vec(&md).unwrap().as_slice()).unwrap();

		if s.write_all(unsafe { front.as_slice().align_to::<u8>().1 }).is_err() {
			return;
		};
		while ind >= QUEUE.read().unwrap().len() - 5 {
			std::thread::sleep(Duration::from_millis(100))
		}
	}
}

async fn listen_mic() {
	let host = cpal::default_host();

	let device = host.default_input_device().unwrap();
	let config = device.default_input_config().unwrap();
	let stream = match config.sample_format() {
		cpal::SampleFormat::F32 => device.build_input_stream(
			&config.into(),
			move |data: &[f32], _: &_| {
				let samples = data.iter().map(|x| (*x * 32767.0) as i16).collect();
				QUEUE.write().unwrap().push_back(samples);
			},
			|e| eprintln!("Error while reading: {}", e),
			None,
		),
		_ => {
			unimplemented!()
		}
	}
	.unwrap();
	loop {
		stream.play().unwrap();
		std::thread::sleep(Duration::from_millis(100));
	}
}
