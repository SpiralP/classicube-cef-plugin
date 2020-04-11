mod bindings;

use self::bindings::*;
pub use self::bindings::{RustRefApp, RustRefBrowser, RustRefClient};
use std::{ffi::CString, os::raw::c_int};

#[derive(Debug)]
pub struct CefError {
    return_value: c_int,
}
impl From<c_int> for CefError {
    fn from(return_value: c_int) -> Self {
        Self { return_value }
    }
}

pub struct CefInterface;
impl CefInterface {
    pub fn step() -> CefResult<()> {
        to_result(unsafe { cef_interface_step() })
    }
}

pub type CefResult<T> = Result<T, CefError>;

fn to_result(n: c_int) -> CefResult<()> {
    if n == 0 {
        Ok(())
    } else {
        Err(CefError::from(n))
    }
}

// types to mimic CefRefPtr's Release on drop
impl RustRefApp {
    pub fn create(
        on_context_initialized_callback: OnContextInitializedCallback,
        on_after_created_callback: OnAfterCreatedCallback,
        on_before_close_callback: OnBeforeCloseCallback,
        on_paint_callback: OnPaintCallback,
    ) -> Self {
        unsafe {
            cef_interface_create_app(
                on_context_initialized_callback,
                on_after_created_callback,
                on_before_close_callback,
                on_paint_callback,
            )
        }
    }

    pub fn initialize(&mut self) -> CefResult<()> {
        to_result(unsafe { cef_interface_initialize(self.get()) })
    }

    fn get(&mut self) -> *mut MyApp {
        self.ptr
    }
}
impl Drop for RustRefApp {
    fn drop(&mut self) {
        println!("drop RustRefApp {:?}", self.get());
        to_result(unsafe { cef_interface_release_rust_ref_app(self.get()) }).unwrap();
    }
}

impl RustRefClient {
    pub fn create_browser(&mut self, startup_url: String) -> CefResult<()> {
        let startup_url = CString::new(startup_url).unwrap();

        to_result(unsafe { cef_interface_create_browser(self.get(), startup_url.as_ptr()) })
    }

    fn get(&mut self) -> *mut MyClient {
        self.ptr
    }
}
impl Drop for RustRefClient {
    fn drop(&mut self) {
        println!("drop RustRefClient {:?}", self.get());
        to_result(unsafe { cef_interface_release_rust_ref_client(self.get()) }).unwrap();
    }
}

impl RustRefBrowser {
    pub fn load_url(&mut self, url: String) -> CefResult<()> {
        let url = CString::new(url).unwrap();

        to_result(unsafe { cef_interface_browser_load_url(self.get(), url.as_ptr()) })
    }

    fn get(&mut self) -> *mut CefBrowser {
        self.ptr
    }
}
impl Drop for RustRefBrowser {
    fn drop(&mut self) {
        println!("drop RustRefBrowser {:?}", self.get());
        to_result(unsafe { cef_interface_release_rust_ref_browser(self.get()) }).unwrap();
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     fn print<T: Into<String>>(s: T) {
//         use std::io::Write;

//         let stdout = std::io::stdout();
//         let mut stdout = stdout.lock();
//         writeln!(stdout, "{}", s.into()).unwrap();
//     }

//     extern "C" fn on_paint_callback(
//         new_pixels: *const ::std::os::raw::c_void,
//         new_width: ::std::os::raw::c_int,
//         new_height: ::std::os::raw::c_int,
//     ) {
//         print(format!(
//             "paint {} {} {:?}",
//             new_width, new_height, new_pixels
//         ));
//     }

//     #[ignore]
//     #[test]
//     fn it_works() {
//         unsafe {
//             println!("cef_init");
//             assert_eq!(cef_init(Some(on_paint_callback)), 0);

//             fn run_script(code: String) {
//                 use std::ffi::CString;
//                 let c_str = CString::new(code).unwrap();
//                 unsafe {
//                     assert_eq!(cef_run_script(c_str.as_ptr()), 0);
//                 }
//             }

//             println!("loop");

//             for i in 0..200 {
//                 if i == 50 {
//                     run_script(format!("player.loadVideoById(\"{}\");", "gQngg8iQipk"));
//                 }
//                 assert_eq!(cef_step(), 0);
//                 std::thread::sleep(std::time::Duration::from_millis(20));
//             }

//             println!("cef_free");
//             assert_eq!(cef_free(), 0);
//         }
//     }
// }
