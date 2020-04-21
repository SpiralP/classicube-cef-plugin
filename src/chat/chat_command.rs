#![allow(clippy::single_match)]

use super::Chat;
use crate::{
    async_manager::AsyncManager,
    chat::{PlayerSnapshot, ENTITIES},
    entity_manager::{CefEntity, EntityManager, CEF_HEIGHT, CEF_WIDTH},
    error::*,
    players, search,
};
use classicube_sys::{OwnedChatCommand, Vec3, ENTITIES_SELF_ID};
use log::debug;
use std::{os::raw::c_int, slice};

extern "C" fn c_chat_command_callback(args: *const classicube_sys::String, args_count: c_int) {
    let args = unsafe { slice::from_raw_parts(args, args_count as _) };
    let args: Vec<String> = args.iter().map(|cc_string| cc_string.to_string()).collect();

    let player_snapshot = ENTITIES
        .with(|cell| {
            let entities = &*cell.borrow();
            let entities = entities.as_ref().unwrap();
            entities.get(ENTITIES_SELF_ID as _).map(|entity| {
                let position = entity.get_position();
                let eye_position = entity.get_eye_position();
                let head = entity.get_head();
                let rot = entity.get_rot();
                PlayerSnapshot {
                    Position: position,
                    eye_position,
                    Pitch: head[0],
                    Yaw: head[1],
                    RotX: rot[0],
                    RotY: rot[1],
                    RotZ: rot[2],
                }
            })
        })
        .unwrap();

    AsyncManager::spawn_local_on_main_thread(async move {
        if let Err(e) = command_callback(&player_snapshot, args, true).await {
            Chat::print(format!("cef command error: {}", e));
        }
    });
}

fn move_entity(entity: &mut CefEntity, player: &PlayerSnapshot) {
    let dir = Vec3::get_dir_vector(player.Yaw.to_radians(), player.Pitch.to_radians());

    entity.entity.Position.set(
        player.eye_position.X + dir.X,
        player.eye_position.Y + dir.Y,
        player.eye_position.Z + dir.Z,
    );

    // turn it to face the player
    entity.entity.RotY = player.Yaw + 180f32;
    entity.entity.RotX = 360f32 - player.Pitch;
}

