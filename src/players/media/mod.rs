use super::{helpers::start_update_loop, PlayerTrait, VolumeMode, WebPlayer};
use crate::{
    async_manager,
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
    volume: f32,
    volume_mode: VolumeMode,

    autoplay: bool,

    #[serde(skip)]
    pub update_loop_handle: Option<RemoteHandle<()>>,

    #[serde(skip)]
    last_title: String,
}

impl Default for MediaPlayer {
    fn default() -> Self {
        Self {
            url: String::new(),
            time: Duration::from_millis(0),
            volume: 1.0,
            volume_mode: VolumeMode::Distance { distance: 28.0 },
            autoplay: true,
            update_loop_handle: None,
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
            volume_mode: self.volume_mode,
            autoplay: self.autoplay,
            ..Default::default()
        }
    }
}

impl PlayerTrait for MediaPlayer {
    fn type_name(&self) -> &'static str {
        "Media"
    }

    fn from_input(url: &str) -> Result<Self> {
        // make sure it's a normal url
        WebPlayer::from_input(url)?;

        let url = Url::parse(url)?;
        Ok(Self::from_url(&url)?)
    }

    fn on_create(&mut self) -> String {
        debug!("MediaPlayer on_create {}", self.url);

        format!(
            "data:text/html;base64,{}",
            base64::encode(
                PAGE_HTML
                    .replace("MEDIA_URL", &self.url)
                    .replace("START_TIME", &format!("{}", self.time.as_secs()))
                    .replace("START_VOLUME", &format!("{}", self.volume))
                    .replace("AUTOPLAY", &format!("{}", self.autoplay))
            )
        )
    }

    fn on_page_loaded(&mut self, entity_id: usize, _browser: &RustRefBrowser) {
        let (f, remote_handle) = start_update_loop(entity_id).remote_handle();
        self.update_loop_handle = Some(remote_handle);

        async_manager::spawn_local_on_main_thread(f);
    }

    fn on_title_change(
        &mut self,
        _entity_id: usize,
        _browser: &RustRefBrowser,
        title: String,
        silent: bool,
    ) {
        if self.last_title == title || title == "Media Loading" {
            return;
        }

        if !silent {
            Chat::print(format!(
                "{}Now playing {}{}",
                color::TEAL,
                color::SILVER,
                title,
            ));
        }

        self.last_title = title;
    }

    fn get_current_time(&self) -> Result<Duration> {
        Ok(self.time)
    }

    fn set_current_time(&mut self, browser: &RustRefBrowser, time: Duration) -> Result<()> {
        Self::execute_with_player_field(browser, &format!("currentTime = {}", time.as_secs_f32()));
        self.time = time;

        Ok(())
    }

    fn get_volume(&self) -> f32 {
        self.volume
    }

    /// volume is a float between 0-1
    fn set_volume(&mut self, browser: Option<&RustRefBrowser>, volume: f32) -> Result<()> {
        if let Some(browser) = browser {
            if (volume - self.volume).abs() > 0.0001 {
                Self::execute_with_player_field(browser, &format!("volume = {}", volume));
            }
        }

        self.volume = volume;

        Ok(())
    }

    fn get_volume_mode(&self) -> VolumeMode {
        self.volume_mode
    }

    fn set_volume_mode(
        &mut self,
        _browser: Option<&RustRefBrowser>,
        mode: VolumeMode,
    ) -> Result<()> {
        if let VolumeMode::Panning { .. } = mode {
            bail!("panning is TODO");
        }

        self.volume_mode = mode;
        Ok(())
    }

    fn get_autoplay(&self) -> bool {
        self.autoplay
    }

    fn set_autoplay(&mut self, autoplay: bool) {
        self.autoplay = autoplay;
    }

    fn get_url(&self) -> String {
        self.url.clone()
    }

    fn get_title(&self) -> String {
        self.last_title.clone()
    }

    fn is_finished_playing(&self) -> bool {
        // TODO
        false
    }

    fn set_playing(&mut self, browser: &RustRefBrowser, playing: bool) -> Result<()> {
        if playing {
            Self::execute_with_player_field(browser, "play()");
        } else {
            Self::execute_with_player_field(browser, "pause()");
        }
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

    fn execute_with_player_field(browser: &RustRefBrowser, field: &str) {
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
    pub fn from_url(url: &Url) -> Result<Self> {
        if url.scheme() != "http" && url.scheme() != "https" {
            Err("not http/https".into())
        } else {
            let parts = url.path_segments().chain_err(|| "no path segments")?;
            let last_part = parts.last().chain_err(|| "no last_part")?;

            let path = Path::new(last_part);
            let ext = path
                .extension()
                .chain_err(|| "no extension")?
                .to_str()
                .chain_err(|| "to_str")?;

            match ext {
                "mp3" | "wav" | "ogg" | "aac" | "mp4" | "webm" | "avi" | "3gp" | "mov" => {
                    Ok(Self {
                        url: url.to_string(),
                        ..Default::default()
                    })
                }

                _ => Err("url didn't end with a audio/video file extension".into()),
            }
        }
    }
}
