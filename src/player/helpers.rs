use super::{MediaPlayer, Player, PlayerTrait, VolumeMode, YoutubePlayer};
use crate::{
    async_manager,
    chat::ENTITIES,
    entity_manager::{CefEntity, EntityManager},
    error::*,
    helpers::vec3_to_vector3,
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

pub fn compute_real_volume(entity: &CefEntity) -> Option<(f32, VolumeMode)> {
    let volume_mode = entity.player.get_volume_mode();

    if volume_mode == VolumeMode::Global {
        let current_volume = entity.player.get_volume();
        return Some((current_volume, volume_mode));
    }

    // use distance or panning volume

    let (my_pos, my_forward) = ENTITIES
        .with_inner(|entities| {
            let me = entities.get(ENTITIES_SELF_ID as _)?;

            // ignore pitch, not really needed
            // also when looking straight down, it swaps ears?
            let [_pitch, yaw] = me.get_head();

            Some((
                vec3_to_vector3(&me.get_eye_position()),
                vec3_to_vector3(&Vec3::get_dir_vector(yaw.to_radians(), 0.0)),
            ))
        })
        .flatten()?;

    let ent_pos = vec3_to_vector3(&entity.entity.Position);

    let (panning, multiplier, distance) = match volume_mode {
        VolumeMode::Global => unreachable!(),

        VolumeMode::Distance {
            multiplier,
            distance,
        } => (false, multiplier, distance),
        VolumeMode::Panning {
            multiplier,
            distance,
            ..
        } => (true, multiplier, distance),
    };

    let diff = my_pos - ent_pos;
    let percent = diff.magnitude() / distance;
    let percent = (1.0 - percent).max(0.0).min(1.0) * multiplier;

    if panning {
        let up = Vector3::y();

        let left = Vector3::cross(&my_forward, &up);
        let left = left.normalize();

        let pan = (ent_pos - my_pos).normalize().dot(&left);
        let pan = pan * 0.7;

        Some((
            percent,
            VolumeMode::Panning {
                multiplier,
                distance,
                pan,
            },
        ))
    } else {
        Some((percent, volume_mode))
    }
}

async fn start_loop(entity_id: usize) -> Result<()> {
    loop {
        // update volume
        EntityManager::with_entity(entity_id, |entity| {
            if let Some((volume, volume_mode)) = compute_real_volume(entity) {
                let _ignore = entity.player.set_volume(entity.browser.as_ref(), volume);

                let _ignore = entity
                    .player
                    .set_volume_mode(entity.browser.as_ref(), volume_mode);
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
                    debug!("finished playing!");

                    EntityManager::with_entity(entity_id, move |entity| {
                        match &mut entity.player {
                            Player::Media(_player) => {
                                //
                            }

                            Player::Youtube(player) => {
                                player.finished = is_finished_playing;
                                entity.skip()?;
                            }

                            _ => {
                                bail!("is_finished_playing not supported");
                            }
                        }
                        Ok(())
                    })?;

                    break;
                }
            }
        }

        async_manager::sleep(Duration::from_millis(32)).await;
    }

    Ok(())
}
