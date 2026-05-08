use std::{cell::RefCell, os::raw::c_int, slice};

use classicube_helpers::async_manager;
use classicube_sys::{ENTITIES_SELF_ID, OwnedChatCommand, cc_string};
use tracing::warn;

use super::{Chat, commands};
use crate::{chat::PlayerSnapshot, plugin};

thread_local!(
    // ClassiCube has no `Commands_Unregister`, so we register exactly once
    // per process and never drop the OwnedChatCommand. Dropping it would
    // free heap memory still referenced by the global cmds_head linked list.
    static COMMAND: RefCell<Option<OwnedChatCommand>> = const { RefCell::new(None) };
);

extern "C" fn c_chat_command_callback(args: *const cc_string, args_count: c_int) {
    if !plugin::is_plugin_active() {
        return;
    }

    let args = unsafe { slice::from_raw_parts(args, args_count.unsigned_abs() as _) };
    let args: Vec<String> = args.iter().map(ToString::to_string).collect();

    let Some(player_snapshot) = PlayerSnapshot::from_entity_id(ENTITIES_SELF_ID as _) else {
        return;
    };

    async_manager::spawn_local_on_main_thread(async move {
        if let Err(e) = commands::run(player_snapshot, args, true, true).await {
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

pub fn initialize() {
    COMMAND.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_some() {
            return;
        }
        let mut cmd = OwnedChatCommand::new("Cef", c_chat_command_callback, false, vec!["cef"]);
        cmd.register();
        *slot = Some(cmd);
    });
}
