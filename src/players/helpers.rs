use super::{MediaPlayer, Player, PlayerTrait, YoutubePlayer};
use crate::{async_manager::AsyncManager, chat::ENTITIES, entity_manager::EntityManager, error::*};
use classicube_helpers::OptionWithInner;
use classicube_sys::ENTITIES_SELF_ID;
use log::*;
use std::time::Duration;

pub async fn start_update_loop(entity_id: usize) {
    let result = start_loop(entity_id).await;

    if let Err(e) = result {
        log::warn!("start_update_loop {} {}", entity_id, e);
    }
}

async fn start_loop(entity_id: usize) -> Result<()> {
    loop {
        // update volume
        EntityManager::with_by_entity_id(entity_id, |entity| {
            if let Some(browser) = &entity.browser {
                let current_volume = entity.player.get_volume(&browser)?;

                if !entity.player.has_global_volume() {
                    // use distance

                    let maybe_my_pos = ENTITIES
                        .with_inner(|entities| {
                            let me = entities.get(ENTITIES_SELF_ID as _)?;

                            Some(me.get_position())
                        })
                        .flatten();

                    if let Some(my_pos) = maybe_my_pos {
                        let entity_pos = entity.entity.Position;

                        let percent = (entity_pos - my_pos).length_squared().sqrt() / 30f32;
                        let percent = (1.0 - percent).max(0.0).min(1.0);

                        entity.player.set_volume(&browser, percent)?;
                    }
                } else {
                    // global volume

                    entity.player.set_volume(&browser, current_volume)?;
                }
            }

            Ok(())
        })?;

        // TODO add a has_timed

        enum Kind {
            Youtube,
            Media,
        }
        let opt = EntityManager::with_by_entity_id(entity_id, |entity| {
            // we have to do this in parts because get_real_time() is async
            // while with_by_entity_id is not
            Ok(match &entity.player {
                Player::Media(_) => Some((entity.browser.as_ref().cloned(), Kind::Media)),
                Player::Youtube(_) => Some((entity.browser.as_ref().cloned(), Kind::Youtube)),

                _ => None,
            })
        })?;

        if let Some((maybe_browser, kind)) = opt {
            if let Some(browser) = maybe_browser {
                let time = match kind {
                    Kind::Media => MediaPlayer::get_real_time(&browser).await,
                    Kind::Youtube => YoutubePlayer::get_real_time(&browser).await,
                };

                // update time field for when we sync to someone else
                if let Ok(time) = time {
                    EntityManager::with_by_entity_id(entity_id, move |entity| {
                        match &mut entity.player {
                            Player::Media(player) => {
                                player.time = time;
                            }
                            Player::Youtube(player) => {
                                player.time = time;
                            }

                            _ => {
                                bail!("not supported");
                            }
                        }
                        Ok(())
                    })?;
                }

                // check if finished playing
                let is_finished_playing = match kind {
                    Kind::Media => unimplemented!(), /* MediaPlayer::is_finished_playing(&browser).await, */
                    Kind::Youtube => YoutubePlayer::real_is_finished_playing(&browser).await?,
                };

                if is_finished_playing {
                    debug!("finished playing!");

                    EntityManager::entity_skip(entity_id)?;
                    break;
                }
            }
        }

        AsyncManager::sleep(Duration::from_millis(64)).await;
    }

    Ok(())
}
