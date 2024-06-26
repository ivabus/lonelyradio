use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum Message {
	T(TrackMetadata),
	F(FragmentMetadata),
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct TrackMetadata {
	pub track_length_secs: u64,
	pub track_length_frac: f32,
	pub channels: u16,
	pub sample_rate: u32,
	pub flac: bool,
	pub title: String,
	pub album: String,
	pub artist: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct FragmentMetadata {
	// In samples or bytes (if FLAC)
	pub length: u64,
}
