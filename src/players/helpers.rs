use super::{MediaPlayer, Player, PlayerTrait, YoutubePlayer};
use crate::{
    async_manager,
    chat::ENTITIES,
    entity_manager::{CefEntity, EntityManager},
    error::*,
    helpers::vec3_to_vector3,
    players::RustRefBrowser,
};
use classicube_helpers::OptionWithInner;
use classicube_sys::{Vec3, ENTITIES_SELF_ID};
use log::*;
use nalgebra::Vector3;
use std::time::Duration;

pub async fn start_update_loop(entity_id: usize) {
    let result = start_loop(entity_id).await;

    if let Err(e) = result {
        log::warn!("start_update_loop {} {}", entity_id, e);
    }
}

pub fn compute_real_volume(browser: &RustRefBrowser, entity: &CefEntity) -> Option<f32> {
    let current_volume = entity.player.get_volume().ok()?;
    if entity.player.has_global_volume() {
        // global volume

        Some(current_volume)
    } else {
        // use distance volume

        let (my_pos, my_forward) = ENTITIES
            .with_inner(|entities| {
                let me = entities.get(ENTITIES_SELF_ID as _)?;

                let [pitch, yaw] = me.get_head();
                Some((
                    vec3_to_vector3(&me.get_eye_position()),
                    vec3_to_vector3(&Vec3::get_dir_vector(yaw.to_radians(), pitch.to_radians())),
                ))
            })
            .flatten()?;

        let ent_pos = vec3_to_vector3(&entity.entity.Position);

        if true {
            let up = Vector3::y();

            let left = Vector3::cross(&my_forward, &up);
            let left = left.normalize();

            let pan = (ent_pos - my_pos).normalize().dot(&left);
            let pan = pan.max(-0.95).min(0.95);

            let _ignore = browser.execute_javascript_on_frame(
                "https://www.youtube.com",
                format!("window.panner.pan.value = {}", pan),
            );
        }

        let diff = my_pos - ent_pos;
        let percent = diff.magnitude() / 30f32;
        let percent = (1.0 - percent).max(0.0).min(1.0);

        Some(percent)
    }
}

async fn start_loop(entity_id: usize) -> Result<()> {
    loop {
        // update volume
        EntityManager::with_entity(entity_id, |entity| {
            if let Some(browser) = &entity.browser {
                if let Some(volume) = compute_real_volume(browser, entity) {
                    entity.player.set_volume(browser, volume)?;
                }
            }

            Ok(())
        })?;

        // TODO add a has_timed

        enum Kind {
            Youtube,
            Media,
        }
        let opt = EntityManager::with_entity(entity_id, |entity| {
            // we have to do this in parts because get_real_time() is async
            // while with_entity is not
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
                    EntityManager::with_entity(entity_id, move |entity| {
                        match &mut entity.player {
                            Player::Media(player) => {
                                player.time = time;
                            }
                            Player::Youtube(player) => {
                                player.time = time;
                            }

                            _ => {
                                bail!("not supported time");
                            }
                        }
                        Ok(())
                    })?;
                }

                // check if finished playing
                let is_finished_playing = match kind {
                    Kind::Media => {
                        // TODO
                        false
                    }

                    Kind::Youtube => YoutubePlayer::real_is_finished_playing(&browser).await?,
                };

                if is_finished_playing {
                    EntityManager::with_entity(entity_id, move |entity| {
                        match &mut entity.player {
                            Player::Media(_player) => {
                                //
                            }

                            Player::Youtube(player) => {
                                player.finished = is_finished_playing;
                            }

                            _ => {
                                bail!("not supported is_finished_playing");
                            }
                        }
                        Ok(())
                    })?;

                    debug!("finished playing!");

                    EntityManager::entity_skip(entity_id)?;
                    break;
                }
            }
        }

        async_manager::sleep(Duration::from_millis(32)).await;
    }

    Ok(())
}
