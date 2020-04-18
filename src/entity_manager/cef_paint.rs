use super::EntityManager;
use crate::cef::RustRefBrowser;
use classicube_sys::*;
use log::warn;

/// This gets called from cef browser's OnPaint
pub extern "C" fn cef_paint_callback(
    browser: RustRefBrowser,
    new_pixels: *const ::std::os::raw::c_void,
    new_width: ::std::os::raw::c_int,
    new_height: ::std::os::raw::c_int,
) {
    let browser_id = browser.get_identifier();

    if let Err(e) = EntityManager::with_by_browser_id(browser_id, |entity| {
        let part = Bitmap {
            Scan0: new_pixels as *mut _,
            Width: new_width as i32,
            Height: new_height as i32,
        };

        entity.update_texture(part);

        Ok(())
    }) {
        warn!("cef_paint_callback: {}", e);
    }
}
