mod media;
mod web;
mod youtube;

pub use self::{media::MediaPlayer, web::WebPlayer, youtube::YoutubePlayer};
use crate::{cef::RustRefBrowser, error::*};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub trait PlayerTrait {
    fn from_input(input: &str) -> Result<Self>
    where
        Self: Sized + Clone;

    /// Called after entity is created, given an entity_id
    ///
    /// Called before creating the browser, returns a url
    fn on_create(&mut self, _entity_id: usize) -> String;

    /// Called after page is loaded
    fn on_page_loaded(&mut self, _browser: &mut RustRefBrowser) {}

    fn on_title_change(&mut self, _browser: &mut RustRefBrowser, _title: String) {}

    fn get_current_time(&self, _browser: &mut RustRefBrowser) -> Result<Duration> {
        bail!("getting time not supported");
    }

    fn set_current_time(&mut self, _browser: &mut RustRefBrowser, _time: Duration) -> Result<()> {
        bail!("setting time not supported");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Player {
    Youtube(YoutubePlayer),
    Media(MediaPlayer),
    Web(WebPlayer),
}

impl PlayerTrait for Player {
    fn from_input(input: &str) -> Result<Self> {
        match YoutubePlayer::from_input(input) {
            Ok(player) => Ok(Player::Youtube(player)),
            Err(_) => {
                match MediaPlayer::from_input(input) {
                    Ok(player) => Ok(Player::Media(player)),
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
        }
    }

    fn on_create(&mut self, entity_id: usize) -> String {
        match self {
            Player::Youtube(player) => player.on_create(entity_id),
            Player::Media(player) => player.on_create(entity_id),
            Player::Web(player) => player.on_create(entity_id),
        }
    }

    fn on_page_loaded(&mut self, browser: &mut RustRefBrowser) {
        match self {
            Player::Youtube(player) => player.on_page_loaded(browser),
            Player::Media(player) => player.on_page_loaded(browser),
            Player::Web(player) => player.on_page_loaded(browser),
        }
    }

    fn on_title_change(&mut self, browser: &mut RustRefBrowser, title: String) {
        match self {
            Player::Youtube(player) => player.on_title_change(browser, title),
            Player::Media(player) => player.on_title_change(browser, title),
            Player::Web(player) => player.on_title_change(browser, title),
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
