mod builder;
mod dash;
mod helpers;
mod media;
mod volume_fade;
mod web;
mod youtube;

pub use self::{
    builder::PlayerBuilder, dash::DashPlayer, media::MediaPlayer, web::WebPlayer,
    youtube::YoutubePlayer,
};
use crate::{cef::RustRefBrowser, error::*};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub trait PlayerTrait: Clone {
    fn type_name(&self) -> &'static str;

    fn from_input(input: &str) -> Result<Self>
    where
        Self: Sized;

    /// Called before creating the browser, returns a url
    fn on_create(&mut self) -> String;

    /// Called after page is loaded
    fn on_page_loaded(&mut self, _entity_id: usize, _browser: &RustRefBrowser) {}

    fn on_title_change(&mut self, _entity_id: usize, _browser: &RustRefBrowser, _title: String) {}

    fn get_current_time(&self) -> Result<Duration> {
        bail!("getting time not supported");
    }
    fn set_current_time(&mut self, _browser: &RustRefBrowser, _time: Duration) -> Result<()> {
        bail!("setting time not supported");
    }

    fn get_volume(&self) -> f32 {
        1.0
    }
    fn set_volume(&mut self, _browser: Option<&RustRefBrowser>, _percent: f32) -> Result<()> {
        bail!("setting volume not supported");
    }

    fn get_volume_mode(&self) -> VolumeMode {
        VolumeMode::Global
    }
    fn set_volume_mode(
        &mut self,
        _browser: Option<&RustRefBrowser>,
        _mode: VolumeMode,
    ) -> Result<()> {
        bail!("setting volume mode not supported");
    }

    fn get_autoplay(&self) -> bool {
        true
    }
    fn set_autoplay(&mut self, _browser: Option<&RustRefBrowser>, autoplay: bool) -> Result<()> {
        if !autoplay {
            bail!("unsetting autoplay not supported");
        } else {
            Ok(())
        }
    }

    fn get_loop(&self) -> bool {
        false
    }
    fn set_loop(&mut self, _browser: Option<&RustRefBrowser>, should_loop: bool) -> Result<()> {
        if should_loop {
            bail!("looping unsupported");
        } else {
            Ok(())
        }
    }

    fn get_url(&self) -> String;

    fn get_title(&self) -> String;

    fn is_finished_playing(&self) -> bool;

    fn set_playing(&mut self, _browser: &RustRefBrowser, playing: bool) -> Result<()> {
        if !playing {
            bail!("pausing not supported");
        } else {
            Ok(())
        }
    }

    fn set_silent(&mut self, silent: bool) -> Result<()> {
        if silent {
            bail!("setting silent unsupported");
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
pub enum VolumeMode {
    Global,
    Distance { distance: f32 },
    Panning { distance: f32, pan: f32 },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Player {
    Youtube(YoutubePlayer),
    Dash(DashPlayer),
    Media(MediaPlayer),
    Web(WebPlayer),
}

impl PlayerTrait for Player {
    fn type_name(&self) -> &'static str {
        match self {
            Player::Youtube(player) => player.type_name(),
            Player::Dash(player) => player.type_name(),
            Player::Media(player) => player.type_name(),
            Player::Web(player) => player.type_name(),
        }
    }

    fn from_input(input: &str) -> Result<Self> {
        match YoutubePlayer::from_input(input) {
            Ok(player) => Ok(Player::Youtube(player)),
            Err(_) => {
                match DashPlayer::from_input(input) {
                    Ok(player) => Ok(Player::Dash(player)),
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
        }
    }

    fn on_create(&mut self) -> String {
        match self {
            Player::Youtube(player) => player.on_create(),
            Player::Dash(player) => player.on_create(),
            Player::Media(player) => player.on_create(),
            Player::Web(player) => player.on_create(),
        }
    }

    fn on_page_loaded(&mut self, entity_id: usize, browser: &RustRefBrowser) {
        match self {
            Player::Youtube(player) => player.on_page_loaded(entity_id, browser),
            Player::Dash(player) => player.on_page_loaded(entity_id, browser),
            Player::Media(player) => player.on_page_loaded(entity_id, browser),
            Player::Web(player) => player.on_page_loaded(entity_id, browser),
        }
    }

    fn on_title_change(&mut self, entity_id: usize, browser: &RustRefBrowser, title: String) {
        match self {
            Player::Youtube(player) => player.on_title_change(entity_id, browser, title),
            Player::Dash(player) => player.on_title_change(entity_id, browser, title),
            Player::Media(player) => player.on_title_change(entity_id, browser, title),
            Player::Web(player) => player.on_title_change(entity_id, browser, title),
        }
    }

    fn get_current_time(&self) -> Result<Duration> {
        match self {
            Player::Youtube(player) => player.get_current_time(),
            Player::Dash(player) => player.get_current_time(),
            Player::Media(player) => player.get_current_time(),
            Player::Web(player) => player.get_current_time(),
        }
    }

    fn set_current_time(&mut self, browser: &RustRefBrowser, time: Duration) -> Result<()> {
        match self {
            Player::Youtube(player) => player.set_current_time(browser, time),
            Player::Dash(player) => player.set_current_time(browser, time),
            Player::Media(player) => player.set_current_time(browser, time),
            Player::Web(player) => player.set_current_time(browser, time),
        }
    }

    fn get_volume(&self) -> f32 {
        match self {
            Player::Youtube(player) => player.get_volume(),
            Player::Dash(player) => player.get_volume(),
            Player::Media(player) => player.get_volume(),
            Player::Web(player) => player.get_volume(),
        }
    }

    fn set_volume(&mut self, browser: Option<&RustRefBrowser>, percent: f32) -> Result<()> {
        match self {
            Player::Youtube(player) => player.set_volume(browser, percent),
            Player::Dash(player) => player.set_volume(browser, percent),
            Player::Media(player) => player.set_volume(browser, percent),
            Player::Web(player) => player.set_volume(browser, percent),
        }
    }

    fn get_volume_mode(&self) -> VolumeMode {
        match self {
            Player::Youtube(player) => player.get_volume_mode(),
            Player::Dash(player) => player.get_volume_mode(),
            Player::Media(player) => player.get_volume_mode(),
            Player::Web(player) => player.get_volume_mode(),
        }
    }

    fn set_volume_mode(
        &mut self,
        browser: Option<&RustRefBrowser>,
        mode: VolumeMode,
    ) -> Result<()> {
        match self {
            Player::Youtube(player) => player.set_volume_mode(browser, mode),
            Player::Dash(player) => player.set_volume_mode(browser, mode),
            Player::Media(player) => player.set_volume_mode(browser, mode),
            Player::Web(player) => player.set_volume_mode(browser, mode),
        }
    }

    fn get_autoplay(&self) -> bool {
        match self {
            Player::Youtube(player) => player.get_autoplay(),
            Player::Dash(player) => player.get_autoplay(),
            Player::Media(player) => player.get_autoplay(),
            Player::Web(player) => player.get_autoplay(),
        }
    }

    fn set_autoplay(&mut self, browser: Option<&RustRefBrowser>, autoplay: bool) -> Result<()> {
        match self {
            Player::Youtube(player) => player.set_autoplay(browser, autoplay),
            Player::Dash(player) => player.set_autoplay(browser, autoplay),
            Player::Media(player) => player.set_autoplay(browser, autoplay),
            Player::Web(player) => player.set_autoplay(browser, autoplay),
        }
    }

    fn get_loop(&self) -> bool {
        match self {
            Player::Youtube(player) => player.get_loop(),
            Player::Dash(player) => player.get_loop(),
            Player::Media(player) => player.get_loop(),
            Player::Web(player) => player.get_loop(),
        }
    }

    fn set_loop(&mut self, browser: Option<&RustRefBrowser>, should_loop: bool) -> Result<()> {
        match self {
            Player::Youtube(player) => player.set_loop(browser, should_loop),
            Player::Dash(player) => player.set_loop(browser, should_loop),
            Player::Media(player) => player.set_loop(browser, should_loop),
            Player::Web(player) => player.set_loop(browser, should_loop),
        }
    }

    fn get_url(&self) -> String {
        match self {
            Player::Youtube(player) => player.get_url(),
            Player::Dash(player) => player.get_url(),
            Player::Media(player) => player.get_url(),
            Player::Web(player) => player.get_url(),
        }
    }

    fn get_title(&self) -> String {
        match self {
            Player::Youtube(player) => player.get_title(),
            Player::Dash(player) => player.get_title(),
            Player::Media(player) => player.get_title(),
            Player::Web(player) => player.get_title(),
        }
    }

    fn is_finished_playing(&self) -> bool {
        match self {
            Player::Youtube(player) => player.is_finished_playing(),
            Player::Dash(player) => player.is_finished_playing(),
            Player::Media(player) => player.is_finished_playing(),
            Player::Web(player) => player.is_finished_playing(),
        }
    }

    fn set_playing(&mut self, browser: &RustRefBrowser, playing: bool) -> Result<()> {
        match self {
            Player::Youtube(player) => player.set_playing(browser, playing),
            Player::Dash(player) => player.set_playing(browser, playing),
            Player::Media(player) => player.set_playing(browser, playing),
            Player::Web(player) => player.set_playing(browser, playing),
        }
    }

    fn set_silent(&mut self, silent: bool) -> Result<()> {
        match self {
            Player::Youtube(player) => player.set_silent(silent),
            Player::Dash(player) => player.set_silent(silent),
            Player::Media(player) => player.set_silent(silent),
            Player::Web(player) => player.set_silent(silent),
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

pub fn on_new_map() {
    volume_fade::on_new_map();
}

pub fn on_new_map_loaded() {
    volume_fade::on_new_map_loaded();
}