use crate::*;

use std::ffi::{c_char, c_float, c_ushort};
use std::ffi::{CStr, CString};

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn c_start(server: *const c_char) {
	let serv = unsafe { CStr::from_ptr(server) };
	run(match serv.to_str() {
		Ok(s) => s,
		_ => "",
	})
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
pub extern "C" fn c_get_state() -> c_ushort {
	let state = STATE.read().unwrap();
	*state as c_ushort
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
pub extern "C" fn c_get_metadata_length() -> *mut c_float {
	let md = MD.read().unwrap();
	match md.as_ref() {
		Some(md) => &mut (md.length as c_float / md.sample_rate as c_float),
		None => &mut 0.0,
	}
}
