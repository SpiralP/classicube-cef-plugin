mod rust_option;

use std::{cell::Cell, ffi::CString, os::raw::c_char};

use classicube_sys::{cc_string, Options_Get, Options_Set, OwnedString, STRING_SIZE};

use self::rust_option::RustOption;

fn get<S: Into<Vec<u8>>>(key: S) -> Option<String> {
    let c_key = CString::new(key).unwrap();
    let c_default = CString::new("").unwrap();

    let mut buffer: [c_char; (STRING_SIZE as usize) + 1] = [0; (STRING_SIZE as usize) + 1];
    let mut cc_string_value = cc_string {
        buffer: buffer.as_mut_ptr(),
        capacity: STRING_SIZE as u16,
        length: 0,
    };

    unsafe {
        Options_Get(c_key.as_ptr(), &mut cc_string_value, c_default.as_ptr());
    }

    let string_value = cc_string_value.to_string();

    if string_value.is_empty() {
        None
    } else {
        Some(string_value)
    }
}

fn set<S: Into<Vec<u8>>>(key: S, value: String) {
    let c_key = CString::new(key).unwrap();

    let cc_string_value = OwnedString::new(value);

    unsafe {
        Options_Set(c_key.as_ptr(), cc_string_value.as_cc_string());
    }
}

macro_rules! option {
    ($name:expr, $default:expr, $type:ty) => {{
        thread_local!(
            static CACHE: Cell<Option<$type>> = Cell::new(None);
        );

        RustOption::new($name, $default, CACHE)
    }};
}

pub const MUTE_LOSE_FOCUS: RustOption<bool> = option!("cef-mute-lose-focus", true, bool);
pub const AUTOPLAY_MAP_THEMES: RustOption<bool> = option!("cef-autoplay-map-themes", true, bool);
pub const VOLUME: RustOption<f32> = option!("cef-volume", 1.0, f32);
pub const MAP_THEME_VOLUME: RustOption<f32> = option!("cef-map-theme-volume", 0.4, f32);
pub const FRAME_RATE: RustOption<u16> = option!("cef-frame-rate", 30, u16);
pub const SUBTITLES: RustOption<bool> = option!("cef-subtitles", true, bool);
