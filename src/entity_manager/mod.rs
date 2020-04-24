mod cef_paint;
mod context_handler;
mod entity;
mod model;
mod render_model_detour;

pub use self::{cef_paint::cef_paint_callback, entity::CefEntity};
use self::{
    context_handler::ContextHandler, model::CefModel, render_model_detour::RenderModelDetour,
};
use crate::{
    async_manager::AsyncManager,
    cef::{Cef, CefEvent, RustRefBrowser},
    chat::hidden_communication::LightEntity,
    error::*,
    players::{Player, PlayerTrait},
};
use classicube_sys::Vec3;
use futures::{
    future::RemoteHandle,
    prelude::*,
    stream::{FuturesUnordered, StreamExt},
};
use log::{debug, warn};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    os::raw::*,
};

pub const CEF_WIDTH: u32 = 1920;
pub const CEF_HEIGHT: u32 = 1080;

pub const TEXTURE_WIDTH: usize = 2048;
pub const TEXTURE_HEIGHT: usize = 2048;

pub const MODEL_WIDTH: u8 = 16;
pub const MODEL_HEIGHT: u8 = 9;

thread_local!(
    static ENTITY_ID: Cell<usize> = Cell::new(0);
);

// entity_id, entity
thread_local!(
    static ENTITIES: RefCell<HashMap<usize, CefEntity>> = RefCell::new(HashMap::new());
);

thread_local!(
    static BROWSER_ID_TO_ENTITY_ID: RefCell<HashMap<c_int, usize>> = RefCell::new(HashMap::new());
);

pub struct EntityManager {
    // model is just the shape, the entities holds the texture id and scaling
    model: Option<CefModel>,

    render_model_detour: RenderModelDetour,
    context_handler: ContextHandler,

    cef_event_loop: Option<RemoteHandle<()>>,
}

impl EntityManager {
    pub fn new() -> Self {
        let render_model_detour = RenderModelDetour::new();

        Self {
            model: None,
            render_model_detour,
            context_handler: ContextHandler::new(),
            cef_event_loop: None,
        }
    }

    pub fn initialize(&mut self) {
        debug!("initialize entity_manager");

        self.context_handler.initialize();
        self.render_model_detour.initialize();
        self.model = Some(CefModel::register());

        // let mut event_listener = Cef::create_event_listener();
        let (f, remote_handle) = async move {
            // loop {
            //     if let CefEvent::BrowserPageLoaded(mut browser) =
            //         event_listener.recv().await.unwrap()
            //     {
            //         let browser_id = browser.get_identifier();

            //         EntityManager::with_by_browser_id(browser_id, |entity| {
            //             entity.player.on_page_loaded(&mut browser);
            //             Ok(())
            //         })
            //         .unwrap();
            //     }
            // }
        }
        .remote_handle();

        AsyncManager::spawn_local_on_main_thread(f);

        self.cef_event_loop = Some(remote_handle);
    }

    pub fn on_new_map_loaded(&mut self) {
        debug!("on_new_map_loaded entity_manager");

        AsyncManager::block_on_local(async {
            Self::remove_all_entities().await;
        });
    }

    pub fn shutdown(&mut self) {
        debug!("shutdown entity_manager");

        self.context_handler.shutdown();
        self.render_model_detour.shutdown();
        self.model.take();
        drop(self.cef_event_loop.take());

        AsyncManager::block_on_local(async {
            Self::remove_all_entities().await;
        });
    }

    /// returns entity_id
    #[must_use]
    fn new_entity(player: Player) -> usize {
        ENTITIES.with(|entities| {
            let entities = &mut *entities.borrow_mut();

            let entity_id = ENTITY_ID.with(|cell| {
                let entity_id = cell.get();
                cell.set(entity_id + 1);
                entity_id
            });

            let entity = CefEntity::register(entity_id, player);
            debug!("entity created {}", entity_id);
            entities.insert(entity_id, entity);

            entity_id
        })
    }

