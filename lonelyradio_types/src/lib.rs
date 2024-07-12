use serde::{Deserialize, Serialize};

pub const HELLO_MAGIC: u64 = 0x104e1374d10;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum Message {
	T(TrackMetadata),
	F(FragmentMetadata),
}

#[repr(C)]
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct Settings {
	#[serde(rename = "e")]
	pub encoder: Encoder,

	#[serde(rename = "co")]
	pub cover: i32,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct ServerCapabilities {
	#[serde(rename = "e")]
	pub encoders: Vec<Encoder>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct TrackMetadata {
	#[serde(rename = "tls")]
	pub track_length_secs: u64,
	#[serde(rename = "tlf")]
	pub track_length_frac: f32,
	#[serde(rename = "c")]
	pub channels: u16,
	#[serde(rename = "sr")]
	pub sample_rate: u32,
	#[serde(rename = "e")]
	pub encoder: Encoder,
	#[serde(rename = "mt")]
	pub title: String,
	#[serde(rename = "mal")]
	pub album: String,
	#[serde(rename = "mar")]
	pub artist: String,
	#[serde(rename = "co")]
	#[serde(with = "serde_bytes")]
	pub cover: Option<Vec<u8>>,
}

#[repr(u8)]
#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq)]
pub enum Encoder {
	Pcm16 = 0,
	PcmFloat = 1,
	Flac = 2,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct FragmentMetadata {
	// In bytes
	#[serde(rename = "l")]
	pub length: u64,
}
