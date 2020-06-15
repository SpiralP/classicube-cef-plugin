use crate::{
    cef::Cef,
    entity_manager::EntityManager,
    error::*,
    players::{Player, PlayerTrait},
};
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Serialize, Deserialize)]
pub struct LightEntity {
    pub id: usize,
    pub player: Player,
    pub queue: VecDeque<Player>,
    pub pos: [f32; 3],
    pub ang: [f32; 2],
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
            .filter_map(|(&id, entity)| {
                if !entity.player.get_should_send() {
                    return None;
                }

                let e = &entity.entity;

                let pos = [e.Position.X, e.Position.Y, e.Position.Z];
                let ang = [e.RotX, e.RotY];
                let scale = entity.get_scale();
                let size = entity.get_size();
                let resolution = entity.browser.as_ref().map(Cef::get_browser_size);

                let player = entity.player.clone();
                let queue = entity.queue.clone();

                Some(LightEntity {
                    id,
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

    for mut info in message.entities.drain(..) {
        // if it already exists don't do anything
        // TODO?? use unique ids!
        if EntityManager::with_by_entity_id(info.id, |_| Ok(())).is_ok() {
            warn!("entity {} already exists, skipping", info.id);
            continue;
        }

        // don't know why should_send is false when receiving,
        // when default is true?
        // oh well, just always make sure it's true
        info.player.set_should_send(true);

        debug!("creating {:#?}", info);

        EntityManager::create_entity_from_light_entity(info).await?;

        had_data = true;
    }

    Ok(had_data)
}
