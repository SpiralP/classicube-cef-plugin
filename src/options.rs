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

/// defaults to true
pub const MUTE_LOSE_FOCUS_OPTION: &str = "cef-mute-lose-focus";

pub fn get_mute_lose_focus() -> bool {
    get(MUTE_LOSE_FOCUS_OPTION)
        .and_then(|o| o.parse().ok())
        .unwrap_or(true)
}

pub fn set_mute_lose_focus(option: bool) {
    set(MUTE_LOSE_FOCUS_OPTION, format!("{}", option))
}

/// defaults to true
pub const AUTOPLAY_MAP_THEMES: &str = "cef-autoplay-map-themes";

pub fn get_autoplay_map_themes() -> bool {
    get(AUTOPLAY_MAP_THEMES)
        .and_then(|o| o.parse().ok())
        .unwrap_or(true)
}

pub fn set_autoplay_map_themes(option: bool) {
    set(AUTOPLAY_MAP_THEMES, format!("{}", option))
}

/// defaults to true
pub const MAP_THEME_VOLUME: &str = "cef-map-theme-volume";

pub fn get_map_theme_volume() -> f32 {
    get(MAP_THEME_VOLUME)
        .and_then(|o| o.parse().ok())
        .unwrap_or(0.5)
}

pub fn set_map_theme_volume(option: f32) {
    set(MAP_THEME_VOLUME, format!("{}", option))
}
