use crate::*;

use std::ffi::{c_char, c_float};
use std::ffi::{CStr, CString};

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct CTrackMetadata {
	pub title: *mut c_char,
	pub album: *mut c_char,
	pub artist: *mut c_char,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct CSettings {
	/// See lonelyradio_types for numeric representation -> Encoder
	pub encoder: u8,
	pub cover: i32,
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
/// Starts audio playback using rodio
/// Play without playlist => playlist = ""
pub extern "C" fn c_start(server: *const c_char, settings: CSettings, playlist: *const c_char) {
	let serv = unsafe { CStr::from_ptr(server) };
	let playlist = unsafe { CStr::from_ptr(playlist) };
	run(
		serv.to_str().unwrap_or_default(),
		Settings {
			encoder: match settings.encoder {
				0 => Encoder::Pcm16,
				1 => Encoder::PcmFloat,
				2 => Encoder::Flac,
				3 => Encoder::Alac,
				7 => Encoder::Vorbis,
				8 => Encoder::Sea,
				_ => return,
			},
			cover: settings.cover,
		},
		playlist.to_str().unwrap_or_default(),
	)
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
/// Playlists separated by '\n'
pub extern "C" fn c_list_playlists(server: *const c_char) -> *mut c_char {
	let serv = unsafe { CStr::from_ptr(server) };
	let playlists = list_playlists(serv.to_str().unwrap_or_default());
	CString::new(match playlists {
		None => "".to_string(),
		Some(s) => s.join("\n"),
	})
	.unwrap()
	.into_raw()
}

#[no_mangle]
pub extern "C" fn c_toggle() {
	toggle()
}

#[no_mangle]
pub extern "C" fn c_stop() {
	stop()
}

#[no_mangle]
pub extern "C" fn c_get_state() -> c_char {
	let state = STATE.read().unwrap();
	*state as c_char
}

#[no_mangle]
pub extern "C" fn c_get_metadata_artist() -> *mut c_char {
	let md = MD.read().unwrap();
	let md = md.clone();
	CString::new(match md {
		Some(md) => md.artist,
		None => "".to_string(),
	})
	.unwrap()
	.into_raw()
}

#[no_mangle]
pub extern "C" fn c_get_metadata_album() -> *mut c_char {
	let md = MD.read().unwrap();
	let md = md.clone();
	CString::new(match md {
		Some(md) => md.album,
		None => "".to_string(),
	})
	.unwrap()
	.into_raw()
}

#[no_mangle]
pub extern "C" fn c_get_metadata_title() -> *mut c_char {
	let md = MD.read().unwrap();
	let md = md.clone();
	CString::new(match md {
		Some(md) => md.title,
		None => "".to_string(),
	})
	.unwrap()
	.into_raw()
}

#[no_mangle]
pub extern "C" fn c_get_metadata_length() -> c_float {
	let md = MD.read().unwrap();
	match md.as_ref() {
		Some(md) => md.track_length_secs as c_float + md.track_length_frac as c_float,
		None => 0.0,
	}
}

#[repr(C)]
pub struct CImageJpeg {
	pub length: u32,
	pub bytes: *mut u8,
}

/// # Safety
/// Manually deallocate returned memory after use
#[no_mangle]
pub unsafe extern "C" fn c_get_cover_jpeg() -> CImageJpeg {
	let md = MD.read().unwrap();
	if let Some(md) = md.as_ref() {
		if let Some(cov) = md.cover.as_ref() {
			//eprintln!("{} {:p}", *len, cov.as_ptr());
			let len = cov.len() as u32;
			//let b = Box::new(.as_slice());
			let clone = cov.clone();
			let ptr = clone.as_ptr() as *mut u8;
			std::mem::forget(clone);
			CImageJpeg {
				length: len,
				bytes: ptr,
			}
		} else {
			eprintln!("No cov");
			CImageJpeg {
				length: 0,
				bytes: std::ptr::null_mut(),
			}
		}
	} else {
		eprintln!("No md");
		CImageJpeg {
			length: 0,
			bytes: std::ptr::null_mut(),
		}
	}
}
/// # Safety
/// None
#[no_mangle]
pub unsafe extern "C" fn c_drop(ptr: *mut u8, count: usize) {
	std::alloc::dealloc(ptr, std::alloc::Layout::from_size_align(count, 1).unwrap());
}
