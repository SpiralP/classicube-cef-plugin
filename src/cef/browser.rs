use super::{bindings::RustRect, CefEvent, CEF_DEFAULT_HEIGHT, CEF_DEFAULT_WIDTH, EVENT_QUEUE};
use crate::cef::RustRefBrowser;
use classicube_helpers::OptionWithInner;
use log::debug;
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::CStr,
    os::raw::{c_char, c_int},
};

// identifier, browser
thread_local!(
    pub static BROWSERS: RefCell<HashMap<c_int, RustRefBrowser>> = RefCell::new(HashMap::new());
);

thread_local!(
    pub static BROWSER_SIZES: RefCell<HashMap<c_int, (c_int, c_int)>> = Default::default();
);

// OnAfterCreated
pub extern "C" fn on_after_created(browser: RustRefBrowser) {
    let id = browser.get_identifier();
    debug!("on_after_created {}", id);

    {
        let browser = browser.clone();
        EVENT_QUEUE
            .with_inner_mut(move |(sender, _receiver)| {
                let _ignore_error = sender.send(CefEvent::BrowserCreated(browser));
            })
            .unwrap();
    }

    BROWSERS.with(move |cell| {
        let browsers = &mut *cell.borrow_mut();
        browsers.insert(id, browser);
    });
}

// OnBeforeClose
pub extern "C" fn on_before_close(browser: RustRefBrowser) {
    let id = browser.get_identifier();
    debug!("on_before_close {}", id);

    EVENT_QUEUE
        .with_inner_mut(move |(sender, _receiver)| {
            let _ignore_error = sender.send(CefEvent::BrowserClosed(browser));
        })
        .unwrap();

    BROWSERS.with(move |cell| {
        let browsers = &mut *cell.borrow_mut();
        browsers.remove(&id);
    });
}

// OnPageLoaded
pub extern "C" fn on_page_loaded(browser: RustRefBrowser) {
    let id = browser.get_identifier();
    debug!("on_page_loaded {}", id);

    EVENT_QUEUE
        .with_inner_mut(move |(sender, _receiver)| {
            let _ignore_error = sender.send(CefEvent::BrowserPageLoaded(browser));
        })
        .unwrap();
}

// OnTitleChange
pub extern "C" fn on_title_change(browser: RustRefBrowser, title_c_str: *const c_char) {
    let id = browser.get_identifier();
    let title = unsafe { CStr::from_ptr(title_c_str) }
        .to_string_lossy()
        .to_string();
    debug!("on_title_change {} {}", id, title);

    EVENT_QUEUE
        .with_inner_mut(move |(sender, _receiver)| {
            let _ignore_error = sender.send(CefEvent::BrowserTitleChange(browser, title));
        })
        .unwrap();
}

pub extern "C" fn get_view_rect(browser: RustRefBrowser) -> RustRect {
    let browser_id = browser.get_identifier();

    BROWSER_SIZES.with(move |cell| {
        let sizes = &mut *cell.borrow_mut();

        let (width, height) = sizes
            .get(&browser_id)
            .unwrap_or(&(CEF_DEFAULT_WIDTH, CEF_DEFAULT_HEIGHT));

        RustRect {
            x: 0,
            y: 0,
            width: *width,
            height: *height,
        }
    })
}

pub extern "C" fn on_certificate_error_callback(browser: RustRefBrowser) -> bool {
    // browser.
    false
}
