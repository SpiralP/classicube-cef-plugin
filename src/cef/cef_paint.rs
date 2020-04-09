use super::CEF;
use classicube_sys::*;

pub extern "C" fn cef_paint_callback(
    new_pixels: *const ::std::os::raw::c_void,
    new_width: ::std::os::raw::c_int,
    new_height: ::std::os::raw::c_int,
) {
    CEF.with(|option| {
        if let Some(cef) = &mut *option.borrow_mut() {
            if let Some(model) = cef.model.as_mut() {
                let mut part = Bitmap {
                    Scan0: new_pixels as *mut _,
                    Width: new_width as i32,
                    Height: new_height as i32,
                };

                unsafe {
                    println!("cef paint");
                    let texture = model.texture.as_ref().unwrap();
                    Gfx_UpdateTexturePart(texture.resource_id, 0, 0, &mut part, 0);
                }
            }
        }
    });
}
