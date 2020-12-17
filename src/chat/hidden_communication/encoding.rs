use crate::{
    cef::Cef,
    entity_manager::{EntityBuilder, EntityManager},
    error::*,
    player::{Player, PlayerTrait},
};
use tracing::debug;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, time::Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct LightEntity {
    pub name: Option<String>,
    pub player: Player,
    pub queue: VecDeque<Player>,
    pub pos: (f32, f32, f32),
    pub ang: (f32, f32),
    pub scale: f32,
    pub size: (u16, u16),
    pub resolution: Option<(u16, u16)>,
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
            .filter_map(|(&_id, entity)| {
                if !entity.should_send() {
                    return None;
                }

                let e = &entity.entity;

                let name = entity.name.clone();
                let pos = (e.Position.X, e.Position.Y, e.Position.Z);
                let ang = (e.RotX, e.RotY);
                let scale = entity.get_scale();
                let size = entity.get_size();
                let resolution = entity.browser.as_ref().map(Cef::get_browser_size);

                let player = entity.player.clone();
                let queue = entity
                    .queue
                    .iter()
                    .map(|(player, _)| player)
                    .cloned()
                    .collect();

                Some(LightEntity {
                    name,
                    pos,
                    ang,
                    player,
                    queue,
                    scale,
                    size,
                    resolution,
                })
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
            .filter(|(_, entity)| entity.should_send())
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
            .position(info.pos.0, info.pos.1, info.pos.2)
            .rotation(info.ang.0, info.ang.1)
            .scale(info.scale)
            .size(info.size.0, info.size.1);

        if let Some(name) = info.name {
            builder = builder.name(name);
        }

        if let Some(res) = info.resolution {
            builder = builder.resolution(res.0, res.1);
        }

        builder.create()?;

        had_data = true;
    }

    Ok(had_data)
}
