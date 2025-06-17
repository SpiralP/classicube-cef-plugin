mod generated;

use std::{
    env,
    ffi::{CStr, CString},
    fs, io,
    iter::Iterator,
    mem,
    os::raw::c_int,
    process, ptr, slice,
    time::{SystemTime, UNIX_EPOCH},
};

use tracing::{debug, warn};
use url::Url;

pub use self::generated::*;
use super::{javascript, javascript::RustV8Value};
use crate::error::{bail, ErrorKind, Result, ResultExt};

#[no_mangle]
pub unsafe extern "C" fn rust_debug(c_str: *const ::std::os::raw::c_char) {
    let s = CStr::from_ptr(c_str).to_string_lossy().to_string();

    debug!("{}", s);
}

#[no_mangle]
pub unsafe extern "C" fn rust_warn(c_str: *const ::std::os::raw::c_char) {
    let s = CStr::from_ptr(c_str).to_string_lossy().to_string();

    warn!("{}", s);
}

const YOUTUBE_HTML: &[u8] = include_bytes!("../../player/youtube/page.html");
const MEDIA_HTML: &[u8] = include_bytes!("../../player/media/page.html");

fn handle_scheme_create(
    _browser: RustRefBrowser,
    _scheme_name: *const ::std::os::raw::c_char,
    url: *const ::std::os::raw::c_char,
) -> Result<&'static [u8]> {
    // let scheme_name = unsafe { CStr::from_ptr(scheme_name) }.to_str()?;
    let url = unsafe { CStr::from_ptr(url) }.to_str()?;
    let url = Url::parse(url)?;
    let host = url.host_str().chain_err(|| "no host part on url")?;

    debug!("rust_handle_scheme_create {:?}", host);

    match host {
        "youtube" => Ok(YOUTUBE_HTML),
        "media" => Ok(MEDIA_HTML),

        _ => {
            bail!("no such local scheme for {:?}", host);
        }
    }
}

#[no_mangle]
pub extern "C" fn rust_handle_scheme_create(
    browser: RustRefBrowser,
    scheme_name: *const ::std::os::raw::c_char,
    url: *const ::std::os::raw::c_char,
) -> RustSchemeReturn {
    match handle_scheme_create(browser, scheme_name, url) {
        Ok(data) => RustSchemeReturn {
            data: data.as_ptr() as *mut std::os::raw::c_void,
            data_size: data.len() as _,
            mime_type: c"text/html".as_ptr().cast_mut(),
        },

        Err(e) => {
            warn!("{}", e);

            RustSchemeReturn {
                data: ptr::null_mut(),
                data_size: 0,
                mime_type: ptr::null_mut(),
            }
        }
    }
}

// #[no_mangle]
// pub unsafe extern "C" fn rust_wprint(c_str: *const u16) {
//     use widestring::WideCStr;

//     let string = WideCStr::from_ptr_str(c_str);

