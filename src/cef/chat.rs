use super::chat_command::command_callback;
use classicube_sys::{Chat_Add, MsgType, MsgType_MSG_TYPE_NORMAL, OwnedString, Server};
use std::cell::{Cell, RefCell};

thread_local!(
    static LAST_CHAT: RefCell<Option<String>> = RefCell::new(None);
);

thread_local!(
    static SIMULATING: Cell<bool> = Cell::new(false);
);

// TODO make this an impl of Cef, also make command_callback a method!

pub fn handle_chat_received(message: String, message_type: MsgType) {
    if SIMULATING.with(|a| a.get()) {
        return;
    }
    if message_type != MsgType_MSG_TYPE_NORMAL {
        return;
    }

    if let Some((_name, message)) = get_player_name_and_message(message) {
        // let name: String = remove_color(name).trim().to_string();

        // don't remove colors because & might be part of url!
        // let message: String = remove_color(message).trim().to_string();

        let mut split = message
            .split(' ')
            .map(|a| a.to_string())
            .collect::<Vec<String>>();

        if split
            .get(0)
            .map(|first| remove_color(first).trim() == "cef")
            .unwrap_or(false)
        {
            split.remove(0);

            command_callback(split);
        }
    }
}

pub fn remove_color<T: AsRef<str>>(text: T) -> String {
    let mut found_ampersand = false;

    text.as_ref()
        .chars()
        .filter(|&c| {
            if c == '&' {
                // we remove all amps but they're kept in chat if repeated
                found_ampersand = true;
                false
            } else if found_ampersand {
                found_ampersand = false;
                false
            } else {
                true
            }
        })
        .collect()
}

fn get_player_name_and_message(mut full_msg: String) -> Option<(String, String)> {
    if unsafe { Server.IsSinglePlayer } != 0 {
        // in singleplayer there is no tab list, even self id infos are null

        return Some((String::new(), full_msg));
    }

    LAST_CHAT.with(|cell| {
        let mut last_chat = cell.borrow_mut();

        if !full_msg.starts_with("> &f") {
            *last_chat = Some(full_msg.clone());
        } else if let Some(chat_last) = &*last_chat {
            // we're a continue message
            full_msg = full_msg.split_off(4); // skip "> &f"

            // most likely there's a space
            // the server trims the first line :(
            // TODO try both messages? with and without the space?
            full_msg = format!("{}{}", chat_last, full_msg);
            *last_chat = Some(full_msg.clone());
        }

        // &]SpiralP: &faaa
        // let full_msg = full_msg.into();

        // nickname_resolver_handle_message(full_msg.to_string());

        // find colon from the left
        if let Some(pos) = full_msg.find(": ") {
            // &]SpiralP
            let left = &full_msg[..pos]; // left without colon
                                         // &faaa
            let right = &full_msg[(pos + 2)..]; // right without colon

            // TODO title is [ ] before nick, team is < > before nick, also there are rank
            // symbols? &f┬ &f♂&6 Goodly: &fhi

            let full_nick = left.to_string();
            let said_text = right.to_string();

            Some((full_nick, said_text))
        } else {
            None
        }
    })
}

pub fn print<S: Into<Vec<u8>>>(s: S) {
    let owned_string = OwnedString::new(s);

    SIMULATING.with(|a| a.set(true));
    unsafe {
        Chat_Add(owned_string.as_cc_string());
    }
    SIMULATING.with(|a| a.set(false));
}
