use super::Chat;
use crate::{async_manager::AsyncManager, chat::TAB_LIST, plugin::APP_NAME};
use async_std::future;
use classicube_helpers::{detour::static_detour, tab_list::remove_color};
use classicube_sys::{Chat_AddOf, MsgType_MSG_TYPE_NORMAL, ENTITIES_SELF_ID};
use log::debug;
use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

static_detour! {
    static DETOUR: unsafe extern "C" fn(*const  classicube_sys:: String, ::std::os::raw::c_int);
}

thread_local!(
    static SIMULATING: Cell<bool> = Cell::new(false);
);

fn chat_add_hook(text: *const classicube_sys::String, message_type: ::std::os::raw::c_int) {
    if message_type == MsgType_MSG_TYPE_NORMAL
        && handle_chat_message(unsafe { (*text).to_string() })
    {
        return;
    }

    unsafe { DETOUR.call(text, message_type) }
}

pub fn initialize() {
    debug!("initialize hidden_communication");

    unsafe {
        DETOUR.initialize(Chat_AddOf, chat_add_hook).unwrap();
        DETOUR.enable().unwrap();
    }

    SIMULATING.with(|a| a.set(true));
    Chat::send("/clients");

    // &7Players using:
    // &7  ClassiCube 1.1.3: &f� saiko, \n doberman411, Fist,
    // > &fLemonLeman, � che, gemsgem, � xenon, Guzz

    // &7  ClassiCube 1.1.3 +cef0.2.0 + More Models v1.2.4 + Ponies
    // > &7v2.1: &f� Goodly
}

pub fn shutdown() {
    debug!("shutdown hidden_communication");

    unsafe {
        DETOUR.disable().unwrap();
    }
}

thread_local!(
    static LISTENING: Cell<bool> = Cell::new(false);
);

thread_local!(
    static MESSAGES: RefCell<Vec<String>> = RefCell::new(Vec::new());
);

#[must_use]
fn handle_chat_message(message: String) -> bool {
    if !SIMULATING.with(|a| a.get()) {
        return false;
    }

    if message == "&7Players using:" {
        LISTENING.with(|cell| cell.set(true));

        // give it a couple seconds before stop listening
        AsyncManager::spawn_local_on_main_thread(async {
            let _ = future::timeout(Duration::from_secs(2), future::pending::<()>()).await;

            if LISTENING.with(|cell| cell.get()) {
                debug!("stopping because of timer");
                LISTENING.with(|cell| cell.set(false));
                SIMULATING.with(|a| a.set(false));

                let messages = MESSAGES.with(|cell| {
                    let messages = &mut *cell.borrow_mut();
                    messages.drain(..).collect()
                });
                process_clients_response(messages);
            }
        });

        return true;
    }

    if !LISTENING.with(|cell| cell.get()) {
        return false;
    }

    if message.starts_with("&7  ") || message.starts_with("> &f") || message.starts_with("> &7") {
        // probably a /clients response
        debug!("{:?}", message);
        MESSAGES.with(|cell| {
            let messages = &mut *cell.borrow_mut();
            messages.push(message.to_string());
        });

        // don't show this message!
        return true;
    } else {
        debug!("stopping because of {:?}", message);
        LISTENING.with(|cell| cell.set(false));
        SIMULATING.with(|a| a.set(false));

        let messages = MESSAGES.with(|cell| {
            let messages = &mut *cell.borrow_mut();
            messages.drain(..).collect()
        });
        process_clients_response(messages);
    }

    false
}

fn process_clients_response(messages: Vec<String>) {
    debug!("{:#?}", messages);

    let mut full_lines = Vec::new();

    for message in &messages {
        if message.starts_with("&7  ") {
            // start of line

            // "&7  "
            full_lines.push(message[4..].to_string());
        } else {
            // a continuation message

            let last = full_lines.last_mut().unwrap();

            // "> &f" or "> &7"
            *last = format!("{} {}", last, message[2..].to_string());
        }
    }

    debug!("{:#?}", full_lines);

    let mut names_with_cef: Vec<String> = Vec::new();
    for message in &full_lines {
        let pos = message.find(": &f").unwrap();

        let (left, right) = message.split_at(pos);
        // skip ": &f"
        let right = &right[4..];

        let left = remove_color(left);
        let right = remove_color(right);

        let mut names: Vec<String> = right.split(", ").map(|a| a.to_string()).collect();

        if left.contains(APP_NAME) {
            names_with_cef.append(&mut names);
        }
    }

    debug!("{:#?}", names_with_cef);

    let player_ids_with_cef: Vec<u8> = names_with_cef
        .drain(..)
        .filter_map(|name| {
            TAB_LIST.with(|cell| {
                let tab_list = &*cell.borrow();
                let tab_list = tab_list.as_ref().unwrap();
                let id = tab_list.find_entry_by_nick_name(&name).unwrap().get_id();

                if id == ENTITIES_SELF_ID as u8 {
                    None
                } else {
                    Some(id)
                }
            })
        })
        .collect();

    debug!("{:#?}", player_ids_with_cef);

    if !player_ids_with_cef.is_empty() {
        let names: Vec<String> = player_ids_with_cef
            .iter()
            .map(|id| {
                TAB_LIST.with(|cell| {
                    let tab_list = &*cell.borrow();
                    let tab_list = tab_list.as_ref().unwrap();
                    let entry = tab_list.get(*id).unwrap();

                    entry.get_real_name().unwrap()
                })
            })
            .collect();

        Chat::print(format!("Other players with cef: {}", names.join(", ")));
    }
}