//     debug!("{}", string.to_string().unwrap());
// }

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
        let current_dir_path = env::current_dir().chain_err(|| "current_dir() None")?;
        let cef_dir_path = current_dir_path.join("cef");

        #[cfg(target_os = "windows")]
        let browser_subprocess_path = cef_dir_path.join("cef.exe");
        #[cfg(target_os = "macos")]
        let browser_subprocess_path = cef_dir_path
            .join("cef.app")
            .join("Contents")
            .join("MacOS")
            .join("cef");
        #[cfg(target_os = "linux")]
        let browser_subprocess_path = cef_dir_path.join("cef");

        let root_cache_path = {
            let cache_dir = cef_dir_path.join("cache");
            fs::create_dir_all(&cache_dir).chain_err(|| "create cache dir")?;

            let mut dirs = fs::read_dir(&cache_dir)
                .and_then(Iterator::collect::<io::Result<Vec<_>>>)
                .map(|dirs| {
                    dirs.into_iter()
                        .filter(|dir| dir.path().is_dir())
                        .map(|dir| dir.path())
                        .collect::<Vec<_>>()
                })
                .chain_err(|| "read_dir cache_dir")?;

            // sort by oldest first
            dirs.sort();
            debug!(?dirs);

            while dirs.len() >= 10 {
                let dir = dirs.remove(0);
                debug!("removing old cache dir: {}", dir.display());
                fs::remove_dir_all(&dir)
                    .chain_err(|| format!("remove_dir_all: {}", dir.display()))?;
            }

            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .chain_err(|| "get current time")?;
            let epoch = current_time.as_millis();
            let pid = process::id();
            cache_dir.join(format!("{epoch}-{pid}"))
        };

        // ClassiCube doesn't need to run as an app bundle, but
        // this has to be set or else we get errors, although it seems to work with any non-empty string
        // ERROR:mach_port_rendezvous.cc(380)] bootstrap_look_up com.classicube.game.cef.MachPortRendezvousServer.16540: Unknown service name (1102)
        // ERROR:shared_memory_switch.cc(237)] No rendezvous client, terminating process (parent died?)
        let main_bundle_path = current_dir_path.clone();
        let framework_dir_path = cef_dir_path.join("Chromium Embedded Framework.framework");

        #[cfg(not(target_os = "macos"))]
        let resources_dir_path = cef_dir_path.join("cef_binary");
        #[cfg(target_os = "macos")]
        let resources_dir_path = framework_dir_path.join("Resources");

        #[cfg(not(target_os = "macos"))]
        let locales_dir_path = cef_dir_path.join("cef_binary").join("locales");
        // This value is ignored on MacOS
        #[cfg(target_os = "macos")]
        let locales_dir_path = std::path::PathBuf::new();

        let browser_subprocess_path =
            CString::new(format!("{}", browser_subprocess_path.display()))?;
        let root_cache_path = CString::new(format!("{}", root_cache_path.display()))?;

        let main_bundle_path = CString::new(format!("{}", main_bundle_path.display()))?;
        let framework_dir_path = CString::new(format!("{}", framework_dir_path.display()))?;

        let resources_dir_path = CString::new(format!("{}", resources_dir_path.display()))?;
        let locales_dir_path = CString::new(format!("{}", locales_dir_path.display()))?;

        let paths = CefInitializePaths {
            browser_subprocess_path: browser_subprocess_path.as_ptr(),
            root_cache_path: root_cache_path.as_ptr(),

            resources_dir_path: resources_dir_path.as_ptr(),
            locales_dir_path: locales_dir_path.as_ptr(),

            main_bundle_path: main_bundle_path.as_ptr(),
            framework_dir_path: framework_dir_path.as_ptr(),
        };

        to_result(unsafe { cef_interface_initialize(self.ptr, paths) })
    }

    pub fn step() -> Result<()> {
        to_result(unsafe { cef_interface_step() })
    }

    pub fn shutdown(&self) -> Result<()> {
        to_result(unsafe { cef_interface_shutdown() })
    }
}
impl Drop for RustRefApp {
    fn drop(&mut self) {
        to_result(unsafe { cef_interface_release_ref_app(self.ptr) }).unwrap();
    }
}
impl Clone for RustRefApp {
    fn clone(&self) -> Self {
        unsafe { cef_interface_add_ref_app(self.ptr) }
    }
}

impl RustRefClient {
    pub fn create_browser<T: Into<Vec<u8>>>(
        &self,
        startup_url: T,
        fps: c_int,
        insecure: bool,
        background_color: u32,
    ) -> Result<()> {
        let startup_url = CString::new(startup_url)?;

        to_result(unsafe {
            cef_interface_create_browser(
                self.ptr,
                startup_url.as_ptr(),
                fps,
                insecure,
                background_color,
            )
        })
    }
}
impl Drop for RustRefClient {
    fn drop(&mut self) {
        to_result(unsafe { cef_interface_release_ref_client(self.ptr) }).unwrap();
    }
}
impl Clone for RustRefClient {
    fn clone(&self) -> Self {
        unsafe { cef_interface_add_ref_client(self.ptr) }
    }
}

