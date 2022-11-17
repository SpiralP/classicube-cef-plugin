mod builder;
mod dash;
mod helpers;
mod hls;
mod image;
mod media;
mod volume_fade;
mod web;
mod youtube;

use std::time::Duration;

use serde::{Deserialize, Serialize};

pub use self::{
    builder::PlayerBuilder, dash::DashPlayer, hls::HlsPlayer, image::ImagePlayer,
    media::MediaPlayer, web::WebPlayer, youtube::YouTubePlayer,
};
use crate::{
    cef::RustRefBrowser,
    error::{bail, Result},
};

pub trait PlayerTrait: Clone {
    fn type_name(&self) -> &'static str;

    fn from_input(input: &str) -> Result<Self>
    where
        Self: Sized;

    /// Called before creating the browser, returns a url
    fn on_create(&mut self) -> Result<String>;

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
        if autoplay {
            Ok(())
        } else {
            bail!("unsetting autoplay not supported");
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
        if playing {
            Ok(())
        } else {
            bail!("pausing not supported");
        }
    }

    fn set_silent(&mut self, silent: bool) -> Result<()> {
        if silent {
            bail!("setting silent unsupported");
        } else {
            Ok(())
        }
    }

    fn set_speed(&mut self, _browser: Option<&RustRefBrowser>, speed: f32) -> Result<()> {
        if (speed - 1.0).abs() > 0.01 {
            bail!("setting speed unsupported");
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
pub enum VolumeMode {
    Global,
    Distance {
        multiplier: f32,
        distance: f32,
    },
    Panning {
        multiplier: f32,
        distance: f32,
        pan: f32,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Player {
    YouTube(YouTubePlayer),
    Dash(DashPlayer),
    Hls(HlsPlayer),
    Media(MediaPlayer),
    Image(ImagePlayer),
    Web(WebPlayer),
}

impl PlayerTrait for Player {
    fn type_name(&self) -> &'static str {
        match self {
            Player::YouTube(player) => player.type_name(),
            Player::Dash(player) => player.type_name(),
            Player::Hls(player) => player.type_name(),
            Player::Media(player) => player.type_name(),
            Player::Image(player) => player.type_name(),
            Player::Web(player) => player.type_name(),
        }
    }

    fn from_input(input: &str) -> Result<Self> {
        match YouTubePlayer::from_input(input) {
            Ok(player) => Ok(Player::YouTube(player)),
            Err(_) => {
                match DashPlayer::from_input(input) {
                    Ok(player) => Ok(Player::Dash(player)),
                    Err(_) => {
                        match HlsPlayer::from_input(input) {
                            Ok(player) => Ok(Player::Hls(player)),
                            Err(_) => {
                                match MediaPlayer::from_input(input) {
                                    Ok(player) => Ok(Player::Media(player)),
                                    Err(_) => {
                                        match ImagePlayer::from_input(input) {
                                            Ok(player) => Ok(Player::Image(player)),
                                            Err(_) => {
                                                match WebPlayer::from_input(input) {
                                                    Ok(player) => Ok(Player::Web(player)),

                                                    Err(e) => {
                                                        if input.starts_with("http") {
                                                            bail!(
                                                                "no player matched for input: {}",
                                                                e
                                                            );
                                                        } else {
                                                            // if it didn't start with http, try again with https:// in front
                                                            Player::from_input(&format!(
                                                                "https://{}",
                                                                input
                                                            ))
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
                }
            }
        }
    }

    fn on_create(&mut self) -> Result<String> {
        match self {
            Player::YouTube(player) => player.on_create(),
            Player::Dash(player) => player.on_create(),
            Player::Hls(player) => player.on_create(),
            Player::Media(player) => player.on_create(),
            Player::Image(player) => player.on_create(),
            Player::Web(player) => player.on_create(),
        }
    }

    fn on_page_loaded(&mut self, entity_id: usize, browser: &RustRefBrowser) {
        match self {
            Player::YouTube(player) => player.on_page_loaded(entity_id, browser),
            Player::Dash(player) => player.on_page_loaded(entity_id, browser),
            Player::Hls(player) => player.on_page_loaded(entity_id, browser),
            Player::Media(player) => player.on_page_loaded(entity_id, browser),
            Player::Image(player) => player.on_page_loaded(entity_id, browser),
            Player::Web(player) => player.on_page_loaded(entity_id, browser),
        }
    }

    fn on_title_change(&mut self, entity_id: usize, browser: &RustRefBrowser, title: String) {
        match self {
            Player::YouTube(player) => player.on_title_change(entity_id, browser, title),
            Player::Dash(player) => player.on_title_change(entity_id, browser, title),
            Player::Hls(player) => player.on_title_change(entity_id, browser, title),
            Player::Media(player) => player.on_title_change(entity_id, browser, title),
            Player::Image(player) => player.on_title_change(entity_id, browser, title),
            Player::Web(player) => player.on_title_change(entity_id, browser, title),
        }
    }

    fn get_current_time(&self) -> Result<Duration> {
        match self {
            Player::YouTube(player) => player.get_current_time(),
            Player::Dash(player) => player.get_current_time(),
            Player::Hls(player) => player.get_current_time(),
            Player::Media(player) => player.get_current_time(),
            Player::Image(player) => player.get_current_time(),
            Player::Web(player) => player.get_current_time(),
        }
    }

    fn set_current_time(&mut self, browser: &RustRefBrowser, time: Duration) -> Result<()> {
        match self {
            Player::YouTube(player) => player.set_current_time(browser, time),
            Player::Dash(player) => player.set_current_time(browser, time),
            Player::Hls(player) => player.set_current_time(browser, time),
            Player::Media(player) => player.set_current_time(browser, time),
            Player::Image(player) => player.set_current_time(browser, time),
            Player::Web(player) => player.set_current_time(browser, time),
        }
    }

    fn get_volume(&self) -> f32 {
        match self {
            Player::YouTube(player) => player.get_volume(),
            Player::Dash(player) => player.get_volume(),
            Player::Hls(player) => player.get_volume(),
            Player::Media(player) => player.get_volume(),
            Player::Image(player) => player.get_volume(),
            Player::Web(player) => player.get_volume(),
        }
    }

    fn set_volume(&mut self, browser: Option<&RustRefBrowser>, percent: f32) -> Result<()> {
        match self {
            Player::YouTube(player) => player.set_volume(browser, percent),
            Player::Dash(player) => player.set_volume(browser, percent),
            Player::Hls(player) => player.set_volume(browser, percent),
            Player::Media(player) => player.set_volume(browser, percent),
            Player::Image(player) => player.set_volume(browser, percent),
            Player::Web(player) => player.set_volume(browser, percent),
        }
    }

    fn get_volume_mode(&self) -> VolumeMode {
        match self {
            Player::YouTube(player) => player.get_volume_mode(),
            Player::Dash(player) => player.get_volume_mode(),
            Player::Hls(player) => player.get_volume_mode(),
            Player::Media(player) => player.get_volume_mode(),
            Player::Image(player) => player.get_volume_mode(),
            Player::Web(player) => player.get_volume_mode(),
        }
    }

    fn set_volume_mode(
        &mut self,
        browser: Option<&RustRefBrowser>,
        mode: VolumeMode,
    ) -> Result<()> {
        match self {
            Player::YouTube(player) => player.set_volume_mode(browser, mode),
            Player::Dash(player) => player.set_volume_mode(browser, mode),
            Player::Hls(player) => player.set_volume_mode(browser, mode),
            Player::Media(player) => player.set_volume_mode(browser, mode),
            Player::Image(player) => player.set_volume_mode(browser, mode),
            Player::Web(player) => player.set_volume_mode(browser, mode),
        }
    }

    fn get_autoplay(&self) -> bool {
        match self {
            Player::YouTube(player) => player.get_autoplay(),
            Player::Dash(player) => player.get_autoplay(),
            Player::Hls(player) => player.get_autoplay(),
            Player::Media(player) => player.get_autoplay(),
            Player::Image(player) => player.get_autoplay(),
            Player::Web(player) => player.get_autoplay(),
        }
    }

    fn set_autoplay(&mut self, browser: Option<&RustRefBrowser>, autoplay: bool) -> Result<()> {
        match self {
            Player::YouTube(player) => player.set_autoplay(browser, autoplay),
            Player::Dash(player) => player.set_autoplay(browser, autoplay),
            Player::Hls(player) => player.set_autoplay(browser, autoplay),
            Player::Media(player) => player.set_autoplay(browser, autoplay),
            Player::Image(player) => player.set_autoplay(browser, autoplay),
            Player::Web(player) => player.set_autoplay(browser, autoplay),
        }
    }

    fn get_loop(&self) -> bool {
        match self {
            Player::YouTube(player) => player.get_loop(),
            Player::Dash(player) => player.get_loop(),
            Player::Hls(player) => player.get_loop(),
            Player::Media(player) => player.get_loop(),
            Player::Image(player) => player.get_loop(),
            Player::Web(player) => player.get_loop(),
        }
    }

    fn set_loop(&mut self, browser: Option<&RustRefBrowser>, should_loop: bool) -> Result<()> {
        match self {
            Player::YouTube(player) => player.set_loop(browser, should_loop),
            Player::Dash(player) => player.set_loop(browser, should_loop),
            Player::Hls(player) => player.set_loop(browser, should_loop),
            Player::Media(player) => player.set_loop(browser, should_loop),
            Player::Image(player) => player.set_loop(browser, should_loop),
            Player::Web(player) => player.set_loop(browser, should_loop),
        }
    }

    fn get_url(&self) -> String {
        match self {
            Player::YouTube(player) => player.get_url(),
            Player::Dash(player) => player.get_url(),
            Player::Hls(player) => player.get_url(),
            Player::Media(player) => player.get_url(),
            Player::Image(player) => player.get_url(),
            Player::Web(player) => player.get_url(),
        }
    }

    fn get_title(&self) -> String {
        match self {
            Player::YouTube(player) => player.get_title(),
            Player::Dash(player) => player.get_title(),
            Player::Hls(player) => player.get_title(),
            Player::Media(player) => player.get_title(),
            Player::Image(player) => player.get_title(),
            Player::Web(player) => player.get_title(),
        }
    }

    fn is_finished_playing(&self) -> bool {
        match self {
            Player::YouTube(player) => player.is_finished_playing(),
            Player::Dash(player) => player.is_finished_playing(),
            Player::Hls(player) => player.is_finished_playing(),
            Player::Media(player) => player.is_finished_playing(),
            Player::Image(player) => player.is_finished_playing(),
            Player::Web(player) => player.is_finished_playing(),
        }
    }

    fn set_playing(&mut self, browser: &RustRefBrowser, playing: bool) -> Result<()> {
        match self {
            Player::YouTube(player) => player.set_playing(browser, playing),
            Player::Dash(player) => player.set_playing(browser, playing),
            Player::Hls(player) => player.set_playing(browser, playing),
            Player::Media(player) => player.set_playing(browser, playing),
            Player::Image(player) => player.set_playing(browser, playing),
            Player::Web(player) => player.set_playing(browser, playing),
        }
    }

    fn set_silent(&mut self, silent: bool) -> Result<()> {
        match self {
            Player::YouTube(player) => player.set_silent(silent),
            Player::Dash(player) => player.set_silent(silent),
            Player::Hls(player) => player.set_silent(silent),
            Player::Media(player) => player.set_silent(silent),
            Player::Image(player) => player.set_silent(silent),
            Player::Web(player) => player.set_silent(silent),
        }
    }

    fn set_speed(&mut self, browser: Option<&RustRefBrowser>, speed: f32) -> Result<()> {
        match self {
            Player::YouTube(player) => player.set_speed(browser, speed),
            Player::Dash(player) => player.set_speed(browser, speed),
            Player::Hls(player) => player.set_speed(browser, speed),
            Player::Media(player) => player.set_speed(browser, speed),
            Player::Image(player) => player.set_speed(browser, speed),
            Player::Web(player) => player.set_speed(browser, speed),
        }
    }
}

pub fn on_new_map() {
    volume_fade::on_new_map();
}

pub fn on_new_map_loaded() {
    volume_fade::on_new_map_loaded();
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
        if let Player::YouTube(_) = player {
        } else {
            panic!("not YouTube");
        }
    }
}
