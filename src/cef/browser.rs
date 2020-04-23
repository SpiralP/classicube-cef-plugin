use super::{CefEvent, EVENT_QUEUE};
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

// OnAfterCreated
pub extern "C" fn on_after_created(browser: RustRefBrowser) {
    let id = browser.get_identifier();
    debug!("on_after_created {}", id);

    {
        let browser = browser.clone();
        EVENT_QUEUE
            .with_inner_mut(move |event_queue| {
                let _ignore_error = event_queue.send(CefEvent::BrowserCreated(browser));
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
        .with_inner_mut(move |event_queue| {
            let _ignore_error = event_queue.send(CefEvent::BrowserClosed(browser));
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
        .with_inner_mut(move |event_queue| {
            let _ignore_error = event_queue.send(CefEvent::BrowserPageLoaded(browser));
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
        .with_inner_mut(move |event_queue| {
            let _ignore_error = event_queue.send(CefEvent::BrowserTitleChange(browser, title));
        })
        .unwrap();
}