impl RustRefBrowser {
    pub fn get_identifier(&self) -> c_int {
        unsafe { cef_interface_browser_get_identifier(self.ptr) }
    }

    pub fn load_url<T: Into<Vec<u8>>>(&self, url: T) -> Result<()> {
        let url = CString::new(url)?;

        to_result(unsafe { cef_interface_browser_load_url(self.ptr, url.as_ptr()) })
    }

    pub fn execute_javascript<T: Into<Vec<u8>>>(&self, code: T) -> Result<()> {
        let code = CString::new(code)?;

        to_result(unsafe { cef_interface_browser_execute_javascript(self.ptr, code.as_ptr()) })
    }

    pub fn execute_javascript_on_frame<T: Into<Vec<u8>>, U: Into<Vec<u8>>>(
        &self,
        frame_name: T,
        code: U,
    ) -> Result<()> {
        let frame_name = CString::new(frame_name)?;
        let code = CString::new(code)?;

        to_result(unsafe {
            cef_interface_browser_execute_javascript_on_frame(
                self.ptr,
                frame_name.as_ptr(),
                code.as_ptr(),
            )
        })
    }

    pub async fn eval_javascript<T: Into<Vec<u8>>>(&self, code: T) -> Result<RustV8Value> {
        let code = CString::new(code)?;

        let (receiver, task_id) = javascript::create_task();

        to_result(unsafe {
            cef_interface_browser_eval_javascript(self.ptr, task_id, code.as_ptr())
        })?;

        let response = receiver.await?;

        if response.success {
            let ffi_v8_value = unsafe { response.__bindgen_anon_1.result.as_ref() };
            let v8_value = ffi_v8_value.to_v8_value();

            Ok(v8_value)
        } else {
            Err("javascript error".into())
        }
    }

    #[allow(dead_code)]
    pub async fn eval_javascript_on_frame<T: Into<Vec<u8>>, U: Into<Vec<u8>>>(
        &self,
        frame_name: T,
        code: U,
    ) -> Result<RustV8Value> {
        let frame_name = CString::new(frame_name)?;
        let code = CString::new(code)?;

        let (receiver, task_id) = javascript::create_task();

        to_result(unsafe {
            cef_interface_browser_eval_javascript_on_frame(
                self.ptr,
                frame_name.as_ptr(),
                task_id,
                code.as_ptr(),
            )
        })?;

        let response = receiver.await?;

        if response.success {
            let ffi_v8_value = unsafe { response.__bindgen_anon_1.result.as_ref() };
            let v8_value = ffi_v8_value.to_v8_value();

            Ok(v8_value)
        } else {
            Err("javascript error".into())
        }
    }

    pub fn send_click(&self, x: c_int, y: c_int) -> Result<()> {
        to_result(unsafe { cef_interface_browser_send_click(self.ptr, x, y) })
    }

    pub fn send_text<T: Into<Vec<u8>>>(&self, text: T) -> Result<()> {
        let text = CString::new(text)?;
        to_result(unsafe { cef_interface_browser_send_text(self.ptr, text.as_ptr()) })
    }

    pub fn reload(&self) -> Result<()> {
        to_result(unsafe { cef_interface_browser_reload(self.ptr) })
    }

    pub fn was_resized(&self) -> Result<()> {
        to_result(unsafe { cef_interface_browser_was_resized(self.ptr) })
    }

    pub fn open_dev_tools(&self) -> Result<()> {
        to_result(unsafe { cef_interface_browser_open_dev_tools(self.ptr) })
    }

    pub fn set_audio_muted(&self, mute: bool) -> Result<()> {
        to_result(unsafe { cef_interface_browser_set_audio_muted(self.ptr, mute) })
    }

