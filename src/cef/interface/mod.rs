mod bindings;

use self::bindings::*;
pub use self::bindings::{RustRefApp, RustRefBrowser, RustRefClient};
use crate::error::*;
use std::{ffi::CString, os::raw::c_int};

fn to_result(n: c_int) -> Result<()> {
    if n == 0 {
        Ok(())
    } else {
        Err(ErrorKind::CefError(n).into())
    }
}

// types to mimic CefRefPtr's Release on drop
impl RustRefApp {
    pub fn create(
        on_context_initialized_callback: OnContextInitializedCallback,
        on_before_close_callback: OnBeforeCloseCallback,
        on_paint_callback: OnPaintCallback,
        on_load_end_callback: OnLoadEndCallback,
    ) -> Self {
        unsafe {
            cef_interface_create_app(
                on_context_initialized_callback,
                on_before_close_callback,
                on_paint_callback,
                on_load_end_callback,
            )
        }
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
    pub fn create_browser(&self, startup_url: String) -> RustRefBrowser {
        let startup_url = CString::new(startup_url).unwrap();

        unsafe { cef_interface_create_browser(self.get(), startup_url.as_ptr()) }
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

    pub fn execute_javascript(&self, code: String) -> Result<()> {
        let code = CString::new(code).unwrap();

        to_result(unsafe { cef_interface_browser_execute_javascript(self.get(), code.as_ptr()) })
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
