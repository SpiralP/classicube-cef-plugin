mod web;
mod youtube;

pub use self::{web::WebPlayer, youtube::YoutubePlayer};
use crate::{
    async_manager::AsyncManager,
    cef::{Cef, RustRefBrowser},
    entity_manager::EntityManager,
    error::*,
};
use log::debug;
use serde::{Deserialize, Serialize};
use std::{any::Any, cell::RefCell, collections::HashMap, os::raw::c_int};

thread_local!(
    #[allow(clippy::type_complexity)]
    static PLAYERS: RefCell<HashMap<c_int, (RustRefBrowser, Box<dyn PlayerTrait>)>> =
        RefCell::new(HashMap::new());
);

pub trait PlayerTrait: Any {
    fn from_input(input: &str) -> Result<Self>
    where
        Self: Sized;

    /// Called before creating the browser, returns a url
    fn on_create(&mut self) -> String;

    // /// Called after page is loaded
    // fn on_page_loaded(&mut self, _browser: &mut RustRefBrowser) {}
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Player {
    Youtube(YoutubePlayer),
    Web(WebPlayer),
}

fn create_player(input: &str) -> Result<Box<dyn PlayerTrait>> {
    match YoutubePlayer::from_input(input) {
        Ok(player) => Ok(Box::new(player)),
        Err(_) => match WebPlayer::from_input(input) {
            Ok(player) => Ok(Box::new(player)),

            Err(e) => {
                if !input.starts_with("http") {
                    // if it didn't start with http, try again with https:// in front
                    create_player(&format!("https://{}", input))
                } else {
                    bail!("no player matched for input: {}", e);
                }
            }
        },
    }
}

#[test]
fn test_create_player() {
    use std::any::TypeId;

    let good_web = [
        "https://www.classicube.net/",
        "www.classicube.net/",
        "https://youtube.com/",
    ];

    for url in &good_web {
        let player: Box<dyn PlayerTrait> = create_player(url).unwrap();
        assert_eq!((*player).type_id(), TypeId::of::<WebPlayer>());
    }

    let good_youtube = [
        "https://www.youtube.com/watch?v=9pkD2czKTjE",
        "www.youtube.com/watch?v=9pkD2czKTjE",
    ];

    for url in &good_youtube {
        let player: Box<dyn PlayerTrait> = create_player(url).unwrap();
        assert_eq!((*player).type_id(), TypeId::of::<YoutubePlayer>());
    }
}

/// Create an entity screen, start rendering a loading screen
/// while we create a cef browser and wait for it start rendering to it.
///
/// returns browser_id
pub fn create(input: &str) -> Result<usize> {
    let entity_id = EntityManager::create_entity();

    let mut player = create_player(input)?;
    let url = player.on_create();

    AsyncManager::spawn_local_on_main_thread(async move {
        let browser = Cef::create_browser(url).await;

        EntityManager::attach_browser(entity_id, browser.clone());

        let browser_id = browser.get_identifier();

        PLAYERS.with(move |cell| {
            let players = &mut *cell.borrow_mut();
            players.insert(browser_id, (browser, player));
        });
    });

    Ok(entity_id)
}

pub fn play(input: &str, entity_id: usize) -> Result<()> {
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
