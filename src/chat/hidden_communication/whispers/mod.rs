mod incoming;
mod outgoing;

use crate::chat::{ENTITIES, TAB_LIST};
use log::debug;

pub fn start_whispering(ids: Vec<u8>) {
    debug!("start_whispering");

    let id = ids[0];

    let entity_exists = ENTITIES.with(|cell| {
        let entities = &*cell.borrow();
        let entities = entities.as_ref().unwrap();
        entities.get(id).is_some()
    });

    if entity_exists {
        let maybe_real_name = TAB_LIST.with(|cell| {
            let tab_list = &*cell.borrow();
            let tab_list = tab_list.as_ref().unwrap();
            tab_list.get(id).and_then(|entry| entry.get_real_name())
        });

        if let Some(real_name) = maybe_real_name {
            outgoing::query_whisper(real_name);
        }
    }
}

#[must_use]
pub fn handle_chat_message(message: &str) -> bool {
    let a = outgoing::handle_chat_message(message);
    let b = incoming::handle_chat_message(message);

    if a || b {
        return true;
    }

    false
}
