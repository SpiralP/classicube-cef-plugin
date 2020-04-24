use crate::{entity_manager::EntityManager, error::*, players::Player};
use log::debug;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LightEntity {
    pub id: usize,
    pub player: Player,
    pub pos: [f32; 3],
    pub ang: [f32; 2],
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

            let player = entity.player.clone();

            light_entities.push(LightEntity {
                id: *id,
                pos,
                ang,
                player,
            });
        }
    });

    Message {
        entities: light_entities,
    }
}

pub fn received_message(mut message: Message) -> Result<()> {
    for info in message.entities.drain(..) {
        debug!("creating {:#?}", info);

        EntityManager::create_entity_from_light_entity(info)?;
    }

    Ok(())
}
