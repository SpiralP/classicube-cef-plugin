mod cef_paint;
mod context_handler;
mod entity;
mod entity_builder;
mod helpers;
mod model;
mod render_model_hook;

pub use self::{cef_paint::cef_paint_callback, entity::CefEntity, entity_builder::EntityBuilder};
use self::{context_handler::ContextHandler, model::CefModel};
use crate::{
    async_manager,
    cef::{Cef, CefEvent, RustRefBrowser},
    chat::PlayerSnapshot,
    error::*,
    player::PlayerTrait,
};
use classicube_sys::Vec3;
use futures::{
    future::RemoteHandle,
    prelude::*,
    stream::{FuturesUnordered, StreamExt},
};
use log::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    os::raw::*,
};

pub const TEXTURE_WIDTH: u16 = 2048;
pub const TEXTURE_HEIGHT: u16 = 2048;

pub const DEFAULT_MODEL_WIDTH: u8 = 16;
pub const DEFAULT_MODEL_HEIGHT: u8 = 9;

thread_local!(
    static ENTITY_ID: Cell<usize> = Cell::new(0);
);

// entity_id, entity
thread_local!(
    static ENTITIES: RefCell<HashMap<usize, CefEntity>> = Default::default();
);

thread_local!(
    static NAME_TO_ID: RefCell<HashMap<String, usize>> = Default::default();
);

thread_local!(
    static BROWSER_ID_TO_ENTITY_ID: RefCell<HashMap<c_int, usize>> = Default::default();
);

pub struct EntityManager {
    // model is just the shape, the entities holds the texture id and scaling
    model: Option<CefModel>,

    context_handler: ContextHandler,

    cef_event_page_loaded: Option<RemoteHandle<()>>,
    cef_event_title_change: Option<RemoteHandle<()>>,
}

impl EntityManager {
    pub fn new() -> Self {
        Self {
            model: None,
            context_handler: ContextHandler::new(),
            cef_event_page_loaded: None,
            cef_event_title_change: None,
        }
    }

    pub fn initialize(&mut self) {
        debug!("initialize entity_manager");

        self.context_handler.initialize();
        render_model_hook::initialize();
        self.model = Some(CefModel::register());

        self.initialize_listeners();
    }

    fn initialize_listeners(&mut self) {
        let mut event_listener = Cef::create_event_listener();
        let (f, remote_handle) = async move {
            while let Ok(event) = event_listener.recv().await {
                if let CefEvent::BrowserPageLoaded(browser) = event {
                    let browser_id = browser.get_identifier();

                    if let Err(e) = EntityManager::with_by_browser_id(browser_id, |entity| {
                        entity.on_page_loaded(&browser);
                        Ok(())
                    }) {
                        warn!("{}", e);
                    }
                }
            }
        }
        .remote_handle();
        async_manager::spawn_local_on_main_thread(f);
        self.cef_event_page_loaded = Some(remote_handle);

        let mut event_listener = Cef::create_event_listener();
        let (f, remote_handle) = async move {
            while let Ok(event) = event_listener.recv().await {
                if let CefEvent::BrowserTitleChange(browser, title) = event {
                    let browser_id = browser.get_identifier();

                    if let Err(e) = EntityManager::with_by_browser_id(browser_id, |entity| {
                        entity.player.on_title_change(entity.id, &browser, title);
                        Ok(())
                    }) {
                        warn!("{}", e);
                    }
                }
            }
        }
        .remote_handle();
        async_manager::spawn_local_on_main_thread(f);
        self.cef_event_title_change = Some(remote_handle);
    }

    pub fn on_new_map_loaded(&mut self) {
        debug!("on_new_map_loaded entity_manager");

        async_manager::block_on_local(async {
            let _ignore_error = Self::remove_all_entities().await;
        });
    }

    pub fn shutdown(&mut self) {
        debug!("shutdown entity_manager");

        self.context_handler.shutdown();
        render_model_hook::shutdown();
        self.model.take();
        self.cef_event_page_loaded.take();
        self.cef_event_title_change.take();

        async_manager::block_on_local(async {
            Self::remove_all_entities().await.unwrap();
        });
    }

