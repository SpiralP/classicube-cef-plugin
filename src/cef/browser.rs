use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::CStr,
    os::raw::{c_char, c_int},
};

use classicube_helpers::WithInner;
use tracing::{debug, warn};

use super::{bindings::RustRect, CefEvent, CEF_DEFAULT_HEIGHT, CEF_DEFAULT_WIDTH, EVENT_QUEUE};
use crate::cef::RustRefBrowser;

// identifier, browser
thread_local!(
    pub static BROWSERS: RefCell<HashMap<c_int, RustRefBrowser>> = RefCell::default();
);

thread_local!(
    pub static BROWSER_SIZES: RefCell<HashMap<c_int, (u16, u16)>> = RefCell::default();
);

thread_local!(
    pub static ALLOW_INSECURE: RefCell<HashMap<c_int, bool>> = RefCell::default();
);

// OnAfterCreated
#[tracing::instrument(fields(browser = browser.get_identifier()))]
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
#[tracing::instrument(fields(browser = browser.get_identifier()))]
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
#[tracing::instrument(fields(browser = browser.get_identifier()))]
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
#[tracing::instrument(fields(browser = browser.get_identifier(), title_c_str))]
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

#[tracing::instrument(fields(browser = browser.get_identifier()))]
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
            width: *width as c_int,
            height: *height as c_int,
        }
    })
}

#[tracing::instrument(fields(browser = browser.get_identifier()))]
pub extern "C" fn on_certificate_error_callback(browser: RustRefBrowser) -> bool {
    let browser_id = browser.get_identifier();

    warn!("certificate error for browser {}", browser_id);

    ALLOW_INSECURE.with(move |cell| {
        let allow_insecure = &mut *cell.borrow_mut();

        allow_insecure.get(&browser_id).is_some_and(|allow| *allow)
    })
}
