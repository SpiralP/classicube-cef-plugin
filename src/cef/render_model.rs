use super::{cef_paint::CEF_CAN_DRAW, CEF};
use crate::helpers::WithInner;
use classicube_sys::*;
use std::{
    os::raw::{c_double, c_float},
    sync::atomic::Ordering,
};

/// This is called when LocalPlayer_RenderModel is called.
pub extern "C" fn local_player_render_model_hook(entity: *mut Entity, delta: c_double, t: c_float) {
    CEF.with_inner_mut(|cef| unsafe {
        cef.local_player_render_model_detour.call(entity, delta, t);
    })
    .unwrap();

    {
        if !CEF_CAN_DRAW.load(Ordering::SeqCst) {
            println!("can't render!");
            return;
        }
    }

    CEF.with_inner_mut(|cef| {
        unsafe {
            let entity = cef.entity.as_mut().unwrap();
            let entity = entity.as_mut().project();
            let entity = entity.entity;

            // Entities.List[i]->VTABLE->RenderModel(Entities.List[i], delta, t);
            let v_table = &*entity.VTABLE;
            let render_model = v_table.RenderModel.unwrap();
            render_model(entity, delta, t);
        }
    })
    .unwrap();
}
