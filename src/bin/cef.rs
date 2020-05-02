#![allow(clippy::missing_safety_doc)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[link(name = "User32", kind = "dylib")]
extern "C" {}

#[link(name = "ucrtd", kind = "static")]
extern "C" {}

#[link(name = "libcef_dll_wrapper", kind = "dylib")]
extern "C" {}

#[link(name = "libcef", kind = "dylib")]
extern "C" {}

#[link(name = "cef_interface", kind = "static")]
extern "C" {}

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::{ffi::CStr, os::raw::c_char};

#[no_mangle]
pub unsafe extern "C" fn rust_print(c_str: *const c_char) {
    let s = CStr::from_ptr(c_str).to_string_lossy().to_string();

    println!("{}", s);
}

fn main() {
    unsafe {
        assert_eq!(cef_interface_execute_process(), 0);
    }
}
