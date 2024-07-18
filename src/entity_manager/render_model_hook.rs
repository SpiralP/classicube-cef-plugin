use std::{cell::Cell, os::raw::c_float, pin::Pin};

use classicube_sys::{Entities, Entity, EntityVTABLE, ENTITIES_SELF_ID};

use super::ENTITIES;

thread_local!(
    static ORIGINAL_FN: Cell<Option<unsafe extern "C" fn(*mut Entity, c_float, c_float)>> =
        Cell::new(None);
);

thread_local!(
    static VTABLE: Cell<Option<Pin<Box<EntityVTABLE>>>> = const { Cell::new(None) };
);

/// This is called when `LocalPlayer_RenderModel` is called.
extern "C" fn hook(local_player_entity: *mut Entity, delta: c_float, t: c_float) {
    ORIGINAL_FN.with(|cell| {
        if let Some(f) = cell.get() {
            unsafe {
                f(local_player_entity, delta, t);
            }
        }
    });

    ENTITIES.with(|entities| {
        let entities = &mut *entities.borrow_mut();

        for entity in entities.values_mut() {
            entity.render_model();
        }
    });
}

pub fn initialize() {
    let me = unsafe { &mut *Entities.List[ENTITIES_SELF_ID as usize] };
    let v_table = unsafe { &*me.VTABLE };

    ORIGINAL_FN.with(|cell| {
        cell.set(v_table.RenderModel);
    });

    let mut new_v_table = EntityVTABLE {
        Tick: v_table.Tick,
        Despawn: v_table.Despawn,
        SetLocation: v_table.SetLocation,
        GetCol: v_table.GetCol,
        RenderModel: v_table.RenderModel,
        ShouldRenderName: v_table.ShouldRenderName,
    };

    new_v_table.RenderModel = Some(hook);

    let new_v_table = Box::pin(new_v_table);
    me.VTABLE = new_v_table.as_ref().get_ref();

    VTABLE.with(|cell| {
        cell.set(Some(new_v_table));
    });
}

pub fn shutdown() {
    // self entity doesn't exist anymore so don't do anything
}
