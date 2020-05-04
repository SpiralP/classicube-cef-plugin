mod helpers;
mod media;
mod mute_lose_focus;
mod volume_fade;
mod web;
mod youtube;

pub use self::{
    media::MediaPlayer, mute_lose_focus::IS_FOCUSED, web::WebPlayer, youtube::YoutubePlayer,
};
use crate::{cef::RustRefBrowser, error::*};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub trait PlayerTrait {
    fn from_input(input: &str) -> Result<Self>
    where
        Self: Sized + Clone;

    /// Called before creating the browser, returns a url
    fn on_create(&mut self) -> String;

    /// Called after page is loaded
    fn on_page_loaded(&mut self, _entity_id: usize, _browser: &RustRefBrowser) {}

    fn on_title_change(&mut self, _entity_id: usize, _browser: &RustRefBrowser, _title: String) {}

    fn get_current_time(&self, _browser: &RustRefBrowser) -> Result<Duration> {
        bail!("getting time not supported");
    }

    fn set_current_time(&mut self, _browser: &RustRefBrowser, _time: Duration) -> Result<()> {
        bail!("setting time not supported");
    }

    fn get_volume(&self, _browser: &RustRefBrowser) -> Result<f32> {
        Ok(1.0)
    }

    fn set_volume(&mut self, _browser: &RustRefBrowser, _percent: f32) -> Result<()> {
        bail!("setting volume not supported");
    }

    fn has_global_volume(&self) -> bool {
        true
    }

    fn set_global_volume(&mut self, _global_volume: bool) -> Result<()> {
        bail!("setting global volume not supported");
    }

    fn get_should_send(&self) -> bool;

    fn set_should_send(&mut self, _should_send: bool);
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

    fn on_create(&mut self) -> String {
        match self {
            Player::Youtube(player) => player.on_create(),
            Player::Media(player) => player.on_create(),
            Player::Web(player) => player.on_create(),
        }
    }

    fn on_page_loaded(&mut self, entity_id: usize, browser: &RustRefBrowser) {
        match self {
            Player::Youtube(player) => player.on_page_loaded(entity_id, browser),
            Player::Media(player) => player.on_page_loaded(entity_id, browser),
            Player::Web(player) => player.on_page_loaded(entity_id, browser),
        }
    }

    fn on_title_change(&mut self, entity_id: usize, browser: &RustRefBrowser, title: String) {
        match self {
            Player::Youtube(player) => player.on_title_change(entity_id, browser, title),
            Player::Media(player) => player.on_title_change(entity_id, browser, title),
            Player::Web(player) => player.on_title_change(entity_id, browser, title),
        }
    }

    fn get_current_time(&self, browser: &RustRefBrowser) -> Result<Duration> {
        match self {
            Player::Youtube(player) => player.get_current_time(browser),
            Player::Media(player) => player.get_current_time(browser),
            Player::Web(player) => player.get_current_time(browser),
        }
    }

    fn set_current_time(&mut self, browser: &RustRefBrowser, time: Duration) -> Result<()> {
        match self {
            Player::Youtube(player) => player.set_current_time(browser, time),
            Player::Media(player) => player.set_current_time(browser, time),
            Player::Web(player) => player.set_current_time(browser, time),
        }
    }

    fn get_volume(&self, browser: &RustRefBrowser) -> Result<f32> {
        match self {
            Player::Youtube(player) => player.get_volume(browser),
            Player::Media(player) => player.get_volume(browser),
            Player::Web(player) => player.get_volume(browser),
        }
    }

    fn set_volume(&mut self, browser: &RustRefBrowser, percent: f32) -> Result<()> {
        match self {
            Player::Youtube(player) => player.set_volume(browser, percent),
            Player::Media(player) => player.set_volume(browser, percent),
            Player::Web(player) => player.set_volume(browser, percent),
        }
    }

    fn has_global_volume(&self) -> bool {
        match self {
            Player::Youtube(player) => player.has_global_volume(),
            Player::Media(player) => player.has_global_volume(),
            Player::Web(player) => player.has_global_volume(),
        }
    }

    fn set_global_volume(&mut self, global_volume: bool) -> Result<()> {
        match self {
            Player::Youtube(player) => player.set_global_volume(global_volume),
            Player::Media(player) => player.set_global_volume(global_volume),
            Player::Web(player) => player.set_global_volume(global_volume),
        }
    }

    fn get_should_send(&self) -> bool {
        match self {
            Player::Youtube(player) => player.get_should_send(),
            Player::Media(player) => player.get_should_send(),
            Player::Web(player) => player.get_should_send(),
        }
    }

    fn set_should_send(&mut self, should_send: bool) {
        match self {
            Player::Youtube(player) => player.set_should_send(should_send),
            Player::Media(player) => player.set_should_send(should_send),
            Player::Web(player) => player.set_should_send(should_send),
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

pub fn initialize() {
    mute_lose_focus::initialize();
}

pub fn on_new_map() {
    volume_fade::on_new_map();
}

pub fn on_new_map_loaded() {
    volume_fade::on_new_map_loaded();
}

pub fn shutdown() {
    mute_lose_focus::shutdown();
}
