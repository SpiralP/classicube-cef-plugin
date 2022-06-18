use std::os::raw::{c_int, c_void};

use classicube_sys::Bitmap;
use tracing::warn;

use super::EntityManager;
use crate::cef::RustRefBrowser;

/// This gets called from cef browser's OnPaint
#[tracing::instrument(fields(browser = browser.get_identifier(), new_pixels))]
pub extern "C" fn cef_paint_callback(
    browser: RustRefBrowser,
    new_pixels: *const c_void,
    new_width: c_int,
    new_height: c_int,
) {
    let browser_id = browser.get_identifier();

    if let Err(e) = EntityManager::with_by_browser_id(browser_id, |entity| {
        if entity.get_scale() != 0.0 {
            let part = Bitmap {
                scan0: new_pixels as *mut _,
                width: new_width as i32,
                height: new_height as i32,
            };

            entity.update_texture(part);
        }

        Ok(())
    }) {
        warn!("cef_paint_callback: {}", e);
    }
}
