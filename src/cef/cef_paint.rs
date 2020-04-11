use super::{interface::*, CEF};
use crate::helpers::RefCellOptionUnwrap;
use classicube_sys::*;
use lazy_static::lazy_static;
use std::sync::atomic::{AtomicBool, Ordering};

// Bad fix because trying to do CEF.with() when thread is shutting down
// panics.
lazy_static! {
    pub static ref CEF_CAN_DRAW: AtomicBool = AtomicBool::new(false);
}

/// This gets called from cef browser's OnPaint
pub extern "C" fn cef_paint_callback(
    _browser: RustRefBrowser,
    new_pixels: *const ::std::os::raw::c_void,
    new_width: ::std::os::raw::c_int,
    new_height: ::std::os::raw::c_int,
) {
    {
        if !CEF_CAN_DRAW.load(Ordering::SeqCst) {
            println!("can't draw!");
            return;
        }
    }

    CEF.with_inner_mut(|cef| {
        // println!("cef paint");
        if let Some(model) = cef.model.as_mut() {
            if let Some(texture) = &model.texture {
                let mut part = Bitmap {
                    Scan0: new_pixels as *mut _,
                    Width: new_width as i32,
                    Height: new_height as i32,
                };

                unsafe {
                    Gfx_UpdateTexturePart(texture.resource_id, 0, 0, &mut part, 0);
                }
            }
        }
    })
    .unwrap();
}
