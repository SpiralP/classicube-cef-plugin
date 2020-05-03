use super::wait_for_message;
use crate::{
    async_manager::AsyncManager,
    entity_manager::EntityManager,
    error::*,
    players::{MediaPlayer, Player, PlayerTrait, YoutubePlayer},
};
use async_std::future::timeout;
use classicube_helpers::tab_list::remove_color;
use futures::{future::RemoteHandle, prelude::*};
use log::{debug, warn};
use std::{cell::Cell, time::Duration};
use url::Url;

thread_local!(
    static CURRENT_RUNNING: Cell<Option<RemoteHandle<()>>> = Default::default();
);

pub fn on_new_map_loaded() {
    let (f, remote_handle) = async {
        match timeout(Duration::from_secs(10), do_check()).await {
            Ok(result) => {
                if let Err(e) = result {
                    warn!("map_theme checker failed: {}", e);
                }
            }

            Err(_timeout) => {}
        }
    }
    .remote_handle();

    AsyncManager::spawn_local_on_main_thread(f);

    CURRENT_RUNNING.with(move |cell| {
        cell.set(Some(remote_handle));
    });
}

async fn do_check() -> Result<()> {
    if let Some(url) = get_map_theme_url().await? {
        debug!("map_theme got {:?}", url);

        let volume = 0.25;
        let player = match YoutubePlayer::from_url(&url) {
            Ok(mut player) => {
                player.volume = volume;
                Player::Youtube(player)
            }
            Err(youtube_error) => match MediaPlayer::from_url(&url) {
                Ok(mut player) => {
                    player.volume = volume;
                    Player::Media(player)
                }
                Err(media_error) => {
                    bail!(
                        "couldn't create any player for url {:?}: {}, {}",
                        url,
                        youtube_error,
                        media_error
                    );
                }
            },
        };

        let entity_id = EntityManager::create_entity_player(player)?;

        // set quiet volume, and don't send to other players
        EntityManager::with_by_entity_id(entity_id, |entity| {
            entity.player.set_should_send(false);
            entity.player.set_global_volume(true)?;

            Ok(())
        })?;
    } else {
        debug!("never found map_theme url");
    }

    Ok(())
}

/// will check for message block messages containing urls
/// a couple seconds after joining a map
async fn get_map_theme_url() -> Result<Option<Url>> {
    let maybe_url = timeout(Duration::from_secs(5), async {
        loop {
            // TODO filter out join/leave, whispers, chat messages
            let message = wait_for_message().await;
            if message.contains("http") {
                debug!("got map_theme url first part {:?}", message);

                let mut parts: Vec<String> = Vec::new();
                parts.push(message);

                let timeout_result = timeout(Duration::from_secs(1), async {
                    loop {
                        let message = wait_for_message().await;
                        if message.starts_with("> &f") {
                            parts.push(message[4..].to_string());
                        } else {
                            debug!("stopping because of other message {:?}", message);
                            break;
                        }
                    }
                })
                .await;

                if timeout_result.is_err() {
                    debug!("stopping because of timeout");
                }

                let full_message: String = parts.join("");
                let full_message = remove_color(full_message);

                let regex = regex::Regex::new(r"https?://(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)").unwrap();

                debug!("trying regex");
                let match_ = regex.find(&full_message).chain_err(|| "regex find url")?;
                let url = match_.as_str();
                debug!("got match {:?}", url);
                let url = Url::parse(url)?;
                debug!("url parsed {:?}", url);

                return Ok::<_, Error>(url);
            }
        }
    })
    .await
    .ok();

    if let Some(maybe_url) = maybe_url {
        Ok(Some(maybe_url?))
    } else {
        Ok(None)
    }
}