    pub fn close(&self) -> Result<()> {
        to_result(unsafe { cef_interface_browser_close(self.ptr) })
    }
}
impl Drop for RustRefBrowser {
    fn drop(&mut self) {
        to_result(unsafe { cef_interface_release_ref_browser(self.ptr) }).unwrap();
    }
}
impl Clone for RustRefBrowser {
    fn clone(&self) -> Self {
        unsafe { cef_interface_add_ref_browser(self.ptr) }
    }
}

impl ToString for RustRefString {
    fn to_string(&self) -> String {
        let s = unsafe { slice::from_raw_parts(self.ptr.cast::<u8>(), self.len) };
        String::from_utf8_lossy(s).into_owned()
    }
}

impl Drop for RustRefString {
    fn drop(&mut self) {
        to_result(unsafe { cef_interface_delete_ref_string(self.ptr) }).unwrap();
    }
}
impl Clone for RustRefString {
    fn clone(&self) -> Self {
        unsafe { cef_interface_new_ref_string(self.ptr, self.len) }
    }
}

impl FFIRustV8Value {
    #[must_use]
    pub fn to_v8_value(&self) -> RustV8Value {
        let inner = &self.__bindgen_anon_1;

        unsafe {
            match self.tag {
                FFIRustV8ValueTag::Unknown => RustV8Value::Unknown,
                FFIRustV8ValueTag::Array => RustV8Value::Array,
                FFIRustV8ValueTag::ArrayBuffer => RustV8Value::ArrayBuffer,
                FFIRustV8ValueTag::Bool => RustV8Value::Bool(*inner.bool_.as_ref()),
                FFIRustV8ValueTag::Date => RustV8Value::Date,
                FFIRustV8ValueTag::Double => RustV8Value::Double(*inner.double_.as_ref()),
                FFIRustV8ValueTag::Function => RustV8Value::Function,
                FFIRustV8ValueTag::Int => RustV8Value::Int(*inner.int_.as_ref()),
                FFIRustV8ValueTag::Null => RustV8Value::Null,
                FFIRustV8ValueTag::Object => RustV8Value::Object,
                FFIRustV8ValueTag::String => RustV8Value::String(inner.string.as_ref().to_string()),
                FFIRustV8ValueTag::UInt => RustV8Value::UInt(*inner.uint.as_ref()),
                FFIRustV8ValueTag::Undefined => RustV8Value::Undefined,
            }
        }
    }
}
impl Drop for FFIRustV8Value {
    fn drop(&mut self) {
        unsafe {
            let inner = &mut self.__bindgen_anon_1;

            // hack to make sure the union fields call our drop
            match self.tag {
                FFIRustV8ValueTag::Bool => mem::swap(inner.bool_.as_mut(), &mut mem::zeroed()),
                FFIRustV8ValueTag::Double => mem::swap(inner.double_.as_mut(), &mut mem::zeroed()),
                FFIRustV8ValueTag::Int => mem::swap(inner.int_.as_mut(), &mut mem::zeroed()),
                FFIRustV8ValueTag::String => mem::swap(inner.string.as_mut(), &mut mem::zeroed()),
                FFIRustV8ValueTag::UInt => mem::swap(inner.uint.as_mut(), &mut mem::zeroed()),
                FFIRustV8ValueTag::Unknown
                | FFIRustV8ValueTag::Array
                | FFIRustV8ValueTag::ArrayBuffer
                | FFIRustV8ValueTag::Date
                | FFIRustV8ValueTag::Function
                | FFIRustV8ValueTag::Null
                | FFIRustV8ValueTag::Object
                | FFIRustV8ValueTag::Undefined => {}
            }
        }
    }
}

impl Drop for FFIRustV8Response {
    fn drop(&mut self) {
        unsafe {
            if self.success {
                mem::swap(self.__bindgen_anon_1.result.as_mut(), &mut mem::zeroed());
            } else {
                mem::swap(self.__bindgen_anon_1.error.as_mut(), &mut mem::zeroed());
            }
        }
    }
}
