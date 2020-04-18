mod web;
mod youtube;

use self::{web::WebPlayer, youtube::YoutubePlayer};
use crate::{
    async_manager::AsyncManager,
    cef::{RustRefBrowser, CEF},
    entity_manager::EntityManager,
    error::*,
};
use classicube_helpers::with_inner::WithInner;
use log::debug;
use std::{cell::RefCell, collections::HashMap, os::raw::c_int};

thread_local!(
    #[allow(clippy::type_complexity)]
    static PLAYERS: RefCell<HashMap<c_int, (RustRefBrowser, Box<dyn Player>)>> =
        RefCell::new(HashMap::new());
);

pub trait Player {
    fn from_input(input: &str) -> Result<Self>
    where
        Self: Sized;

    /// Called before creating the browser, returns a url
    fn on_create(&mut self) -> String;

    // /// Called after page is loaded
    // fn on_page_loaded(&mut self, _browser: &mut RustRefBrowser) {}
}

fn create_player(input: &str) -> Result<Box<dyn Player>> {
    if let Ok(player) = YoutubePlayer::from_input(input) {
        return Ok(Box::new(player));
    }

    Ok(Box::new(WebPlayer::from_input(input)?))
}

/// Create an entity screen, start rendering a loading screen
/// while we create a cef browser and wait for it start rendering to it.
///
/// returns browser_id
pub fn create(input: &str) -> Result<usize> {
    let entity_id = EntityManager::create_entity();

    let mut player = create_player(input)?;
    let url = player.on_create();

    let browser_create_future = CEF
        .with_inner(|cef| cef.create_browser(url))
        .chain_err(|| "CEF not initialized")?;

    AsyncManager::spawn_local_on_main_thread(async move {
        let browser = browser_create_future.await;
        EntityManager::attach_browser(entity_id, browser.clone());

        let browser_id = browser.get_identifier();

        PLAYERS.with(move |cell| {
            let players = &mut *cell.borrow_mut();
            players.insert(browser_id, (browser, player));
        });
    });

    Ok(entity_id)
}

pub fn load(input: &str, entity_id: usize) -> Result<()> {
    let mut player = create_player(input)?;
    let url = player.on_create();

    let browser = EntityManager::get_browser_by_entity_id(entity_id)?;

    browser.load_url(url)?;
    let browser_id = browser.get_identifier();

    PLAYERS.with(move |cell| {
        let players = &mut *cell.borrow_mut();
        players.insert(browser_id, (browser, player));
    });

    Ok(())
}

// pub fn on_browser_page_loaded(_browser: RustRefBrowser) {
//     // let browser_id = browser.get_identifier();

//     // PLAYERS.with(|players| {
//     //     let players = &mut *players.borrow_mut();

//     //     if let Some((browser, player)) = players.get_mut(&browser_id) {
//     //         player.on_page_loaded(browser);
//     //     }
//     // });
// }

pub fn shutdown() {
    debug!("shutdown players");

    PLAYERS.with(move |cell| {
        let players = &mut *cell.borrow_mut();
        players.clear();
    });
}
