use crate::{
    cef::Cef,
    entity_manager::{EntityBuilder, EntityManager},
    error::*,
    player::Player,
};
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Serialize, Deserialize)]
pub struct LightEntity {
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

                let pos = (e.Position.X, e.Position.Y, e.Position.Z);
                let ang = (e.RotX, e.RotY);
                let scale = entity.get_scale();
                let size = entity.get_size();
                let resolution = entity.browser.as_ref().map(Cef::get_browser_size);

                let player = entity.player.clone();
                let queue = entity.queue.clone();

                Some(LightEntity {
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

    // if let Ok(time) = entity.player.get_current_time().await {
    //     // this is a couple seconds behind because of the load time
    //     // of the browser page
    //     if let Player::Youtube(ref mut yt) = &mut player {
    //         yt.time = time + Duration::from_secs(4);
    //     } else if let Player::Media(ref mut media) = &mut player {
    //         media.time = time + Duration::from_secs(4);
    //     }
    // }

    Message {
        entities: light_entities,
    }
}

pub async fn received_message(mut message: Message) -> Result<bool> {
    let mut had_data = false;

    for info in message.entities.drain(..) {
        debug!("creating {:#?}", info);

        let mut builder = EntityBuilder::new(info.player)
            .queue(info.queue)
            .position(info.pos.0, info.pos.1, info.pos.2)
            .rotation(info.ang.0, info.ang.1)
            .scale(info.scale)
            .size(info.size.0, info.size.1);

        if let Some(res) = info.resolution {
            builder = builder.resolution(res.0, res.1);
        }

        builder.create()?;

        had_data = true;
    }

    Ok(had_data)
}
