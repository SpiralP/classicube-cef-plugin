use super::CEF;
use classicube_sys::*;
use std::os::raw::{c_double, c_float};

pub extern "C" fn local_player_render_model_hook(entity: *mut Entity, delta: c_double, t: c_float) {
    CEF.with(|option| {
        if let Some(cef) = &mut *option.borrow_mut() {
            unsafe {
                cef.local_player_render_model_detour.call(entity, delta, t);

                let entity = cef.entity.as_mut().unwrap();
                let entity = entity.as_mut().project();
                let entity = entity.entity;

                // Entities.List[i]->VTABLE->RenderModel(Entities.List[i], delta, t);
                let v_table = &*entity.VTABLE;
                let render_model = v_table.RenderModel.unwrap();
                render_model(entity, delta, t);
            }
        }
    });
}
