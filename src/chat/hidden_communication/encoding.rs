use crate::{entity_manager::EntityManager, error::*, players::Player};
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize)]
pub struct LightEntity {
    pub id: usize,
    pub player: Player,
    pub pos: [f32; 3],
    pub ang: [f32; 2],
    pub scale: f32,
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

pub fn create_message() -> Message {
    let mut light_entities = Vec::new();

    EntityManager::with_all_entities(|entities| {
        for (id, entity) in entities {
            let e = &entity.entity;

            let pos = [e.Position.X, e.Position.Y, e.Position.Z];
            let ang = [e.RotX, e.RotY];
            let scale = entity.get_scale();

            let mut player = entity.player.clone();
            if let Player::Youtube(ref mut yt) = &mut player {
                if let Some(start_time) = &mut yt.start_time {
                    yt.time = Instant::now() - *start_time;
                }
                yt.start_time = None;
            }

            light_entities.push(LightEntity {
                id: *id,
                pos,
                ang,
                player,
                scale,
            });
        }
    });

    Message {
        entities: light_entities,
    }
}

pub async fn received_message(mut message: Message) -> Result<bool> {
    let mut had_data = false;

    for info in message.entities.drain(..) {
        // if it already exists don't do anything
        if EntityManager::with_by_entity_id(info.id, |_| Ok(())).is_ok() {
            warn!("entity {} already exists, skipping", info.id);
        }

        debug!("creating {:#?}", info);

        EntityManager::create_entity_from_light_entity(info).await?;

        had_data = true;
    }

    Ok(had_data)
}
