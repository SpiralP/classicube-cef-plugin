use super::wait_for_message;
use crate::{
    chat::{
        helpers::{is_global_cef_message, is_map_theme_message},
        hidden_communication::SHOULD_BLOCK,
        is_continuation_message, Chat, PlayerSnapshot,
    },
    entity_manager::{EntityBuilder, EntityManager},
    error::{bail, Result},
    options,
    player::{PlayerBuilder, PlayerTrait, VolumeMode},
};
use classicube_helpers::async_manager;
use classicube_sys::ENTITIES_SELF_ID;
use futures::{channel::mpsc, future::RemoteHandle, prelude::*};
use std::{
    cell::{Cell, RefCell},
    pin::Pin,
    time::Duration,
};
use tracing::{debug, info, warn};

thread_local!(
    pub static CURRENT_MAP_THEME: Cell<Option<usize>> = Cell::default();
);

thread_local!(
    static LISTENER: Cell<Option<RemoteHandle<()>>> = Cell::default();
);

thread_local!(
    static WORKER: Cell<Option<RemoteHandle<()>>> = Cell::default();
);

type Item = Pin<Box<dyn Future<Output = ()>>>;
thread_local!(
    static WORKER_SENDER: RefCell<Option<mpsc::UnboundedSender<Item>>> = RefCell::default();
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
}

pub fn on_new_map() {
    WORKER_SENDER.with(move |cell| {
        let option = &mut *cell.borrow_mut();
        *option = None;
    });

    WORKER.with(move |cell| {
        cell.set(None);
    });
}

pub fn on_new_map_loaded() {
    CURRENT_MAP_THEME.set(None);

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

pub async fn listen_loop() {
    loop {
        let message = wait_for_message().await;
        if let Some(global_cef_message) = is_global_cef_message(&message) {
            debug!("got global cef message {:?}", message);
            SHOULD_BLOCK.set(true);

            let args = global_cef_message
                .split(' ')
                .map(ToString::to_string)
                .collect::<Vec<String>>();

            if let Some(player_snapshot) = PlayerSnapshot::from_entity_id(ENTITIES_SELF_ID as _) {
                WORKER_SENDER.with(move |cell| {
                    let option = &mut *cell.borrow_mut();

                    if let Some(worker_sender) = option {
                        let _ignore = worker_sender.unbounded_send(
                            async move {
                                if let Err(e) =
                                    crate::chat::commands::run(player_snapshot, args, false, true)
                                        .await
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
            }
        } else if let Some(first_part_input) = is_map_theme_message(&message) {
            debug!("got map_theme url first part {:?}", message);

            let mut input_parts: Vec<String> = vec![first_part_input.to_string()];

            let timeout_result = async_manager::timeout(Duration::from_secs(1), async {
                loop {
                    let message = wait_for_message().await;
                    if let Some(continuation) = is_continuation_message(&message) {
                        input_parts.push(continuation.to_string());
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

            let full_input: String = input_parts.join("");
            info!("map_theme {:?}", full_input);

            async_manager::spawn_local_on_main_thread(async move {
                match handle_map_theme_url(full_input).await {
                    Ok(()) => {}

                    Err(e) => {
                        warn!("map_theme listen_loop: {}", e);
                    }
                }
            });
        }
    }
}

async fn handle_map_theme_url(input: String) -> Result<()> {
    debug!("map_theme got {:?}", input);

    if !options::AUTOPLAY_MAP_THEMES.get()? {
        return Ok(());
    }

    // let regex = regex::Regex::new(r"https?://(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)").unwrap();

    // let match_ = regex.find(&message).chain_err(|| "regex find url")?;
    // let url = match_.as_str();
    // let url = Url::parse(url)?;

    // let mut player = match YouTubePlayer::from_input(&url) {
    //     Ok(player) => Player::YouTube(player),
    //     Err(youtube_error) => match MediaPlayer::from_input(&url) {
    //         Ok(player) => Player::Media(player),
    //         Err(media_error) => {
    //             bail!(
    //                 "couldn't create any player for url {:?}: {}, {}",
    //                 url,
    //                 youtube_error,
    //                 media_error
    //             );
    //         }
    //     },
    // };

    let volume = options::MAP_THEME_VOLUME.get()?;

    let mut players = PlayerBuilder::new()
        .silent(true)
        .volume(volume)
        .volume_mode(VolumeMode::Global)
        .should_loop(true)
        // use youtube's so it can loop after playlist end
        .use_youtube_playlist(true)
        .build(&input)
        .await?;

    for player in &players {
        if player.type_name() != "YouTube" && player.type_name() != "Media" {
            bail!("map theme not of type YoutubePlayer or MediaPlayer");
        }
    }

    let player = players.remove(0);

    if let Some(entity_id) = CURRENT_MAP_THEME.with(Cell::take) {
        EntityManager::remove_entity(entity_id).await?;
    }

    // 1 fps, 1x1 resolution, don't send to other players, don't print "Now Playing"
    let entity_id = EntityBuilder::new(player)
        .queue(players.into())
        .frame_rate(1)
        .resolution(1, 1)
        .should_send(false)
        .scale(0.0)
        .position(0.0, 0.0, 0.0)
        .create()
        .await?;

    CURRENT_MAP_THEME.set(Some(entity_id));

    Ok(())
}
