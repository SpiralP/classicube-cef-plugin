#![allow(clippy::missing_safety_doc)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[cfg(target_os = "windows")]
#[link(name = "User32", kind = "dylib")]
extern "C" {}

#[cfg(target_os = "windows")]
#[cfg(debug_assertions)]
#[link(name = "ucrtd", kind = "static")]
extern "C" {}

#[cfg(target_os = "linux")]
#[link(name = "stdc++", kind = "static")]
extern "C" {}

// link to cef_interface

#[cfg(target_os = "windows")]
#[link(name = "libcef_dll_wrapper", kind = "dylib")]
extern "C" {}

#[cfg(not(target_os = "windows"))]
#[link(name = "cef_dll_wrapper", kind = "dylib")]
extern "C" {}

// link to cef_interface

#[link(name = "cef_interface", kind = "static")]
extern "C" {}

// link to libcef

#[cfg(target_os = "windows")]
#[link(name = "libcef", kind = "dylib")]
extern "C" {}

#[cfg(target_os = "linux")]
#[link(name = "cef", kind = "dylib")]
extern "C" {}

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::{
    env,
    ffi::{CStr, CString},
    os::raw::{c_char, c_int},
};

#[no_mangle]
pub unsafe extern "C" fn rust_print(c_str: *const c_char) {
    let s = CStr::from_ptr(c_str).to_string_lossy().to_string();

    println!("{}", s);
}

fn main() {
    unsafe {
        // int argc, char* argv[]

        let mut arg_v = env::args()
            .map(|s| CString::new(s).unwrap().into_raw())
            .collect::<Vec<*mut c_char>>();
        let arg_c = arg_v.len();

        assert_eq!(
            cef_interface_execute_process(arg_c as c_int, arg_v.as_mut_ptr()),
            0
        );

        for ptr in arg_v.drain(..) {
            CString::from_raw(ptr);
        }
    }
}