    fn attach_browser_to_entity(entity_id: usize, browser: RustRefBrowser) {
        let browser_id = browser.get_identifier();

        BROWSER_ID_TO_ENTITY_ID.with(|ids| {
            let ids = &mut *ids.borrow_mut();

            ids.insert(browser_id, entity_id);

            ENTITIES.with(|entities| {
                let entities = &mut *entities.borrow_mut();

                if let Some(entity) = entities.get_mut(&entity_id) {
                    entity.browser = Some(browser);
                } else {
                    warn!("couldn't find entity for browser id {}", browser_id);
                }
            });
        });
    }

    /// Create an entity screen, start rendering a loading screen
    /// while we create a cef browser and wait for it to start rendering to it.
    ///
    /// returns browser_id
    pub fn create_entity(input: &str) -> Result<usize> {
        let mut player = Player::from_input(input)?;
        let url = player.on_create();

        let entity_id = EntityManager::new_entity(player);

        AsyncManager::spawn_local_on_main_thread(async move {
            let browser = Cef::create_browser(url).await;

            EntityManager::attach_browser_to_entity(entity_id, browser);
        });

        Ok(entity_id)
    }

    pub fn create_entity_from_light_entity(info: LightEntity) -> Result<usize> {
        let mut player = info.player.clone();
        let url = player.on_create();

        let entity_id = ENTITIES.with(|entities| {
            let entities = &mut *entities.borrow_mut();

            let entity_id = info.id;
            let entity = CefEntity::register(entity_id, player);
            debug!("entity created {}", entity_id);
            entities.insert(entity_id, entity);

            entity_id
        });

        EntityManager::with_by_entity_id(entity_id, |entity| {
            let e = &mut entity.entity;

            e.Position.set(info.pos[0], info.pos[1], info.pos[2]);
            warn!("POSITION {:?}", e.Position);

            e.RotX = info.ang[0];
            e.RotY = info.ang[1];

            AsyncManager::spawn_local_on_main_thread(async move {
                let browser = Cef::create_browser(url).await;

                EntityManager::attach_browser_to_entity(entity_id, browser);
            });

            Ok(entity_id)
        })
    }

    pub fn entity_play(input: &str, entity_id: usize) -> Result<()> {
        let mut player = Player::from_input(input)?;
        let url = player.on_create();

        let browser = EntityManager::with_by_entity_id(entity_id, |entity| {
            entity.player = player;

            let browser = entity.browser.as_ref().chain_err(|| "no browser")?;
            Ok(browser.clone())
        })?;

        browser.load_url(url)?;

        Ok(())
    }

    pub fn get_browser_by_entity_id(entity_id: usize) -> Result<RustRefBrowser> {
        ENTITIES.with(|entities| {
            let entities = &*entities.borrow();

            if let Some(entity) = entities.get(&entity_id) {
                if let Some(browser) = &entity.browser {
                    Ok(browser.clone())
                } else {
                    bail!("no browser for entity {}", entity_id);
                }
            } else {
                bail!("couldn't find entity for entity id {}", entity_id);
            }
        })
    }

    pub async fn remove_entity(entity_id: usize) {
        let maybe_browser = ENTITIES.with(|entities| {
            let entities = &mut *entities.borrow_mut();

            if let Some(mut entity) = entities.remove(&entity_id) {
                if let Some(browser) = entity.browser.take() {
                    Some(browser)
                } else {
                    None
                }
            } else {
                warn!(
                    "remove_entity: couldn't find entity for entity id {}",
                    entity_id
                );
                None
            }
        });

        if let Some(browser) = maybe_browser {
            EntityManager::on_browser_close(&browser);

            debug!(
                "entity_manager closing browser {}",
                browser.get_identifier()
            );
            Cef::close_browser(&browser).await;
        }
    }

