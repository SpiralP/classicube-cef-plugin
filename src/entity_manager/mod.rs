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
    cef::{Cef, RustRefBrowser},
    error::*,
};
use classicube_sys::Vec3;
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
}

impl EntityManager {
    pub fn new() -> Self {
        let render_model_detour = RenderModelDetour::new();

        Self {
            model: None,
            render_model_detour,
            context_handler: ContextHandler::new(),
        }
    }

    pub fn initialize(&mut self) {
        self.context_handler.initialize();
        self.render_model_detour.initialize();
        self.model = Some(CefModel::register());
    }

    pub fn on_new_map_loaded(&mut self) {
        // remove all entities

        let ids: Vec<usize> = ENTITIES.with(|entities| {
            let entities = &*entities.borrow();

            entities.keys().copied().collect()
        });

        for id in &ids {
            Self::remove_entity(*id);
        }
    }

    pub fn shutdown(&mut self) {
        self.context_handler.shutdown();
        self.render_model_detour.shutdown();
        self.model.take();
        ENTITIES.with(|entities| {
            let entities = &mut *entities.borrow_mut();

            entities.clear();
        });
    }

    /// returns entity_id
    #[must_use]
    pub fn create_entity() -> usize {
        ENTITIES.with(|entities| {
            let entities = &mut *entities.borrow_mut();

            let entity_id = ENTITY_ID.with(|cell| {
                let entity_id = cell.get();
                cell.set(entity_id + 1);
                entity_id
            });

            let mut entity = CefEntity::register(entity_id);
            entity.set_scale(0.25);

            entities.insert(entity_id, entity);

            debug!("entity created {}", entity_id);

            entity_id
        })
    }

    pub fn attach_browser(entity_id: usize, browser: RustRefBrowser) {
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

    pub fn remove_entity(entity_id: usize) {
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

            AsyncManager::spawn_local_on_main_thread(async move {
                Cef::close_browser(&browser).await;
            });
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
}
