mod cef_paint;
mod context_handler;
mod entity;
mod helpers;
mod model;
mod render_model_hook;

pub use self::{cef_paint::cef_paint_callback, entity::CefEntity};
use self::{context_handler::ContextHandler, model::CefModel};
use crate::{
    async_manager,
    cef::{Cef, CefEvent, RustRefBrowser},
    chat::{hidden_communication::LightEntity, PlayerSnapshot},
    error::*,
    options::FRAME_RATE,
    players::{Player, PlayerTrait, WebPlayer},
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
    collections::{HashMap, VecDeque},
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
                        entity.player.on_page_loaded(entity.id, &browser);
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
                        entity
                            .player
                            .on_title_change(entity.id, &browser, title, entity.silent);
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

    fn attach_browser_to_entity(entity_id: usize, browser: RustRefBrowser) {
        let browser_id = browser.get_identifier();

        BROWSER_ID_TO_ENTITY_ID.with(|ids| {
            let ids = &mut *ids.borrow_mut();

            ids.insert(browser_id, entity_id);

            ENTITIES.with(|cell| {
                let entities = &mut *cell.borrow_mut();

                if let Some(entity) = entities.get_mut(&entity_id) {
                    entity.attach_browser(browser);
                } else {
                    error!("couldn't find entity for browser id {}", browser_id);
                    browser.close().unwrap();
                }
            });
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

    /// returns entity_id
    pub fn create_entity(
        input: &str,
        fps: u16,
        insecure: bool,
        resolution: Option<(u16, u16)>,
        autoplay: bool,
        should_send: bool,
        silent: bool,
    ) -> Result<usize> {
        let mut player = Player::from_input(input)?;
        player.set_autoplay(autoplay);

        Ok(Self::create_entity_player(
            player,
            fps,
            insecure,
            resolution,
            should_send,
            silent,
        )?)
    }

    // TODO make a higher ScreenEntity class, and have a builder for these flags
    #[allow(clippy::too_many_arguments)]
    pub fn create_named_entity<S: Into<String>>(
        name: S,
        input: &str,
        fps: u16,
        insecure: bool,
        resolution: Option<(u16, u16)>,
        autoplay: bool,
        should_send: bool,
        silent: bool,
    ) -> Result<usize> {
        let name = name.into();

        let entity_id = Self::create_entity(
            input,
            fps,
            insecure,
            resolution,
            autoplay,
            should_send,
            silent,
        )?;
        debug!("created named entity {:?} with id {}", name, entity_id);

        NAME_TO_ID.with(|cell| {
            let name_to_id = &mut *cell.borrow_mut();
            name_to_id.insert(name, entity_id);
        });

        Ok(entity_id)
    }

    fn create_attach_browser(
        entity_id: usize,
        url: String,
        fps: u16,
        insecure: bool,
        resolution: Option<(u16, u16)>,
    ) {
        async_manager::spawn_local_on_main_thread(async move {
            let result = async move {
                let browser = Cef::create_browser(url, fps, insecure).await?;

                if let Some((width, height)) = resolution {
                    Cef::resize_browser(&browser, width, height)?;
                }

                EntityManager::attach_browser_to_entity(entity_id, browser);

                Ok::<_, Error>(())
            };

            if let Err(e) = result.await {
                warn!("create_attach_browser: {}", e);
            }
        });
    }

    pub fn create_entity_player(
        mut player: Player,
        fps: u16,
        insecure: bool,
        resolution: Option<(u16, u16)>,
        should_send: bool,
        silent: bool,
    ) -> Result<usize> {
        let url = player.on_create();

        let entity_id = Self::get_new_id();

        ENTITIES.with(move |cell| {
            let entities = &mut *cell.borrow_mut();

            let entity =
                CefEntity::register(entity_id, player, VecDeque::new(), should_send, silent);
            debug!("entity created {}", entity_id);
            entities.insert(entity_id, entity);
        });

        Self::create_attach_browser(entity_id, url, fps, insecure, resolution);

        Ok(entity_id)
    }

    /// add item to queue
    ///
    /// if item was queued, returns the type-name of player,
    /// else returns None meaning we're about to play the item
    pub fn entity_queue(
        input: &str,
        entity_id: usize,
        autoplay: bool,
    ) -> Result<Option<&'static str>> {
        // this needs to determine if the current player was finished,
        // if it was then we play right away,
        // else we queue it for next

        let mut player = Player::from_input(input)?;
        player.set_autoplay(autoplay);

        let is_finished_playing = EntityManager::with_entity(entity_id, move |entity| {
            Ok(entity.player.is_finished_playing())
        })?;

        if is_finished_playing {
            Self::entity_play_player(player, entity_id)?;

            Ok(None)
        } else {
            let type_name = EntityManager::with_entity(entity_id, move |entity| {
                let type_name = player.type_name();
                entity.queue.push_back(player);
                Ok(type_name)
            })?;
            Ok(Some(type_name))
        }
    }

    pub fn entity_skip(entity_id: usize) -> Result<()> {
        let mut maybe_new_player =
            EntityManager::with_entity(entity_id, move |entity| Ok(entity.queue.pop_front()))?;

        if let Some(new_player) = maybe_new_player.take() {
            Self::entity_play_player(new_player, entity_id)?;
        } else {
            let is_finished_playing = EntityManager::with_entity(entity_id, move |entity| {
                Ok(entity.player.is_finished_playing())
            })?;

            if !is_finished_playing {
                // show blank page
                Self::entity_stop(entity_id)?;
            }
        }

        Ok(())
    }

    pub fn entity_stop(entity_id: usize) -> Result<()> {
        Self::entity_play_player(Player::Web(WebPlayer::blank_page()), entity_id)?;

        Ok(())
    }

    pub fn entity_play_player(mut player: Player, entity_id: usize) -> Result<()> {
        let url = player.on_create();

        // TODO move this into the Player enum's on_create
        let browser = EntityManager::with_entity(entity_id, |entity| {
            let browser = entity.browser.as_ref().chain_err(|| "no browser")?;

            if entity.player.type_name() == player.type_name() {
                // try to persist volume options
                //
                // only persist for same-type because if we went from a
                // Web player which has global volume to a Youtube, it would
                // make the youtube player global volume too
                let volume = entity.player.get_volume();
                let volume_mode = entity.player.get_volume_mode();
                entity.player = player;

                let _ignore = entity.player.set_volume(Some(browser), volume);
                let _ignore = entity.player.set_volume_mode(Some(browser), volume_mode);
            } else {
                entity.player = player;
            }

            Ok(browser.clone())
        })?;

        browser.load_url(url)?;

        Ok(())
    }

    /// returns entity_id
    pub async fn create_entity_from_light_entity(mut info: LightEntity) -> Result<usize> {
        let (pos, ang, scale, size, resolution) =
            (info.pos, info.ang, info.scale, info.size, info.resolution);

        let url = info.player.on_create();

        let entity_id = Self::get_new_id();

        ENTITIES.with(|cell| {
            let entities = &mut *cell.borrow_mut();

            let entity = CefEntity::register(entity_id, info.player, info.queue, true, info.silent);
            debug!("entity {} created", entity_id);
            entities.insert(entity_id, entity);
        });

        EntityManager::with_entity(entity_id, |entity| {
            let e = &mut entity.entity;

            e.Position.set(pos[0], pos[1], pos[2]);

            e.RotX = ang[0];
            e.RotY = ang[1];
            entity.set_scale(scale);
            entity.set_size(size.0, size.1);

            Self::create_attach_browser(entity_id, url, FRAME_RATE.get()?, false, resolution);

            Ok(entity_id)
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
        let entity_ids: Vec<usize> = ENTITIES.with(|cell| {
            let entities = &*cell.borrow();

            entities.keys().copied().collect()
        });

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
