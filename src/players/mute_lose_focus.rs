use crate::options::get_mute_lose_focus;
use classicube_helpers::{events::window::FocusChangedEventHandler, CellGetSet};
use classicube_sys::WindowInfo;
use std::cell::{Cell, RefCell};

thread_local!(
    pub static IS_FOCUSED: Cell<bool> = Cell::new(true);
);

thread_local!(
    static FOCUS_HANDLER: RefCell<Option<FocusChangedEventHandler>> = Default::default();
);

pub fn initialize() {
    let mut focus_handler = FocusChangedEventHandler::new();

    focus_handler.on(|_| {
        if get_mute_lose_focus() {
            let focused = unsafe { WindowInfo.Focused } != 0;
            IS_FOCUSED.set(focused);
        }
    });

    FOCUS_HANDLER.with(|cell| {
        let cell = &mut *cell.borrow_mut();

        *cell = Some(focus_handler);
    });
}

pub fn shutdown() {
    FOCUS_HANDLER.with(|cell| {
        let cell = &mut *cell.borrow_mut();
        *cell = None;
    });
}
