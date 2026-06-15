use std::cell::RefCell;

use classicube_helpers::local_player_vtable_hook::{
    LocalPlayerVTableHook, LocalPlayerVTableHooks, RenderModelFn,
};
use classicube_sys::Entity;

use super::ENTITIES;

thread_local!(
    static HOOK: RefCell<Option<LocalPlayerVTableHook>> = const { RefCell::new(None) };
);

fn render_model(local_player_entity: *mut Entity, delta: f32, t: f32, original: RenderModelFn) {
    unsafe {
        original(local_player_entity, delta, t);
    }

    ENTITIES.with(|entities| {
        let entities = &mut *entities.borrow_mut();

        for entity in entities.values_mut() {
            entity.render_model();
        }
    });
}

pub fn initialize() {
    HOOK.with(|cell| {
        let mut slot = cell.borrow_mut();
        let hook = LocalPlayerVTableHook::install(LocalPlayerVTableHooks {
            render_model: Some(Box::new(render_model)),
            ..Default::default()
        });
        *slot = Some(hook);
    });
}

pub fn shutdown() {
    HOOK.with(|cell| {
        cell.borrow_mut().take();
    });
}
