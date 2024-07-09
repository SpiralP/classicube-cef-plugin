use std::time::{Duration, Instant};

use classicube_helpers::color::{SILVER, TEAL};
use futures::{future::RemoteHandle, prelude::*};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use url::Url;

use super::{
    helpers::{get_ext, start_update_loop},
    PlayerTrait, VolumeMode, WebPlayer,
};
use crate::{
    cef::{RustRefBrowser, RustV8Value},
    chat::Chat,
    error::{bail, Result},
    options,
};
use classicube_helpers::async_manager;

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaPlayer {
    pub url: String,
    pub time: Duration,

    // 0-1
    volume: f32,
    volume_mode: VolumeMode,

    autoplay: bool,
    should_loop: bool,
    silent: bool,
    speed: f32,

    #[serde(skip)]
    pub update_loop_handle: Option<RemoteHandle<()>>,

    #[serde(skip)]
    last_title: String,

    #[serde(skip)]
    pub finished: bool,

    #[serde(skip)]
    pub create_time: Option<Instant>,
}

impl Default for MediaPlayer {
    fn default() -> Self {
        Self {
            url: String::new(),
            time: Duration::from_millis(0),
            volume: 1.0,
            volume_mode: VolumeMode::Distance {
                multiplier: 1.0,
                distance: 28.0,
            },
            autoplay: true,
            should_loop: false,
            silent: false,
            speed: 1.0,
            update_loop_handle: None,
            last_title: String::new(),
            finished: false,
            create_time: None,
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
            should_loop: self.should_loop,
            silent: self.silent,
            speed: self.speed,
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

        match get_ext(&url)? {
            "3gp" | // don't format me
            "aac" |
            "avi" |
            "flac" |
            "m4a" |
            "m4v" |
            "mkv" |
            "mov" |
            "mp3" |
            "mp4" |
            "mpeg" |
            "mpeg4" |
            "mpg" |
            "mpg4" |
            "oga" |
            "ogg" |
            "ogm" |
            "ogv" |
            "ogx" |
            "opus" |
            "wav" |
            "weba" |
            "webm" |

            "media"
              => Ok(Self {
                url: url.to_string(),
                ..Default::default()
            }),

            _ => Err("url didn't end with a audio/video file extension".into()),
        }
    }

    fn on_create(&mut self) -> Result<String> {
        let url = self.url.to_string();
        Self::from_input(&url)?;
        debug!("MediaPlayer on_create {}", url);
        self.create_time = Some(Instant::now());

        let mut params = vec![
            ("url", self.url.to_string()),
            ("time", format!("{}", self.time.as_secs())),
            ("volume", format!("{}", self.volume)),
            ("speed", format!("{}", self.speed)),
        ];

        if self.autoplay {
            params.push(("autoplay", "1".to_string()));
        }

        if self.should_loop {
            params.push(("loop", "1".to_string()));
        }

        Ok(Url::parse_with_params("local://media/", &params)
            .unwrap()
            .into())
    }

    fn on_page_loaded(&mut self, entity_id: usize, _browser: &RustRefBrowser) {
        let (f, remote_handle) = start_update_loop(entity_id).remote_handle();
        self.update_loop_handle = Some(remote_handle);
        async_manager::spawn_local_on_main_thread(f);
    }

    fn on_title_change(&mut self, _entity_id: usize, browser: &RustRefBrowser, title: String) {
        if self.last_title == title || title == "Media Loading" {
            return;
        }

        if !self.silent {
            Chat::print(format!("{TEAL}Now playing {SILVER}{title}"));
        }

        self.last_title = title;

        if self.autoplay {
            let now = Instant::now();
            if let Some(create_time) = self.create_time {
                // if it took a long time to load
                let lag = now - create_time;
                debug!("media started playing after loading {:?}", lag);
                // TODO delay everyone a couple seconds then start playing video!
                if lag > Duration::from_secs(10) {
                    // TODO don't do this if longer than video duration
                    warn!("slow media load, seeking to {:?}", lag);
                    // seek to current time
                    let current_time = self.time + lag;
                    self.set_current_time(browser, current_time).unwrap();
                }
            }
        }
    }

    fn get_current_time(&self) -> Result<Duration> {
        Ok(self.time)
    }

    fn set_current_time(&mut self, browser: &RustRefBrowser, time: Duration) -> Result<()> {
        Self::execute(browser, &format!("setCurrentTime({})", time.as_secs_f32()))?;
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
                let volume_modifier = options::VOLUME.get()?;
                Self::execute(browser, &format!("setVolume({})", volume * volume_modifier))?;
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
        browser: Option<&RustRefBrowser>,
        mode: VolumeMode,
    ) -> Result<()> {
        if let Some(browser) = browser {
            if let VolumeMode::Panning { pan, .. } = mode {
                Self::execute(browser, &format!("handlePanning({pan})"))?;
            } else {
                browser.execute_javascript(
                    r#"
                        if (typeof window.panner !== "undefined") {
                            window.panner.pan.value = 0.0;
                        }
                    "#,
                )?;
            }
        }

        self.volume_mode = mode;
        Ok(())
    }

    fn get_autoplay(&self) -> bool {
        self.autoplay
    }

    fn set_autoplay(&mut self, _browser: Option<&RustRefBrowser>, autoplay: bool) -> Result<()> {
        self.autoplay = autoplay;
        Ok(())
    }

    fn get_loop(&self) -> bool {
        self.should_loop
    }

    fn set_loop(&mut self, _browser: Option<&RustRefBrowser>, should_loop: bool) -> Result<()> {
        self.should_loop = should_loop;
        Ok(())
    }

    fn get_url(&self) -> String {
        self.url.clone()
    }

    fn get_title(&self) -> String {
        self.last_title.clone()
    }

    fn is_finished_playing(&self) -> bool {
        self.finished
    }

    fn set_playing(&mut self, browser: &RustRefBrowser, playing: bool) -> Result<()> {
        Self::execute(browser, &format!("setPlaying({playing})"))?;
        Ok(())
    }

    fn set_silent(&mut self, silent: bool) -> Result<()> {
        self.silent = silent;
        Ok(())
    }

    fn set_speed(&mut self, browser: Option<&RustRefBrowser>, speed: f32) -> Result<()> {
        if let Some(browser) = browser {
            Self::execute(browser, &format!("setPlaybackRate({speed})"))?;
        }

        self.speed = speed;
        Ok(())
    }
}

impl MediaPlayer {
    pub async fn real_is_finished_playing(browser: &RustRefBrowser) -> Result<bool> {
        let ended = match Self::eval(browser, "playerFinished").await? {
            RustV8Value::Bool(ended) => ended,

            other => {
                bail!("non-bool js value {:?}", other);
            }
        };

        Ok(ended)
    }

    pub async fn get_real_time(browser: &RustRefBrowser) -> Result<Duration> {
        let seconds = match Self::eval(browser, "getCurrentTime()").await? {
            RustV8Value::Double(seconds) => seconds as f32,
            RustV8Value::Int(seconds) => seconds as f32,
            RustV8Value::UInt(seconds) => seconds as f32,
            _ => {
                bail!("non-number js value");
            }
        };

        Ok(Duration::from_secs_f32(seconds))
    }

    fn execute(browser: &RustRefBrowser, method: &str) -> Result<()> {
        let code = format!("window.{method};");
        browser.execute_javascript(code)?;
        Ok(())
    }

    async fn eval(browser: &RustRefBrowser, method: &str) -> Result<RustV8Value> {
        let code = format!("window.{method};");
        browser.eval_javascript(code).await
    }
}
