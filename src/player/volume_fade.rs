use std::{cell::Cell, time::Duration};

use classicube_helpers::async_manager;
use futures::{future::RemoteHandle, prelude::*};
use tracing::{debug, warn};

use crate::{
    entity_manager::EntityManager,
    error::{Error, Result},
    player::{Player, PlayerTrait},
};

thread_local!(
    static FADING_HANDLE: Cell<Option<RemoteHandle<()>>> = Cell::default();
);

pub fn on_new_map() {
    debug!("volume_fade on_new_map");

    // fade all volume

    let (f, remote_handle) = async {
        if let Err(e) = fade_all().await {
            warn!("volume_fade: {}", e);
        }
    }
    .remote_handle();

    FADING_HANDLE.with(move |cell| {
        cell.set(Some(remote_handle));
    });
    async_manager::spawn_local_on_main_thread(f);
}

pub fn on_new_map_loaded() {
    debug!("volume_fade on_new_map_loaded");

    FADING_HANDLE.with(move |cell| {
        cell.set(None);
    });
}

async fn fade_all() -> Result<()> {
    debug!("fade_all");

    EntityManager::with_all_entities(|entities| {
        for entity in entities.values_mut() {
            match &mut entity.player {
                Player::YouTube(player) => drop(player.update_loop_handle.take()),
                Player::Dash(player) => drop(player.update_loop_handle.take()),
                Player::Hls(player) => drop(player.update_loop_handle.take()),
                Player::Media(player) => drop(player.update_loop_handle.take()),
                Player::Image(_player) => {}
                Player::Web(_player) => {}
            }
        }

        Ok::<_, Error>(())
    })?;

    loop {
        EntityManager::with_all_entities(|entities| {
            for entity in entities.values_mut() {
                if let Some(browser) = &entity.browser {
                    let current_volume = entity.player.get_volume();
                    let next_volume = (current_volume - 0.025).max(0.0);
                    let _ignore = entity.player.set_volume(Some(browser), next_volume);
                }
            }

            Ok::<_, Error>(())
        })?;

        async_manager::sleep(Duration::from_millis(32)).await;
    }
}
