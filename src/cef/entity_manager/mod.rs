mod cef_paint;
mod context_handler;
mod entity;
mod model;
mod render_model_detour;

pub use self::{cef_paint::cef_paint_callback, entity::CefEntity};
use self::{
    context_handler::ContextHandler, model::CefModel, render_model_detour::RenderModelDetour,
};
use crate::cef::interface::RustRefBrowser;
use classicube_sys::Vec3;
use std::{cell::RefCell, collections::HashMap, os::raw::*};

pub const CEF_WIDTH: u32 = 1920;
pub const CEF_HEIGHT: u32 = 1080;

pub const TEXTURE_WIDTH: usize = 2048;
pub const TEXTURE_HEIGHT: usize = 2048;

// TODO entity spawns first, then browser!
// browser_id, entity
thread_local!(
    pub static ENTITIES: RefCell<HashMap<c_int, (RustRefBrowser, CefEntity)>> =
        RefCell::new(HashMap::new());
);

pub struct CefEntityManager {
    // model is just the shape, the entities holds the texture id and scaling
    model: Option<CefModel>,

    render_model_detour: RenderModelDetour,
    context_handler: ContextHandler,
}

impl CefEntityManager {
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

    pub fn shutdown(&mut self) {
        self.context_handler.shutdown();
        self.render_model_detour.shutdown();
        self.model.take();
        ENTITIES.with(|entities| {
            let entities = &mut *entities.borrow_mut();

            entities.clear();
        });
    }

    pub fn create_entity(browser: RustRefBrowser) {
        let browser_id = browser.get_identifier();

        ENTITIES.with(|entities| {
            let entities = &mut *entities.borrow_mut();

            let mut entity = CefEntity::register();
            entity.set_scale(0.25);

            entities.insert(browser_id, (browser, entity));
        });
    }

    pub fn on_browser_close(browser: RustRefBrowser) {
        let browser_id = browser.get_identifier();

        ENTITIES.with(|entities| {
            let entities = &mut *entities.borrow_mut();

            entities.remove(&browser_id);
        });
    }

    pub fn get_closest_mut(
        position: Vec3,
        entities: &mut HashMap<c_int, (RustRefBrowser, CefEntity)>,
    ) -> Option<(c_int, &mut RustRefBrowser, &mut CefEntity)> {
        let mut last_distance = None;
        let mut closest = None;

        for (id, (browser, entity)) in entities {
            let distance = (position - entity.entity.Position).length_squared();

            if let Some(last_distance) = last_distance.as_mut() {
                if distance < *last_distance {
                    *last_distance = distance;
                    closest = Some((*id, browser, entity));
                }
            } else {
                last_distance = Some(distance);
                closest = Some((*id, browser, entity));
            }
        }

        closest
    }
}
