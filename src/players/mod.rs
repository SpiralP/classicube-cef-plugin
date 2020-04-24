mod web;
mod youtube;

pub use self::{web::WebPlayer, youtube::YoutubePlayer};
use crate::{cef::RustRefBrowser, error::*};
use serde::{Deserialize, Serialize};

pub trait PlayerTrait: Clone {
    fn from_input(input: &str) -> Result<Self>
    where
        Self: Sized;

    /// Called before creating the browser, returns a url
    fn on_create(&mut self) -> String;

    /// Called after page is loaded
    fn on_page_loaded(&mut self, _browser: &mut RustRefBrowser) {}

    fn on_tick(&mut self) {}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    fn on_page_loaded(&mut self, browser: &mut RustRefBrowser) {
        match self {
            Player::Youtube(player) => player.on_page_loaded(browser),
            Player::Web(player) => player.on_page_loaded(browser),
        }
    }

    fn on_tick(&mut self) {
        match self {
            Player::Youtube(player) => player.on_tick(),
            Player::Web(player) => player.on_tick(),
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

// pub fn on_browser_page_loaded(_browser: RustRefBrowser) {
//     // let browser_id = browser.get_identifier();

//     // PLAYERS.with(|players| {
//     //     let players = &mut *players.borrow_mut();

//     //     if let Some((browser, player)) = players.get_mut(&browser_id) {
//     //         player.on_page_loaded(browser);
//     //     }
//     // });
// }
