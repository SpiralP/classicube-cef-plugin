#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use crate::error::*;
use log::debug;
use std::{
    ffi::{CStr, CString},
    os::raw::c_int,
};

#[no_mangle]
pub unsafe extern "C" fn rust_print(c_str: *const ::std::os::raw::c_char) {
    let s = CStr::from_ptr(c_str).to_string_lossy().to_string();

    debug!("{}", s);
}

fn to_result(n: c_int) -> Result<()> {
    if n == 0 {
        Ok(())
    } else {
        Err(ErrorKind::CefError(n).into())
    }
}

// types to mimic CefRefPtr's Release on drop
impl RustRefApp {
    pub fn create(callbacks: Callbacks) -> Self {
        unsafe { cef_interface_create_app(callbacks) }
    }

    pub fn initialize(&self) -> Result<()> {
        to_result(unsafe { cef_interface_initialize(self.get()) })
    }

    pub fn step() -> Result<()> {
        to_result(unsafe { cef_interface_step() })
    }

    pub fn shutdown(&self) -> Result<()> {
        to_result(unsafe { cef_interface_shutdown() })
    }

    fn get(&self) -> *mut MyApp {
        self.ptr
    }
}
impl Drop for RustRefApp {
    fn drop(&mut self) {
        to_result(unsafe { cef_interface_release_ref_app(self.get()) }).unwrap();
    }
}
impl Clone for RustRefApp {
    fn clone(&self) -> Self {
        unsafe { cef_interface_add_ref_app(self.get()) }
    }
}

impl RustRefClient {
    pub fn create_browser(&self, startup_url: String) -> Result<()> {
        let startup_url = CString::new(startup_url).unwrap();

        to_result(unsafe { cef_interface_create_browser(self.get(), startup_url.as_ptr()) })
    }

    fn get(&self) -> *mut MyClient {
        self.ptr
    }
}
impl Drop for RustRefClient {
    fn drop(&mut self) {
        to_result(unsafe { cef_interface_release_ref_client(self.get()) }).unwrap();
    }
}
impl Clone for RustRefClient {
    fn clone(&self) -> Self {
        unsafe { cef_interface_add_ref_client(self.get()) }
    }
}

impl RustRefBrowser {
    pub fn get_identifier(&self) -> c_int {
        unsafe { cef_interface_browser_get_identifier(self.get()) }
    }

    pub fn load_url(&self, url: String) -> Result<()> {
        let url = CString::new(url).unwrap();

        to_result(unsafe { cef_interface_browser_load_url(self.get(), url.as_ptr()) })
    }

    #[allow(dead_code)]
    pub fn execute_javascript(&self, code: String) -> Result<()> {
        let code = CString::new(code).unwrap();

        to_result(unsafe { cef_interface_browser_execute_javascript(self.get(), code.as_ptr()) })
    }

    pub fn send_click(&self, x: c_int, y: c_int) -> Result<()> {
        to_result(unsafe { cef_interface_browser_send_click(self.get(), x, y) })
    }

    pub fn send_text(&self, text: String) -> Result<()> {
        let text = CString::new(text).unwrap();
        to_result(unsafe { cef_interface_browser_send_text(self.get(), text.as_ptr()) })
    }

    pub fn reload(&self) -> Result<()> {
        to_result(unsafe { cef_interface_browser_reload(self.get()) })
    }

    pub fn close(&self) -> Result<()> {
        to_result(unsafe { cef_interface_browser_close(self.get()) })
    }

    fn get(&self) -> *mut CefBrowser {
        self.ptr
    }
}
impl Drop for RustRefBrowser {
    fn drop(&mut self) {
        to_result(unsafe { cef_interface_release_ref_browser(self.get()) }).unwrap();
    }
}
impl Clone for RustRefBrowser {
    fn clone(&self) -> Self {
        unsafe { cef_interface_add_ref_browser(self.get()) }
    }
}
