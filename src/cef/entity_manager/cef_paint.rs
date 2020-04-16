use super::ENTITIES;
use crate::cef::interface::*;
use classicube_sys::*;
use log::debug;

// use lazy_static::lazy_static;
// use std::sync::atomic::{AtomicBool, Ordering};

// Bad fix because trying to do CEF.with() when thread is shutting down
// panics.
// TODO you can use try_with!
// lazy_static! {
//     pub static ref CEF_CAN_DRAW: AtomicBool = AtomicBool::new(false);
// }

/// This gets called from cef browser's OnPaint
pub extern "C" fn cef_paint_callback(
    browser: RustRefBrowser,
    new_pixels: *const ::std::os::raw::c_void,
    new_width: ::std::os::raw::c_int,
    new_height: ::std::os::raw::c_int,
) {
    // {
    //     if !CEF_CAN_DRAW.load(Ordering::SeqCst) {
    //         debug!("can't draw!");
    //         return;
    //     }
    // }

    let id = browser.get_identifier();
    ENTITIES.with(|entities| {
        let entities = &mut *entities.borrow_mut();

        if let Some((_browser, entity)) = entities.get_mut(&id) {
            let part = Bitmap {
                Scan0: new_pixels as *mut _,
                Width: new_width as i32,
                Height: new_height as i32,
            };

            entity.update_texture(part);
        } else {
            debug!("no entity for browser {}!", id);
        }
    });
}
