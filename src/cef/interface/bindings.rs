#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[no_mangle]
pub unsafe extern "C" fn rust_print(c_str: *const ::std::os::raw::c_char) {
    use std::ffi::CStr;

    let s = CStr::from_ptr(c_str).to_string_lossy().to_string();

    println!("{}", s);
}
