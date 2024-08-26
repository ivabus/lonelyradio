use serde::{Deserialize, Serialize};

pub const HELLO_MAGIC: &[u8; 8] = b"lonelyra";

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum Request {
	// Just play what server wants you to give
	#[serde(rename = "p", alias = "Play")]
	Play(Settings),
	#[serde(rename = "lpl", alias = "ListPlaylist")]
	ListPlaylist,
	#[serde(rename = "ppl", alias = "PlayPlaylist")]
	PlayPlaylist(String, Settings),
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct PlaylistResponce {
	pub playlists: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum RequestResult {
	Ok,
	Playlist(PlaylistResponce),
	Error(RequestError),
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum RequestError {
	NoSuchPlaylist,
	WrongCoverSize,
	UnsupportedEncoder,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum PlayMessage {
	T(TrackMetadata),
	F(FragmentMetadata),
}

#[repr(C)]
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct Settings {
	#[serde(rename = "e", alias = "encoder")]
	pub encoder: Encoder,

	#[serde(rename = "co", alias = "cover")]
	pub cover: i32,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct ServerCapabilities {
	#[serde(rename = "e")]
	pub encoders: Vec<Encoder>,
	// Will be used in the next updates
	//#[serde(rename = "ar")]
	//pub available_requests: Vec<Request>,
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

	#[serde(
		rename = "co",
		skip_serializing_if = "Option::is_none",
		with = "serde_bytes",
		default = "none"
	)]
	pub cover: Option<Vec<u8>>,

	pub id: u8,
}

// WavPack, Opus and Aac are currently unimplemented.
#[repr(u8)]
#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq)]
pub enum Encoder {
	Pcm16 = 0,
	PcmFloat = 1,
	Flac = 2,
	Alac = 3,
	WavPack = 4,
	Opus = 5,
	Aac = 6,
	Vorbis = 7,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct FragmentMetadata {
	// In bytes or samples, depends on encoder: Pcm* - samples, any compressed - bytes
	#[serde(rename = "le")]
	pub length: u64,

	#[serde(
		rename = "mc",
		skip_serializing_if = "Option::is_none",
		with = "serde_bytes",
		default = "none"
	)]
	pub magic_cookie: Option<Vec<u8>>,
}

fn none() -> Option<Vec<u8>> {
	None
}
