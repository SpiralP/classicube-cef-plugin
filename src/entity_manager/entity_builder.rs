use super::{CefEntity, EntityManager, ENTITIES, NAME_TO_ID};
use crate::{
    async_manager,
    cef::Cef,
    error::*,
    options::FRAME_RATE,
    player::{Player, PlayerTrait},
};
use log::*;
use std::collections::VecDeque;

pub struct EntityBuilder {
    player: Player,
    queue: VecDeque<Player>,

    name: Option<String>,
    insecure: bool,
    should_send: bool,
    frame_rate: u16,
    resolution: Option<(u16, u16)>,
    size: Option<(u16, u16)>,
    scale: f32,
    rotation: Option<(f32, f32)>,
    position: Option<(f32, f32, f32)>,
}

impl EntityBuilder {
    pub fn new(player: Player) -> Self {
        Self {
            player,
            queue: VecDeque::new(),
            name: None,
            insecure: false,
            should_send: true,
            frame_rate: FRAME_RATE.get().unwrap(),
            resolution: None,
            size: None,
            scale: 0.25,
            rotation: None,
            position: None,
        }
    }

    pub fn create(mut self) -> Result<usize> {
        let name = self.name.take();
        let url = self.player.on_create();

        let entity_id = EntityManager::get_new_id();
        ENTITIES.with(move |cell| {
            let entities = &mut *cell.borrow_mut();

            let mut entity =
                CefEntity::register(entity_id, self.player, self.queue, self.should_send);

            if let Some(pos) = self.position {
                entity.entity.Position.set(pos.0, pos.1, pos.2);
            }

            if let Some(rot) = self.rotation {
                entity.entity.RotX = rot.0;
                entity.entity.RotY = rot.1;
            }

            if let Some(size) = self.size {
                entity.set_size(size.0, size.1);
            }
            entity.set_scale(self.scale);

            debug!("entity {} registered", entity_id);
            entities.insert(entity_id, entity);

            let frame_rate = self.frame_rate;
            let insecure = self.insecure;
            let resolution = self.resolution;
            async_manager::spawn_local_on_main_thread(async move {
                let result = async move {
                    let browser = Cef::create_browser(url, frame_rate, insecure).await?;

                    if let Some((width, height)) = resolution {
                        Cef::resize_browser(&browser, width, height)?;
                    }

                    EntityManager::with_entity(entity_id, |entity| {
                        entity.attach_browser(browser);
                        Ok(())
                    })?;

                    Ok::<_, Error>(())
                };

                if let Err(e) = result.await {
                    warn!("create_attach_browser: {}", e);
                }
            });
        });

        if let Some(name) = name {
            debug!("created named entity {:?} with id {}", name, entity_id);

            NAME_TO_ID.with(|cell| {
                let name_to_id = &mut *cell.borrow_mut();
                name_to_id.insert(name, entity_id);
            });
        } else {
            debug!("created entity with id {}", entity_id);
        }

        Ok(entity_id)
    }

    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn insecure(mut self, insecure: bool) -> Self {
        self.insecure = insecure;
        self
    }

    pub fn should_send(mut self, should_send: bool) -> Self {
        self.should_send = should_send;
        self
    }

    pub fn frame_rate(mut self, frame_rate: u16) -> Self {
        self.frame_rate = frame_rate;
        self
    }

    pub fn resolution(mut self, width: u16, height: u16) -> Self {
        self.resolution = Some((width, height));
        self
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.size = Some((width, height));
        self
    }

    pub fn position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = Some((x, y, z));
        self
    }

    pub fn rotation(mut self, x: f32, y: f32) -> Self {
        self.rotation = Some((x, y));
        self
    }

    pub fn queue(mut self, queue: VecDeque<Player>) -> Self {
        self.queue = queue;
        self
    }
}
