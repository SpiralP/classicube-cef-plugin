use std::cell::{Cell, RefCell};

use classicube_helpers::events::window::FocusChangedEventHandler;
use classicube_sys::WindowInfo;

use crate::{cef::Cef, options::MUTE_LOSE_FOCUS};

thread_local!(
    pub static IS_FOCUSED: Cell<bool> = Cell::new(true);
);

thread_local!(
    static FOCUS_HANDLER: RefCell<Option<FocusChangedEventHandler>> = RefCell::default();
);

pub fn initialize() {
    let mut focus_handler = FocusChangedEventHandler::new();

    focus_handler.on(|_| {
        if MUTE_LOSE_FOCUS.get().unwrap() {
            let focused = unsafe { WindowInfo.Focused } != 0;
            IS_FOCUSED.set(focused);
            Cef::set_audio_muted_all(!focused);
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
