use crate::{
    chat::{Chat, ENTITIES, TAB_LIST},
    helpers::ThreadLocalGetSet,
};
use log::debug;
use std::cell::Cell;

thread_local!(
    static SIMULATING: Cell<bool> = Cell::new(false);
);

#[must_use]
pub fn handle_chat_message(message: &str) -> bool {
    if SIMULATING.get() {
        // my outgoing whisper
        if message.starts_with("&7[<] ") && message.contains(": &f!CEF! ") {
            SIMULATING.set(false);

            debug!("OK");

            return true;
        }
    }

    // incoming whisper
    if message.starts_with("&9[>] ") && message.ends_with(": &f?CEF?") {
        debug!("incoming_whisper {:?}", message);

        // "&9[>] "
        let colon_pos = message.find(": &f").unwrap();
        let nick_name = &message[6..colon_pos];
        debug!("from {:?}", nick_name);

        let maybe_real_name = TAB_LIST.with(|cell| {
            let tab_list = &*cell.borrow();
            let tab_list = tab_list.as_ref().unwrap();
            tab_list
                .find_entry_by_nick_name(&nick_name)
                .and_then(|entry| {
                    let id = entry.get_id();

                    ENTITIES.with(|cell| {
                        let entities = &*cell.borrow();
                        let entities = entities.as_ref().unwrap();
                        if entities.get(id).is_some() {
                            Some(entry.get_real_name()?)
                        } else {
                            None
                        }
                    })
                })
        });

        if let Some(real_name) = maybe_real_name {
            debug!("sending to {:?}", real_name);

            SIMULATING.set(true);
            Chat::send(format!("@{}+ !CEF! no", real_name));

            return true;
        }
    }

    false
}
