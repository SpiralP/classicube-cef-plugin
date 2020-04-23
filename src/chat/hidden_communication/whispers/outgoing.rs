use crate::{async_manager::AsyncManager, chat::Chat};
use classicube_helpers::CellGetSet;
use log::debug;
use std::{cell::Cell, time::Duration};

thread_local!(
    static SIMULATING: Cell<bool> = Cell::new(false);
);

pub fn query_whisper(real_name: String) {
    debug!("start_whispering asking {}", real_name);

    SIMULATING.set(true);
    Chat::send(format!("@{}+ ?CEF?", real_name));
    // &7[<] &uSpiralP2: &f?CEF?
    // &9[>] &uSpiralP: &f?CEF?
}

thread_local!(
    static LISTENING: Cell<bool> = Cell::new(false);
);

#[must_use]
pub fn handle_chat_message(message: &str) -> bool {
    if SIMULATING.get() {
        // my outgoing whisper
        if message.starts_with("&7[<] ") && message.ends_with(": &f?CEF?") {
            LISTENING.set(true);

            // give it a couple seconds before stop listening
            AsyncManager::spawn_local_on_main_thread(async {
                AsyncManager::sleep(Duration::from_secs(2)).await;

                if LISTENING.get() {
                    debug!("stopping because of timer");
                    LISTENING.set(false);
                    SIMULATING.set(false);
                }
            });

            return true;
        }

        if LISTENING.get() {
            // incoming whisper from them
            if message.starts_with("&9[>] ") && message.contains(": &f!CEF! ") {
                debug!("stopping because of {:?}", message);
                LISTENING.set(false);
                SIMULATING.set(false);

                // don't show this message!
                return true;
            }
        }
    }

    false
}
