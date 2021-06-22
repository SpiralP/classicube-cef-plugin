use super::{MediaPlayer, Player, PlayerTrait, VolumeMode, YouTubePlayer};
use crate::{
    async_manager,
    entity_manager::{CefEntity, EntityManager},
    error::*,
    helpers::vec3_to_vector3,
};
use classicube_sys::{Camera, Vec3};
use nalgebra::Vector3;
use std::time::Duration;
use tracing::*;

pub async fn start_update_loop(entity_id: usize) {
    let result = start_loop(entity_id).await;

    if let Err(e) = result {
        warn!("start_update_loop {} {}", entity_id, e);
    }
}

fn compute_real_volume(entity: &CefEntity) -> Option<(f32, VolumeMode)> {
    let volume_mode = entity.player.get_volume_mode();

    if volume_mode == VolumeMode::Global {
        let current_volume = entity.player.get_volume();
        return Some((current_volume, volume_mode));
    }

    // use distance or panning volume

    let (position, orientation) = unsafe {
        if Camera.Active.is_null() {
            warn!("Camera.Active is null!");
            return None;
        }
        let camera = &*Camera.Active;
        let position = camera.GetPosition.map(|f| f(0.0))?;
        let orientation = camera.GetOrientation.map(|f| f())?;
        (position, orientation)
    };

    let my_pos = vec3_to_vector3(&position);
    let my_forward = vec3_to_vector3(&Vec3::get_dir_vector(orientation.X, 0.0));

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
        let pan = pan * 0.8;

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
            YouTube,
            Media,
        }
        let opt = EntityManager::with_entity(entity_id, |entity| {
            // we have to do this in parts because get_real_time() is async
            // while with_entity is not
            Ok(match &entity.player {
                Player::Media(_) => Some((entity.browser.as_ref().cloned(), Kind::Media)),
                Player::YouTube(_) => Some((entity.browser.as_ref().cloned(), Kind::YouTube)),

                _ => None,
            })
        })?;

        if let Some((Some(browser), kind)) = opt {
            // check if finished playing
            // Do this before setting time because get_real_time will return duration
            // instead of a current time after finishing video
            let is_finished_playing = match kind {
                Kind::Media => MediaPlayer::real_is_finished_playing(&browser).await?,
                Kind::YouTube => YouTubePlayer::real_is_finished_playing(&browser).await?,
            };

            let time = match kind {
                Kind::Media => MediaPlayer::get_real_time(&browser).await,
                Kind::YouTube => YouTubePlayer::get_real_time(&browser).await,
            };

            // update time field for when we sync to someone else
            if let Ok(time) = time {
                EntityManager::with_entity(entity_id, move |entity| {
                    match &mut entity.player {
                        Player::Media(player) => {
                            player.time = time;
                        }
                        Player::YouTube(player) => {
                            player.time = time;
                        }

                        _ => {
                            bail!("not supported time");
                        }
                    }
                    Ok(())
                })?;
            }

            if is_finished_playing {
                debug!("finished playing!");

                EntityManager::with_entity(entity_id, move |entity| {
                    match &mut entity.player {
                        Player::Media(player) => {
                            player.finished = true;
                            entity.skip()?;
                        }

                        Player::YouTube(player) => {
                            player.finished = true;
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

        async_manager::sleep(Duration::from_millis(32)).await;
    }

    Ok(())
}
