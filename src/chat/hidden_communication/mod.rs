mod clients;
mod whispers;

use classicube_helpers::detour::static_detour;
use classicube_sys::{Chat_AddOf, MsgType_MSG_TYPE_NORMAL, Server};
use log::debug;
use std::os::raw::c_int;

static_detour! {
    static DETOUR: unsafe extern "C" fn(*const  classicube_sys:: String, c_int);
}

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

    // first map change, query
    clients::query_clients();
}

pub fn on_new_map_loaded() {
    // second etc map change
    clients::query_clients();
}

pub fn shutdown() {
    debug!("shutdown hidden_communication");

    unsafe {
        let _ignore_error = DETOUR.disable();
    }
}

#[must_use]
fn handle_chat_message(message: String) -> bool {
    let a = clients::handle_chat_message(&message);
    let b = whispers::handle_chat_message(&message);

    if a || b {
        return true;
    }

    false
}
