mod rust_option;

use self::rust_option::RustOption;
use classicube_sys::{Options_Get, Options_Set, OwnedString, STRING_SIZE};
use std::{ffi::CString, os::raw::c_char};

pub fn get<S: Into<Vec<u8>>>(key: S) -> Option<String> {
    let c_key = CString::new(key).unwrap();
    let c_default = CString::new("").unwrap();

    let mut buffer: [c_char; (STRING_SIZE as usize) + 1] = [0; (STRING_SIZE as usize) + 1];
    let mut cc_string_value = classicube_sys::String {
        buffer: buffer.as_mut_ptr(),
        capacity: STRING_SIZE as u16,
        length: 0,
    };

    unsafe {
        Options_Get(c_key.as_ptr(), &mut cc_string_value, c_default.as_ptr());
    }

    let string_value = cc_string_value.to_string();

    if string_value == "" {
        None
    } else {
        Some(string_value)
    }
}

pub fn set<S: Into<Vec<u8>>>(key: S, value: String) {
    let c_key = CString::new(key).unwrap();

    let cc_string_value = OwnedString::new(value);

    unsafe {
        Options_Set(c_key.as_ptr(), cc_string_value.as_cc_string());
    }
}

pub const MUTE_LOSE_FOCUS: RustOption<bool> = RustOption::new("cef-mute-lose-focus", "true");
pub const AUTOPLAY_MAP_THEMES: RustOption<bool> =
    RustOption::new("cef-autoplay-map-themes", "true");
pub const VOLUME: RustOption<f32> = RustOption::new("cef-volume", "1.0");
pub const FRAME_RATE: RustOption<u16> = RustOption::new("cef-frame-rate", "30");
pub const SUBTITLES: RustOption<bool> = RustOption::new("cef-subtitles", "true");
