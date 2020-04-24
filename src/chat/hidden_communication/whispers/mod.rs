mod incoming;
mod outgoing;

use super::{encoding, wait_for_message, SHOULD_BLOCK};
use crate::{async_manager::AsyncManager, chat::ENTITIES, error::*};
use classicube_helpers::OptionWithInner;
use futures::{future::RemoteHandle, prelude::*};
use log::{debug, warn};
use std::cell::Cell;

thread_local!(
    static LISTENER: Cell<Option<RemoteHandle<()>>> = Default::default();
);

pub async fn start_whispering(players: Vec<(u8, String)>) -> Result<()> {
    if players.is_empty() {
        return Ok(());
    }

    debug!("start_whispering");

    for (id, real_name) in players {
        // check if they're on our map
        let entity_exists = ENTITIES
            .with_inner(|entities| entities.get(id).is_some())
            .unwrap();

        if entity_exists {
            if let Err(e) = outgoing::query_whisper(&real_name).await {
                warn!("query_whisper {} failed: {}", real_name, e);
            }
        }
    }

    Ok(())
}

pub fn start_listening() {
    let (f, remote_handle) = async {
        incoming::listen_loop().await;
    }
    .remote_handle();

    AsyncManager::spawn_local_on_main_thread(f);

    LISTENER.with(move |cell| {
        cell.set(Some(remote_handle));
    });
}

pub fn stop_listening() {
    LISTENER.with(move |cell| {
        cell.set(None);
    });
}
