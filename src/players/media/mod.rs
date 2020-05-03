use super::{helpers::start_update_loop, PlayerTrait, WebPlayer};
use crate::{
    async_manager::AsyncManager,
    cef::{RustRefBrowser, RustV8Value},
    chat::Chat,
    error::*,
};
use classicube_helpers::color;
use futures::{future::RemoteHandle, prelude::*};
use log::debug;
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};
use url::Url;

const PAGE_HTML: &str = include_str!("page.html");

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaPlayer {
    pub url: String,
    pub time: Duration,

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

    fn on_create(&mut self, _entity_id: usize) -> String {
        debug!("MediaPlayer on_create {}", self.url);

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

    fn on_page_loaded(&mut self, entity_id: usize, _browser: &mut RustRefBrowser) {
        let (f, remote_handle) = start_update_loop(entity_id).remote_handle();
        self.volume_loop_handle = Some(remote_handle);

        AsyncManager::spawn_local_on_main_thread(f);
    }

    fn on_title_change(&mut self, _entity_id: usize, _browser: &mut RustRefBrowser, title: String) {
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

    fn get_current_time(&self, _browser: &RustRefBrowser) -> Result<Duration> {
        Ok(self.time)
    }

    fn set_current_time(&mut self, browser: &mut RustRefBrowser, time: Duration) -> Result<()> {
        Self::get_player_field(browser, &format!("currentTime = {}", time.as_secs_f32()));
        Self::get_player_field(browser, "play()");
        self.time = time;

        Ok(())
    }

    fn get_volume(&self, _browser: &RustRefBrowser) -> Result<f32> {
        Ok(self.volume)
    }

    /// volume is a float between 0-1
    fn set_volume(&mut self, browser: &RustRefBrowser, percent: f32) -> Result<()> {
        Self::get_player_field(browser, &format!("volume = {}", percent));
        self.volume = percent;

        Ok(())
    }
}

impl MediaPlayer {
    pub async fn get_real_time(browser: &RustRefBrowser) -> Result<Duration> {
        let seconds = match Self::eval_player_field(browser, "currentTime").await? {
            RustV8Value::Double(seconds) => seconds as f32,
            RustV8Value::Int(seconds) => seconds as f32,
            RustV8Value::UInt(seconds) => seconds as f32,
            _ => {
                bail!("non-number js value");
            }
        };

        Ok(Duration::from_secs_f32(seconds))
    }

    #[allow(dead_code)]
    pub async fn get_real_volume(browser: &RustRefBrowser) -> Result<f32> {
        let percent = match Self::eval_player_field(browser, "volume").await? {
            RustV8Value::Double(percent) => percent as f32,
            RustV8Value::Int(percent) => percent as f32,
            RustV8Value::UInt(percent) => percent as f32,

            _ => {
                bail!("non-number js value");
            }
        };

        Ok(percent)
    }

    fn get_player_field(browser: &RustRefBrowser, field: &str) {
        let code = format!(
            r#"if (typeof window.player !== "undefined") {{
                window.player.{};
            }}"#,
            field
        );
        browser.execute_javascript(code).unwrap();
    }

    async fn eval_player_field(browser: &RustRefBrowser, field: &str) -> Result<RustV8Value> {
        let code = format!(
            r#"
                (() => {{
                    if (typeof window.player !== "undefined") {{
                        return window.player.{};
                    }}
                }})()
            "#,
            field
        );
        Ok(browser.eval_javascript(code).await?)
    }
}

impl MediaPlayer {
    pub fn from_url(url: Url) -> Option<Self> {
        let parts = url.path_segments()?;
        let last_part = parts.last()?;

        let path = Path::new(last_part);
        let ext = path.extension()?.to_str()?;

        match ext {
            "mp3" | "wav" | "ogg" | "aac" | "mp4" | "webm" | "avi" | "3gp" | "mov" => Some(Self {
                url: url.to_string(),
                ..Default::default()
            }),

            _ => None,
        }
    }
}
