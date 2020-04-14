use super::ENTITIES;
use classicube_helpers::detour::static_detour;
use classicube_sys::*;
use std::os::raw::{c_double, c_float};

static_detour! {
    static DETOUR: unsafe extern "C" fn(*mut Entity, c_double, c_float);
}

/// This is called when LocalPlayer_RenderModel is called.
fn hook(local_player_entity: *mut Entity, delta: c_double, t: c_float) {
    unsafe {
        DETOUR.call(local_player_entity, delta, t);
    }

    ENTITIES.with(|entities| {
        let entities = &mut *entities.borrow_mut();

        for (_browser, entity) in entities.values_mut() {
            let mut entity = entity.entity;

            // Entities.List[i]->VTABLE->RenderModel(Entities.List[i], delta, t);

            let v_table = unsafe { &*entity.VTABLE };
            let render_model = v_table.RenderModel.unwrap();
            unsafe {
                render_model(&mut entity, delta, t);
            }
        }
    });
}

pub struct RenderModelDetour {}

impl RenderModelDetour {
    pub fn new() -> Self {
        Self {}
    }

    pub fn initialize(&mut self) {
        unsafe {
            let me = &*Entities.List[ENTITIES_SELF_ID as usize];
            let v_table = &*me.VTABLE;
            let target = v_table.RenderModel.unwrap();

            DETOUR.initialize(target, hook).unwrap();

            DETOUR.enable().unwrap();
        }
    }

    pub fn shutdown(&mut self) {
        unsafe {
            let _ignore_error = DETOUR.disable();
        }
    }
}

impl Drop for RenderModelDetour {
    fn drop(&mut self) {
        self.shutdown();
    }
}
