mod clients;
mod encoding;
mod map_themes;
mod whispers;

pub use self::{encoding::LightEntity, map_themes::CURRENT_MAP_THEME};
use super::SIMULATING;
use crate::async_manager;
use classicube_helpers::{detour::static_detour, CellGetSet};
use classicube_sys::{Chat_AddOf, MsgType_MSG_TYPE_NORMAL, Server};
use futures::channel::oneshot;
use log::debug;
use std::{
    cell::{Cell, RefCell},
    os::raw::c_int,
};

static_detour! {
    static DETOUR: unsafe extern "C" fn(*const  classicube_sys:: String, c_int);
}

thread_local!(
    static SHOULD_BLOCK: Cell<bool> = Cell::new(false);
);

thread_local!(
    static WAITING_FOR_MESSAGE: RefCell<Vec<oneshot::Sender<String>>> = Default::default();
);

fn chat_add_hook(text: *const classicube_sys::String, message_type: c_int) {
    if message_type == MsgType_MSG_TYPE_NORMAL as c_int
        && handle_chat_message(unsafe { (*text).to_string() })
    {
        return;
    }

    unsafe { DETOUR.call(text, message_type) }
}

pub fn initialize() {
    debug!("initialize hidden_communication");

    if unsafe { Server.IsSinglePlayer } != 0 {
        return;
    }

    unsafe {
        DETOUR.initialize(Chat_AddOf, chat_add_hook).unwrap();
        DETOUR.enable().unwrap();
    }

    whispers::start_listening();
    map_themes::start_listening();
}

pub fn on_new_map() {
    if unsafe { Server.IsSinglePlayer } == 0 {
        clients::stop_query();
    }
}

pub fn on_new_map_loaded() {
    if unsafe { Server.IsSinglePlayer } == 0 {
        clients::query();
        map_themes::on_new_map_loaded();
    }
}

pub fn shutdown() {
    debug!("shutdown hidden_communication");

    map_themes::stop_listening();
    whispers::stop_listening();

    unsafe {
        let _ignore_error = DETOUR.disable();
    }
}

pub async fn wait_for_message() -> String {
    let (sender, receiver) = oneshot::channel();

    WAITING_FOR_MESSAGE.with(|cell| {
        let waiting = &mut cell.borrow_mut();
        waiting.push(sender);
    });

    // unwrap ok because we don't drop the sender before send()
    receiver.await.unwrap()
}

#[must_use]
fn handle_chat_message(message: String) -> bool {
    // don't recurse from Chat::send()
    if SIMULATING.get() {
        return false;
    }

    // resolve the awaits here so that our very next step() will trigger that code section
    let waiting: Vec<_> = WAITING_FOR_MESSAGE.with(|cell| {
        let waiting = &mut cell.borrow_mut();
        waiting.drain(..).collect()
    });

    for sender in waiting {
        let _ignore_error = sender.send(message.to_string());
    }

    async_manager::step();

    // check SHOULD_BLOCK to see if any futures said to block
    let should_block = SHOULD_BLOCK.get();
    SHOULD_BLOCK.set(false);

    should_block
}
