use std::{collections::VecDeque, time::Duration};

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    cef::Cef,
    entity_manager::{EntityBuilder, EntityManager},
    error::*,
    player::{Player, PlayerTrait},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct LightEntity {
    player: Player,
    queue: VecDeque<Player>,

    name: Option<String>,
    resolution: Option<(u16, u16)>,
    size: (u16, u16),
    scale: f32,
    rotation: (f32, f32),
    position: (f32, f32, f32),
    background_color: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub entities: Vec<LightEntity>,
}

/// to base64
pub fn encode(message: &Message) -> Result<String> {
    let data = bincode::serialize(message)?;

    Ok(base64::encode(data))
}

/// from base64
pub fn decode<T: AsRef<[u8]>>(input: T) -> Result<Message> {
    let data = base64::decode(input)?;

    Ok(bincode::deserialize(&data)?)
}

pub async fn create_message() -> Message {
    let light_entities: Vec<_> = EntityManager::with_all_entities(|entities| {
        entities
            .iter()
            .filter(|(_id, entity)| entity.should_send)
            .map(|(&_id, entity)| {
                let e = &entity.entity;

                let player = entity.player.clone();
                let queue = entity
                    .queue
                    .iter()
                    .map(|(player, _)| player)
                    .cloned()
                    .collect();

                let name = entity.name.clone();
                let resolution = entity.browser.as_ref().map(Cef::get_browser_size);
                let size = entity.get_size();
                let scale = entity.get_scale();
                let rotation = (e.RotX, e.RotY);
                let position = (e.Position.X, e.Position.Y, e.Position.Z);
                let background_color = entity.background_color;

                LightEntity {
                    player,
                    queue,
                    name,
                    resolution,
                    size,
                    scale,
                    rotation,
                    position,
                    background_color,
                }
            })
            .collect()
    });

    Message {
        entities: light_entities,
    }
}

pub async fn received_message(mut message: Message) -> Result<bool> {
    let mut had_data = false;

    // only remove synced browsers
    let mut entity_ids: Vec<usize> = EntityManager::with_all_entities(|entities| {
        entities
            .iter()
            .filter(|(_, entity)| entity.should_send)
            .map(|(&id, _)| id)
            .collect()
    });

    for id in entity_ids.drain(..) {
        EntityManager::remove_entity(id).await?;
    }

    for mut info in message.entities.drain(..) {
        debug!("creating {:#?}", info);

        if let Ok(time) = info.player.get_current_time() {
            if time > Duration::from_secs(1) {
                // this is a couple seconds behind because of the load time
                // of the browser page and whisper delay, so add a couple seconds
                if let Player::YouTube(ref mut yt) = &mut info.player {
                    yt.time = time + Duration::from_secs(4);
                } else if let Player::Media(ref mut media) = &mut info.player {
                    media.time = time + Duration::from_secs(4);
                }
            }
        }

        let mut builder = EntityBuilder::new(info.player)
            .queue(info.queue)
            .size(info.size.0, info.size.1)
            .scale(info.scale)
            .rotation(info.rotation.0, info.rotation.1)
            .position(info.position.0, info.position.1, info.position.2)
            .background_color(info.background_color);

        if let Some(name) = info.name {
            builder = builder.name(name);
        }

        if let Some(res) = info.resolution {
            builder = builder.resolution(res.0, res.1);
        }

        builder.create().await?;

        had_data = true;
    }

    Ok(had_data)
}