    pub async fn remove_all_entities() {
        // don't drain here because we remove them in remove_entity()
        let entity_ids: Vec<usize> = ENTITIES.with(|entities| {
            let entities = &*entities.borrow();

            entities.keys().copied().collect()
        });

        let mut entity_ids: FuturesUnordered<_> = entity_ids
            .iter()
            .map(|entity_id| async move {
                debug!("entity_manager remove entity {}", entity_id);
                Self::remove_entity(*entity_id).await;
                entity_id
            })
            .collect();

        while let Some(entity_id) = entity_ids.next().await {
            debug!("entity_manager entity {} removed", entity_id);
        }
    }

    fn on_browser_close(browser: &RustRefBrowser) {
        let browser_id = browser.get_identifier();

        BROWSER_ID_TO_ENTITY_ID.with(|ids| {
            let ids = &mut *ids.borrow_mut();

            if let Some(entity_id) = ids.remove(&browser_id) {
                ENTITIES.with(|entities| {
                    let entities = &mut *entities.borrow_mut();

                    entities.remove(&entity_id);
                });
            } else {
                warn!("couldn't convert browser id {} to entity", browser_id);
            }
        });
    }

    pub fn with_by_browser_id<F, T>(browser_id: c_int, f: F) -> Result<T>
    where
        F: FnOnce(&mut CefEntity) -> Result<T>,
    {
        ENTITIES.with(|entities| {
            let entities = &mut *entities.borrow_mut();

            if let Some(entity) = EntityManager::get_by_browser_id(browser_id, entities) {
                f(entity)
            } else {
                bail!("No entity for browser id {}!", browser_id);
            }
        })
    }

    pub fn get_by_browser_id(
        browser_id: c_int,
        entities: &mut HashMap<usize, CefEntity>,
    ) -> Option<&mut CefEntity> {
        let maybe_entity_id = BROWSER_ID_TO_ENTITY_ID.with(|ids| {
            let ids = &*ids.borrow();
            ids.get(&browser_id).copied()
        });

        if let Some(entity_id) = maybe_entity_id {
            entities.get_mut(&entity_id)
        } else {
            None
        }
    }

    pub fn with_by_entity_id<F, T>(entity_id: usize, f: F) -> Result<T>
    where
        F: FnOnce(&mut CefEntity) -> Result<T>,
    {
        ENTITIES.with(|entities| {
            let entities = &mut *entities.borrow_mut();

            if let Some(entity) = EntityManager::get_by_entity_id(entity_id, entities) {
                f(entity)
            } else {
                bail!("No entity for entity id {}!", entity_id);
            }
        })
    }

    pub fn get_by_entity_id(
        entity_id: usize,
        entities: &mut HashMap<usize, CefEntity>,
    ) -> Option<&mut CefEntity> {
        entities.get_mut(&entity_id)
    }

    pub fn with_closest<F, T>(pos: Vec3, f: F) -> Result<T>
    where
        F: FnOnce(&mut CefEntity) -> Result<T>,
    {
        ENTITIES.with(|entities| {
            let entities = &mut *entities.borrow_mut();

            if let Some(entity) = EntityManager::get_closest_mut(pos, entities) {
                f(entity)
            } else {
                bail!("No browser to control!");
            }
        })
    }

    fn get_closest_mut(
        position: Vec3,
        entities: &mut HashMap<usize, CefEntity>,
    ) -> Option<&mut CefEntity> {
        let mut last_distance = None;
        let mut closest = None;

        for entity in entities.values_mut() {
            let distance = (position - entity.entity.Position).length_squared();

            if let Some(last_distance) = last_distance.as_mut() {
                if distance < *last_distance {
                    *last_distance = distance;
                    closest = Some(entity);
                }
            } else {
                last_distance = Some(distance);
                closest = Some(entity);
            }
        }

        closest
    }

    pub fn with_all_entities<F, T>(f: F) -> T
    where
        F: FnOnce(&mut HashMap<usize, CefEntity>) -> T,
    {
        ENTITIES.with(|entities| {
            let entities = &mut *entities.borrow_mut();

            f(entities)
        })
    }
}