pub async fn command_callback(
    player: &PlayerSnapshot,
    args: Vec<String>,
    is_self: bool,
) -> Result<()> {
    debug!("command_callback {:?}", args);
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
    let args: &[&str] = &args;

    // static commands not targetted at a specific entity
    match args {
        ["create"] => {
            let entity_id = players::create("https://www.classicube.net/")?;
            EntityManager::with_by_entity_id(entity_id, |entity| {
                move_entity(entity, player);

                entity.entity.RotY = 180f32;
                Ok(())
            })?;
        }

        ["create", url] => {
            let entity_id = players::create(url)?;
            EntityManager::with_by_entity_id(entity_id, |entity| {
                move_entity(entity, player);

                Ok(())
            })?;
        }

        ["search", ..] => {
            if is_self {
                let input: Vec<_> = args.iter().skip(1).copied().collect();
                let input = input.join(" ");
                let input = (*input).to_string();
                let id = search::youtube::search(&input).await?;

                Chat::send(format!("cef play {}", id));
            }
        }

        _ => {}
    }

    // commands that target a certain entity by id
    match args {
        ["here", entity_id] => {
            let entity_id: usize = entity_id.parse()?;

            EntityManager::with_by_entity_id(entity_id, |entity| {
                move_entity(entity, player);

                Ok(())
            })?;
        }

        _ => {}
    }

    // commands that target the closest entity/browser
    match args {
        ["here"] | ["move"] => EntityManager::with_closest(player.eye_position, |entity| {
            move_entity(entity, player);

            Ok(())
        })?,

        ["at", x, y, z] => EntityManager::with_closest(player.eye_position, |entity| {
            let x = x.parse()?;
            let y = y.parse()?;
            let z = z.parse()?;

            entity.entity.Position.set(x, y, z);

            Ok(())
        })?,

        ["scale", scale] => EntityManager::with_closest(player.eye_position, |entity| {
            let scale = scale.parse()?;

            entity.set_scale(scale);

            Ok(())
        })?,

        ["load", url] | ["play", url] => {
            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;
            players::play(url, entity_id)?;
        }

        ["close"] | ["remove"] => {
            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;
            EntityManager::remove_entity(entity_id);
        }

        ["refresh"] | ["reload"] => {
            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;
            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            browser.reload()?;
        }

        ["click"] => {
            let (entity_id, entity_pos, [entity_pitch, entity_yaw], entity_scale) =
                EntityManager::with_closest(player.eye_position, |closest_entity| {
                    Ok((
                        closest_entity.id,
                        closest_entity.entity.Position,
                        [closest_entity.entity.RotX, closest_entity.entity.RotY],
                        closest_entity.entity.ModelScale,
                    ))
                })?;

            use nalgebra::*;
            use ncollide3d::{query::*, shape::*};

            fn intersect(
                eye_pos: Point3<f32>,
                [aim_pitch, aim_yaw]: [f32; 2],
                screen_pos: Point3<f32>,
                [screen_pitch, screen_yaw]: [f32; 2],
            ) -> Option<(Ray<f32>, RayIntersection<f32>)> {
                // when angles 0 0, aiming towards -z
                let normal = -Vector3::<f32>::z_axis();

                let aim_dir = Rotation3::from_euler_angles(
                    -aim_pitch.to_radians(),
                    -aim_yaw.to_radians(),
                    0.0,
                )
                .transform_vector(&normal);

                // positive pitch is clockwise on the -x axis
                // positive yaw is clockwise on the -y axis
                let rot = UnitQuaternion::from_euler_angles(
                    -screen_pitch.to_radians(),
                    -screen_yaw.to_radians(),
                    0.0,
                );
                let iso = Isometry3::from_parts(screen_pos.coords.into(), rot);

                let ray = Ray::new(eye_pos, aim_dir);
                let plane = Plane::new(normal);
                if let Some(intersection) = plane.toi_and_normal_with_ray(&iso, &ray, 10.0, true) {
                    if intersection.toi == 0.0 {
                        // 0 if aiming from wrong side
                        None
                    } else {
                        Some((ray, intersection))
                    }
                } else {
                    None
                }
            }

            fn vec3_to_vector3(v: &Vec3) -> Vector3<f32> {
                Vector3::new(v.X, v.Y, v.Z)
            }

            let eye_pos = vec3_to_vector3(&player.eye_position);
            let screen_pos = vec3_to_vector3(&entity_pos);

            if let Some((ray, intersection)) = intersect(
                eye_pos.into(),
                [player.Pitch, player.Yaw],
                screen_pos.into(),
                [entity_pitch, entity_yaw],
            ) {
                let intersection_point = ray.point_at(intersection.toi).coords;
                log::warn!("normal {}", intersection.normal);

                let scale = Vector3::new(entity_scale.X * 16.0, entity_scale.Y * 9.0, 1.0);
                let scaled_point = (intersection_point - screen_pos).component_div(&scale);

                let (x, y) = (0.5 + scaled_point.x, 1.0 - scaled_point.y);
                if x < 0.0 || x > 1.0 || y < 0.0 || y > 1.0 {
                    return Err("not looking at a screen".into());
                }
                debug!("{} {}", x, y);

                let (x, y) = (x * CEF_WIDTH as f32, y * CEF_HEIGHT as f32);
                debug!("{} {}", x, y);

                let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
                browser.send_click(x as _, y as _)?;
            }
        }

        ["type", ..] => {
            let text: Vec<_> = args.iter().skip(1).copied().collect();
            let text = text.join(" ");
            let text = (*text).to_string();

            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;

            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            browser.send_text(text)?;
        }

        ["click", x, y] => {
            let x: c_int = x.parse()?;
            let y: c_int = y.parse()?;

            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;

            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            browser.send_click(x, y)?;
        }

        _ => {}
    }

    Ok(())
}

pub struct CefChatCommand {
    chat_command: OwnedChatCommand,
}

impl CefChatCommand {
    pub fn new() -> Self {
        Self {
            chat_command: OwnedChatCommand::new("Cef", c_chat_command_callback, false, vec!["cef"]),
        }
    }

    pub fn initialize(&mut self) {
        self.chat_command.register();
    }

    pub fn shutdown(&mut self) {}
}
