mod decode;

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Local;
use clap::Parser;
use futures_util::pin_mut;
use lofty::Accessor;
use lofty::TaggedFileExt;
use rand::prelude::*;
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use walkdir::DirEntry;

use crate::decode::decode_file;
use crate::decode::get_meta;

#[derive(Parser)]
struct Args {
	dir: PathBuf,
	#[arg(short, default_value = "0.0.0.0:5894")]
	address: String,

	#[arg(short, long)]
	public_log: bool,

	#[arg(short, long)]
	war: bool,

	#[arg(short, long, default_value = "96000")]
	max_samplerate: u32,
}

#[derive(Serialize, Clone)]
struct SentMetadata {
	/// Fragment length
	length: u64,
	/// Total track length
	track_length_secs: u64,
	track_length_frac: f32,
	channels: u16,
	sample_rate: u32,
	title: String,
	album: String,
	artist: String,
}

async fn stream_samples(
	samples_stream: impl Stream<Item = Vec<i16>>,
	war: bool,
	md: SentMetadata,
	s: &mut TcpStream,
) -> bool {
	pin_mut!(samples_stream);

	while let Some(samples) = samples_stream.next().await {
		let mut md = md.clone();
		md.length = samples.len() as u64;
		if s.write_all(rmp_serde::to_vec(&md).unwrap().as_slice()).await.is_err() {
			return true;
		}
		for sample in samples {
			if s.write_all(
				&(if war {
					sample.signum() * 32767
				} else {
					sample
				}
				.to_le_bytes()),
			)
			.await
			.is_err()
			{
				return true;
			};
		}
	}
	false
}

#[tokio::main]
async fn main() {
	let listener = TcpListener::bind(Args::parse().address).await.unwrap();
	let tracklist = Arc::new(
		walkdir::WalkDir::new(Args::parse().dir)
			.into_iter()
			.filter_entry(is_not_hidden)
			.filter_map(|v| v.ok())
			.map(|x| x.into_path())
			.filter(|x| track_valid(x))
			.collect::<Vec<PathBuf>>(),
	);
	loop {
		let (socket, _) = listener.accept().await.unwrap();
		tokio::spawn(stream(socket, tracklist.clone()));
	}
}
fn is_not_hidden(entry: &DirEntry) -> bool {
	entry.file_name().to_str().map(|s| entry.depth() == 0 || !s.starts_with('.')).unwrap_or(false)
}

fn track_valid(track: &Path) -> bool {
	if !track.metadata().unwrap().is_file() {
		return false;
	}
	// Skipping "images" (covers)
	if "jpgjpegpngwebp".contains(&track.extension().unwrap().to_str().unwrap().to_ascii_lowercase())
	{
		return false;
	}
	true
}

async fn stream(mut s: TcpStream, tracklist: Arc<Vec<PathBuf>>) {
	let args = Args::parse();
	s.set_nodelay(true).unwrap();

	loop {
		let track = tracklist.choose(&mut thread_rng()).unwrap().clone();

		let mut title = String::new();
		let mut artist = String::new();
		let mut album = String::new();
		let mut file = std::fs::File::open(&track).unwrap();
		let tagged = lofty::read_from(&mut file).unwrap();
		if let Some(id3v2) = tagged.primary_tag() {
			title =
				id3v2.title().unwrap_or(track.file_stem().unwrap().to_string_lossy()).to_string();
			album = id3v2.album().unwrap_or("[No tag]".into()).to_string();
			artist = id3v2.artist().unwrap_or("[No tag]".into()).to_string();
		};
		let track_message = format!("{} - {} - {}", &artist, &album, &title);
		eprintln!(
			"[{}] {} to {}:{}{}",
			Local::now().to_rfc3339(),
			track_message,
			s.peer_addr().unwrap().ip(),
			s.peer_addr().unwrap().port(),
			if args.war {
				" with WAR.rs"
			} else {
				""
			}
		);

		if args.public_log {
			println!(
				"[{}] {} to {}{}",
				Local::now().to_rfc3339(),
				track.to_str().unwrap(),
				s.peer_addr().unwrap().port(),
				if args.war {
					" with WAR.rs"
				} else {
					""
				}
			);
		}
		let (channels, sample_rate, time) = get_meta(track.as_path()).await;
		let (tx, mut rx) = mpsc::channel::<Vec<i16>>(8192);
		tokio::spawn(decode_file(track, tx));
		let stream = async_stream::stream! {
			while let Some(item) = rx.recv().await {
				yield item;
			}
		};
		if stream_samples(
			stream,
			args.war,
			SentMetadata {
				album,
				artist,
				title,
				length: 0,
				track_length_frac: time.frac as f32,
				track_length_secs: time.seconds,
				sample_rate,
				channels,
			},
			&mut s,
		)
		.await
		{
			return;
		};
	}
}
