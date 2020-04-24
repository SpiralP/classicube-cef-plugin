mod web;
mod youtube;

pub use self::{web::WebPlayer, youtube::YoutubePlayer};
use crate::{async_manager::AsyncManager, cef::Cef, entity_manager::EntityManager, error::*};
use serde::{Deserialize, Serialize};
use std::any::Any;

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

impl PlayerTrait for Player {
    fn from_input(input: &str) -> Result<Self> {
        match YoutubePlayer::from_input(input) {
            Ok(player) => Ok(Player::Youtube(player)),
            Err(_) => {
                match WebPlayer::from_input(input) {
                    Ok(player) => Ok(Player::Web(player)),

                    Err(e) => {
                        if !input.starts_with("http") {
                            // if it didn't start with http, try again with https:// in front
                            Player::from_input(&format!("https://{}", input))
                        } else {
                            bail!("no player matched for input: {}", e);
                        }
                    }
                }
            }
        }
    }

    fn on_create(&mut self) -> String {
        match self {
            Player::Youtube(player) => player.on_create(),
            Player::Web(player) => player.on_create(),
        }
    }
}

#[test]
fn test_create_player() {
    let good_web = [
        "https://www.classicube.net/",
        "www.classicube.net/",
        "https://youtube.com/",
    ];

    for url in &good_web {
        let player: Player = Player::from_input(url).unwrap();
        if let Player::Web(_) = player {
        } else {
            panic!("not Web");
        }
    }

    let good_youtube = [
        "https://www.youtube.com/watch?v=9pkD2czKTjE",
        "www.youtube.com/watch?v=9pkD2czKTjE",
    ];

    for url in &good_youtube {
        let player: Player = Player::from_input(url).unwrap();
        if let Player::Youtube(_) = player {
        } else {
            panic!("not Youtube");
        }
    }
}

/// Create an entity screen, start rendering a loading screen
/// while we create a cef browser and wait for it to start rendering to it.
///
/// returns browser_id
pub fn create(input: &str) -> Result<usize> {
    let mut player = Player::from_input(input)?;
    let url = player.on_create();

    let entity_id = EntityManager::create_entity(player);

    AsyncManager::spawn_local_on_main_thread(async move {
        let browser = Cef::create_browser(url).await;

        EntityManager::attach_browser(entity_id, browser);
    });

    Ok(entity_id)
}

pub fn play(input: &str, entity_id: usize) -> Result<()> {
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

// pub fn on_browser_page_loaded(_browser: RustRefBrowser) {
//     // let browser_id = browser.get_identifier();

//     // PLAYERS.with(|players| {
//     //     let players = &mut *players.borrow_mut();

//     //     if let Some((browser, player)) = players.get_mut(&browser_id) {
//     //         player.on_page_loaded(browser);
//     //     }
//     // });
// }
