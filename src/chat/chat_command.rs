use super::{commands, Chat};
use crate::{async_manager, chat::PlayerSnapshot};
use classicube_sys::{OwnedChatCommand, ENTITIES_SELF_ID};
use log::*;
use std::{os::raw::c_int, slice};

extern "C" fn c_chat_command_callback(args: *const classicube_sys::String, args_count: c_int) {
    let args = unsafe { slice::from_raw_parts(args, args_count as _) };
    let args: Vec<String> = args.iter().map(|cc_string| cc_string.to_string()).collect();

    let player_snapshot = PlayerSnapshot::from_entity_id(ENTITIES_SELF_ID as _).unwrap();

    async_manager::spawn_local_on_main_thread(async move {
        if let Err(e) = commands::run(player_snapshot, args, true).await {
            warn!("command error: {:#?}", e);
            Chat::print(format!(
                "{}cef command error: {}{}",
                classicube_helpers::color::RED,
                classicube_helpers::color::WHITE,
                e
            ));
        }
    });
}

pub struct CefChatCommand {
    chat_command: OwnedChatCommand,
}

impl CefChatCommand {
    pub fn new() -> Self {
        Self {
            chat_command: OwnedChatCommand::new("Cef", c_chat_command_callback, false, vec!["cef"]),
        }
    }

    pub fn initialize(&mut self) {
        self.chat_command.register();
    }

    pub fn shutdown(&mut self) {}
}
