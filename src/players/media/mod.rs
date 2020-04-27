use super::{Player, PlayerTrait, WebPlayer};
use crate::{
    async_manager::AsyncManager,
    cef::RustRefBrowser,
    chat::{Chat, ENTITIES},
    entity_manager::EntityManager,
    error::*,
};
use classicube_helpers::{color, OptionWithInner};
use classicube_sys::ENTITIES_SELF_ID;
use futures::{future::RemoteHandle, prelude::*};
use log::debug;
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    time::{Duration, Instant},
};
use url::Url;

const PAGE_HTML: &str = include_str!("page.html");

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaPlayer {
    pub url: String,
    pub time: Duration,

    #[serde(skip)]
    pub start_time: Option<Instant>,

    // 0-1
    pub volume: f32,

    #[serde(skip)]
    volume_loop_handle: Option<RemoteHandle<()>>,

    #[serde(skip)]
    last_title: String,
}

impl Default for MediaPlayer {
    fn default() -> Self {
        Self {
            url: String::new(),
            time: Duration::from_millis(0),
            volume: 1.0,
            start_time: None,
            volume_loop_handle: None,
            last_title: String::new(),
        }
    }
}

impl Clone for MediaPlayer {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            time: self.time,
            volume: self.volume,
            // we need start_time because we use clone in encoding.rs
            start_time: self.start_time,
            ..Default::default()
        }
    }
}

impl PlayerTrait for MediaPlayer {
    fn from_input(url: &str) -> Result<Self> {
        // make sure it's a normal url
        WebPlayer::from_input(url)?;

        let url = Url::parse(url)?;
        if let Some(this) = Self::from_url(url) {
            Ok(this)
        } else {
            Err("not a media url".into())
        }
    }

    fn on_create(&mut self, entity_id: usize) -> String {
        debug!("MediaPlayer on_create {}", self.url);

        let (f, remote_handle) = start_volume_loop(entity_id).remote_handle();
        self.volume_loop_handle = Some(remote_handle);

        AsyncManager::spawn_local_on_main_thread(f);

        format!(
            "data:text/html;base64,{}",
            base64::encode(
                PAGE_HTML
                    .replace("MEDIA_URL", &self.url)
                    .replace("START_TIME", &format!("{}", self.time.as_secs()))
                    .replace("START_VOLUME", &format!("{}", self.volume))
            )
        )
    }

    fn on_page_loaded(&mut self, _browser: &mut RustRefBrowser) {
        self.start_time = Some(Instant::now());
    }

    fn on_title_change(&mut self, _browser: &mut RustRefBrowser, title: String) {
        if self.last_title == title {
            return;
        }
        self.last_title = title.clone();

        if title == "Media Loading" {
            return;
        }

        Chat::print(format!(
            "{}Now playing {}{}",
            color::TEAL,
            color::SILVER,
            title,
        ));
    }

    fn set_current_time(&mut self, browser: &mut RustRefBrowser, time: Duration) -> Result<()> {
        Self::seek_to(browser, time.as_secs_f32());

        Ok(())
    }

    fn get_current_time(&self) -> Result<Duration> {
        let start_time = self.start_time.ok_or("no start time")?;
        Ok(Instant::now() - start_time)
    }
}

async fn start_volume_loop(entity_id: usize) {
    loop {
        AsyncManager::sleep(Duration::from_millis(32)).await;

        let maybe_entity_pos = EntityManager::with_by_entity_id(entity_id, |entity| {
            if let Player::Media(media) = &entity.player {
                if media.start_time.is_some() {
                    // if we're loaded

                    Ok(entity
                        .browser
                        .as_ref()
                        .map(|a| (entity.entity.Position, a.clone())))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        })
        .ok()
        .flatten();

        if let Some((entity_pos, browser)) = maybe_entity_pos {
            let maybe_my_pos = ENTITIES
                .with_inner(|entities| {
                    let me = entities.get(ENTITIES_SELF_ID as _)?;

                    Some(me.get_position())
                })
                .flatten();

            if let Some(my_pos) = maybe_my_pos {
                let percent = (entity_pos - my_pos).length_squared().sqrt() / 30f32;
                let percent = (1.0 - percent).max(0.0).min(1.0);

                MediaPlayer::set_volume(&browser, percent);
            }
        }
    }
}

impl MediaPlayer {
    fn execute_player_method(browser: &RustRefBrowser, method_with_args: &str) {
        let code = format!(
            r#"if (typeof window.player !== "undefined") {{
                window.player.{};
            }}"#,
            method_with_args
        );
        browser.execute_javascript(code).unwrap();
    }

    /// volume is a float between 0-1
    fn set_volume(browser: &RustRefBrowser, percent: f32) {
        Self::execute_player_method(browser, &format!("volume = {}", percent))
    }

    fn seek_to(browser: &RustRefBrowser, seconds: f32) {
        Self::execute_player_method(browser, &format!("currentTime = {}", seconds))
    }
}

impl MediaPlayer {
    pub fn from_url(url: Url) -> Option<Self> {
        let parts = url.path_segments()?;
        let last_part = parts.last()?;

        let path = Path::new(last_part);
        let ext = path.extension()?.to_str()?;

        match ext {
            "mp3" | "wav" | "ogg" | "aac" | "mp4" | "webm" | "avi" | "3gp" => Some(Self {
                url: url.to_string(),
                ..Default::default()
            }),

            _ => None,
        }
    }
}