    fn get_new_id() -> usize {
        ENTITY_ID.with(|cell| {
            let mut entity_id = cell.get();

            // if it already exists, try another
            while EntityManager::with_entity(entity_id, |_| Ok(())).is_ok() {
                entity_id += 1;
            }
            cell.set(entity_id + 1);
            entity_id
        })
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

    pub async fn remove_entity(entity_id: usize) -> Result<()> {
        NAME_TO_ID.with(|cell| {
            let name_to_id = &mut *cell.borrow_mut();
            let mut keys_to_remove: Vec<String> = name_to_id
                .iter()
                .filter_map(|(name, &id)| {
                    if id == entity_id {
                        Some(name.to_string())
                    } else {
                        None
                    }
                })
                .collect();
            for key in keys_to_remove.drain(..) {
                name_to_id.remove(&key);
            }
        });

        let maybe_browser = ENTITIES.with(|cell| {
            let entities = &mut *cell.borrow_mut();

            if let Some(mut entity) = entities.remove(&entity_id) {
                if let Some(browser) = entity.browser.take() {
                    Ok(Some(browser))
                } else {
                    Ok::<_, Error>(None)
                }
            } else {
                bail!(
                    "remove_entity: couldn't find entity for entity id {}",
                    entity_id
                );
            }
        })?;

        if let Some(browser) = maybe_browser {
            EntityManager::on_browser_close(&browser);

            debug!(
                "entity_manager closing browser {}",
                browser.get_identifier()
            );
            Cef::close_browser(&browser).await?;
        }

        Ok(())
    }

    pub async fn remove_all_entities() -> Result<()> {
        // don't drain here because we remove them in remove_entity()
        let entity_ids: Vec<usize> =
            Self::with_all_entities(|entities| entities.keys().copied().collect());

        let mut entity_ids: FuturesUnordered<_> = entity_ids
            .iter()
            .map(|entity_id| async move {
                debug!("entity_manager remove entity {}", entity_id);
                Self::remove_entity(*entity_id).await?;
                Ok::<_, Error>(entity_id)
            })
            .collect();

        while let Some(entity_id) = entity_ids.next().await {
            let entity_id = entity_id?;
            debug!("entity_manager entity {} removed", entity_id);
        }

        Ok(())
    }

    fn on_browser_close(browser: &RustRefBrowser) {
        let browser_id = browser.get_identifier();

        BROWSER_ID_TO_ENTITY_ID.with(|ids| {
            let ids = &mut *ids.borrow_mut();

            if let Some(entity_id) = ids.remove(&browser_id) {
                ENTITIES.with(|cell| {
                    let entities = &mut *cell.borrow_mut();

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
        ENTITIES.with(|cell| {
            let entities = &mut *cell.borrow_mut();

            if let Some(entity) = EntityManager::get_by_browser_id(browser_id, entities) {
                f(entity)
            } else {
                bail!("No entity found with browser id {}!", browser_id);
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

    pub fn with_entity<N, F, T>(target: N, f: F) -> Result<T>
    where
        F: FnOnce(&mut CefEntity) -> Result<T>,
        N: TargetEntity,
    {
        let entity_id = target.get_entity_id()?;

        ENTITIES.with(|cell| {
            let entities = &mut *cell.borrow_mut();

            if let Some(entity) = EntityManager::get_by_entity_id(entity_id, entities) {
                f(entity)
            } else {
                bail!("No entity found with id {}!", entity_id);
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
        ENTITIES.with(|cell| {
            let entities = &mut *cell.borrow_mut();

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
        ENTITIES.with(|cell| {
            let entities = &mut *cell.borrow_mut();

            f(entities)
        })
    }
}

pub trait TargetEntity {
    fn get_entity_id(&self) -> Result<usize>;
}

impl TargetEntity for usize {
    fn get_entity_id(&self) -> Result<usize> {
        Ok(*self)
    }
}

impl TargetEntity for &str {
    fn get_entity_id(&self) -> Result<usize> {
        NAME_TO_ID.with(|cell| {
            let name_to_id = &mut *cell.borrow_mut();

            if let Some(entity_id) = name_to_id.get(*self) {
                Ok::<_, Error>(*entity_id)
            } else {
                bail!("No entity found with name {:?}!", self);
            }
        })
    }
}

impl TargetEntity for String {
    fn get_entity_id(&self) -> Result<usize> {
        self.as_str().get_entity_id()
    }
}

impl TargetEntity for Vec3 {
    fn get_entity_id(&self) -> Result<usize> {
        EntityManager::with_closest(*self, |closest_entity| Ok(closest_entity.id))
    }
}

impl TargetEntity for Box<dyn TargetEntity> {
    fn get_entity_id(&self) -> Result<usize> {
        self.as_ref().get_entity_id()
    }
}

impl<'a> TargetEntity for (&'a clap::ArgMatches<'_>, &'a PlayerSnapshot) {
    fn get_entity_id(&self) -> Result<usize> {
        let &(matches, player) = self;
        if let Some(name) = matches.value_of("name") {
            name.get_entity_id()
        } else {
            player.eye_position.get_entity_id()
        }
    }
}
