use super::ENTITIES;
use classicube_helpers::detour::static_detour;
use classicube_sys::*;
use std::os::raw::{c_double, c_float};

// TODO we can just replace the .RenderModel field!
// nvm, tried it and VTABLE is a *const and might be optimized strangely

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

        for entity in entities.values_mut() {
            entity.render_model();
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
