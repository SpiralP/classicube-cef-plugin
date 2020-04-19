#![allow(clippy::single_match)]

use super::Chat;
use crate::{
    async_manager::AsyncManager,
    chat::PlayerSnapshot,
    entity_manager::{CefEntity, EntityManager},
    error::*,
    players, search,
};
use classicube_sys::{Entities, Entity, OwnedChatCommand, Vec3, ENTITIES_SELF_ID, MATH_DEG2RAD};
use log::debug;
use std::{os::raw::c_int, slice};

extern "C" fn c_chat_command_callback(args: *const classicube_sys::String, args_count: c_int) {
    let args = unsafe { slice::from_raw_parts(args, args_count as _) };
    let args: Vec<String> = args.iter().map(|cc_string| cc_string.to_string()).collect();

    let me = unsafe { &*Entities.List[ENTITIES_SELF_ID as usize] };

    let player_snapshot = PlayerSnapshot {
        Position: me.Position,
        Pitch: me.Pitch,
        Yaw: me.Yaw,
        RotX: me.RotX,
        RotY: me.RotY,
        RotZ: me.RotZ,
    };

    AsyncManager::spawn_local_on_main_thread(async move {
        if let Err(e) = command_callback(&player_snapshot, args, true).await {
            Chat::print(format!("cef command error: {}", e));
        }
    });
}

fn move_entity(entity: &mut CefEntity, player: &PlayerSnapshot) {
    let dir = Vec3::get_dir_vector(
        player.Yaw * MATH_DEG2RAD as f32,
        player.Pitch * MATH_DEG2RAD as f32,
    );

    entity.entity.Position.set(
        player.Position.X + dir.X,
        player.Position.Y + dir.Y,
        player.Position.Z + dir.Z,
    );

    // TODO why RotY? we use Yaw above?
    entity.entity.RotY = player.RotY;
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
        ["here"] => EntityManager::with_closest(player.Position, |entity| {
            move_entity(entity, player);

            Ok(())
        })?,

        ["at", x, y, z] => EntityManager::with_closest(player.Position, |entity| {
            let x = x.parse()?;
            let y = y.parse()?;
            let z = z.parse()?;

            entity.entity.Position.set(x, y, z);

            Ok(())
        })?,

        ["scale", scale] => EntityManager::with_closest(player.Position, |entity| {
            let scale = scale.parse()?;

            entity.set_scale(scale);

            Ok(())
        })?,

        ["play", url] => {
            let entity_id = EntityManager::with_closest(player.Position, |closest_entity| {
                Ok(closest_entity.id)
            })?;
            players::play(url, entity_id)?;
        }

        ["remove"] => {
            let entity_id = EntityManager::with_closest(player.Position, |closest_entity| {
                Ok(closest_entity.id)
            })?;
            EntityManager::remove_entity(entity_id);
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
