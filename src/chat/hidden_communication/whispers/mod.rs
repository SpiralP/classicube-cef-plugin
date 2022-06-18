pub mod incoming;
pub mod outgoing;

use std::cell::Cell;

use classicube_helpers::WithInner;
use futures::{future::RemoteHandle, prelude::*};
use rand::seq::SliceRandom;
use tracing::{debug, warn};

use super::{encoding, wait_for_message, SHOULD_BLOCK};
use crate::{async_manager, chat::ENTITIES, error::Result};

thread_local!(
    static LISTENER: Cell<Option<RemoteHandle<()>>> = Cell::default();
);

pub async fn start_whispering(players: Vec<(u8, String)>) -> Result<()> {
    if players.is_empty() {
        return Ok(());
    }

    let mut real_players: Vec<_> = players
        .iter()
        .filter(|(id, _real_name)| {
            // check if they're on our map
            ENTITIES
                .with_inner(|entities| entities.get(*id).is_some())
                .unwrap()
        })
        .collect();

    debug!("start_whispering {:#?}", real_players);

    real_players.shuffle(&mut rand::thread_rng());

    for (_id, real_name) in real_players {
        match outgoing::query_whisper(real_name).await {
            Ok(had_data) => {
                if had_data {
                    break;
                }
            }

            Err(e) => {
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

    async_manager::spawn_local_on_main_thread(f);

    LISTENER.with(move |cell| {
        cell.set(Some(remote_handle));
    });
}

pub fn stop_listening() {
    LISTENER.with(move |cell| {
        cell.set(None);
    });
}
