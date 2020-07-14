use super::wait_for_message;
use crate::{
    async_manager,
    chat::{hidden_communication::SHOULD_BLOCK, Chat, PlayerSnapshot},
    entity_manager::EntityManager,
    error::*,
    options::{AUTOPLAY_MAP_THEMES, MAP_THEME_VOLUME},
    players::{MediaPlayer, Player, PlayerTrait, YoutubePlayer},
};
use classicube_helpers::{tab_list::remove_color, CellGetSet};
use classicube_sys::ENTITIES_SELF_ID;
use futures::{channel::mpsc, future::RemoteHandle, prelude::*};
use log::{debug, info, warn};
use std::{
    cell::{Cell, RefCell},
    pin::Pin,
    time::Duration,
};
use url::Url;

thread_local!(
    static LISTENER: Cell<Option<RemoteHandle<()>>> = Default::default();
);

thread_local!(
    static WORKER: Cell<Option<RemoteHandle<()>>> = Default::default();
);

type Item = Pin<Box<dyn Future<Output = ()>>>;
thread_local!(
    static WORKER_SENDER: RefCell<Option<mpsc::UnboundedSender<Item>>> = Default::default();
);

pub fn start_listening() {
    let (f, remote_handle) = async {
        listen_loop().await;
    }
    .remote_handle();

    async_manager::spawn_local_on_main_thread(f);

    LISTENER.with(move |cell| {
        cell.set(Some(remote_handle));
    });

    let (sender, receiver) = mpsc::unbounded();

    let (f, remote_handle) = async move {
        worker_loop(receiver).await;
    }
    .remote_handle();

    async_manager::spawn_local_on_main_thread(f);

    WORKER.with(move |cell| {
        cell.set(Some(remote_handle));
    });

    WORKER_SENDER.with(move |cell| {
        let option = &mut *cell.borrow_mut();
        *option = Some(sender);
    });
}

pub fn stop_listening() {
    WORKER_SENDER.with(move |cell| {
        let option = &mut *cell.borrow_mut();
        *option = None;
    });

    WORKER.with(move |cell| {
        cell.set(None);
    });

    LISTENER.with(move |cell| {
        cell.set(None);
    });
}

// we need to execute commands from scripts synchronously
async fn worker_loop(mut receiver: mpsc::UnboundedReceiver<impl Future<Output = ()>>) {
    while let Some(f) = receiver.next().await {
        f.await;
    }
    warn!("global message worker stopped?");
}

fn is_map_theme_message(message: &str) -> bool {
    let m = remove_color(message).to_lowercase();

    m.starts_with("map theme: ") || m.starts_with("map theme song: ")
}

fn is_global_cef_message(mut m: &str) -> Option<String> {
    if m.len() < 4 {
        return None;
    }

    if &m[0..1] == "&" {
        m = &m[2..];
    }

    if m.starts_with("cef ") || m.starts_with("cef: ") {
        Some(m[4..].to_string())
    } else {
        None
    }
}

pub async fn listen_loop() {
    loop {
        let message = wait_for_message().await;
        if let Some(global_cef_message) = is_global_cef_message(&message) {
            debug!("got global cef message {:?}", message);
            SHOULD_BLOCK.set(true);

            let args = global_cef_message
                .split(' ')
                .map(|a| a.to_string())
                .collect::<Vec<String>>();

            let player_snapshot = PlayerSnapshot::from_entity_id(ENTITIES_SELF_ID as _).unwrap();

            WORKER_SENDER.with(move |cell| {
                let option = &mut *cell.borrow_mut();

                if let Some(worker_sender) = option {
                    let _ignore = worker_sender.unbounded_send(
                        async move {
                            if let Err(e) =
                                crate::chat::commands::run(player_snapshot, args, false).await
                            {
                                warn!("command error: {:#?}", e);
                                Chat::print(format!(
                                    "{}cef command error: {}{}",
                                    classicube_helpers::color::RED,
                                    classicube_helpers::color::WHITE,
                                    e
                                ));
                            }
                        }
                        .boxed_local(),
                    );
                }
            });
        } else if is_map_theme_message(&message) {
            debug!("got map_theme url first part {:?}", message);

            let mut parts: Vec<String> = Vec::new();
            parts.push(message);

            let timeout_result = async_manager::timeout(Duration::from_secs(1), async {
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

            if timeout_result.is_none() {
                debug!("stopping because of timeout");
            }

            let full_message: String = parts.join("");
            let full_message = remove_color(full_message);

            info!("map_theme {:?}", full_message);

            async_manager::spawn_local_on_main_thread(async move {
                match handle_map_theme_url(full_message).await {
                    Ok(_) => {}

                    Err(e) => {
                        warn!("map_theme listen_loop: {}", e);
                    }
                }
            });
        }
    }
}

thread_local!(
    pub static CURRENT_MAP_THEME: Cell<Option<usize>> = Default::default();
);

pub fn on_new_map_loaded() {
    CURRENT_MAP_THEME.set(None);
}

async fn handle_map_theme_url(message: String) -> Result<()> {
    if !AUTOPLAY_MAP_THEMES.get()? {
        return Ok(());
    }

    let regex = regex::Regex::new(r"https?://(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)").unwrap();

    let match_ = regex.find(&message).chain_err(|| "regex find url")?;
    let url = match_.as_str();
    let url = Url::parse(url)?;

    debug!("map_theme got {:?}", url);

    let volume = MAP_THEME_VOLUME.get()?;
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

    let entity_id = if let Some(entity_id) = CURRENT_MAP_THEME.get() {
        EntityManager::entity_play_player(player, entity_id)?;
        entity_id
    } else {
        // 1 fps, 1x1 resolution
        let entity_id = EntityManager::create_entity_player(player, 1, false, Some((1, 1)), true)?;
        EntityManager::with_entity(entity_id, |entity| {
            entity.set_scale(0.0);

            Ok(())
        })?;

        entity_id
    };

    CURRENT_MAP_THEME.set(Some(entity_id));

    // set quiet volume, and don't send to other players
    EntityManager::with_entity(entity_id, |entity| {
        entity.player.set_should_send(false);
        entity.player.set_global_volume(true)?;

        Ok(())
    })?;

    Ok(())
}
