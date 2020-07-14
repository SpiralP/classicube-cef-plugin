use super::{helpers::start_update_loop, PlayerTrait};
use crate::{
    async_manager,
    cef::{RustRefBrowser, RustV8Value},
    chat::Chat,
    error::*,
    options::SUBTITLES,
};
use classicube_helpers::color;
use futures::{future::RemoteHandle, prelude::*};
use log::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use url::Url;

const PAGE_HTML: &str = include_str!("page.html");

#[derive(Debug, Serialize, Deserialize)]
pub struct YoutubePlayer {
    pub id: String,
    pub time: Duration,

    // 0-1
    pub volume: f32,

    pub global_volume: bool,

    #[serde(skip)]
    should_send: bool,

    autoplay: bool,

    #[serde(skip)]
    pub update_loop_handle: Option<RemoteHandle<()>>,

    #[serde(skip)]
    last_title: String,

    #[serde(skip)]
    pub finished: bool,

    #[serde(skip)]
    pub create_time: Option<Instant>,
}

impl Default for YoutubePlayer {
    fn default() -> Self {
        Self {
            id: String::new(),
            time: Duration::from_millis(0),
            volume: 1.0,
            global_volume: false,
            should_send: true,
            autoplay: true,
            update_loop_handle: None,
            last_title: String::new(),
            finished: false,
            create_time: None,
        }
    }
}

impl Clone for YoutubePlayer {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            time: self.time,
            volume: self.volume,
            global_volume: self.global_volume,
            autoplay: self.autoplay,
            ..Default::default()
        }
    }
}

impl PlayerTrait for YoutubePlayer {
    fn type_name(&self) -> &'static str {
        "Youtube"
    }

    fn from_input(url_or_id: &str) -> Result<Self> {
        let url_or_id = url_or_id.replace("%feature=", "&feature=");
        if let Ok(url) = Url::parse(&url_or_id) {
            Self::from_url(&url)
        } else if let Some(this) = Self::from_id(url_or_id.to_string()) {
            Ok(this)
        } else {
            Err("couldn't match id or url from input".into())
        }
    }

    fn on_create(&mut self) -> String {
        debug!("YoutubePlayer on_create {}", self.id);
        self.create_time = Some(Instant::now());

        format!(
            "data:text/html;base64,{}",
            base64::encode(
                PAGE_HTML
                    .replace("VIDEO_ID", &self.id)
                    .replace("START_TIME", &format!("{}", self.time.as_secs()))
                    .replace(
                        "START_VOLUME",
                        &format!("{}", (self.volume * 100f32) as u32)
                    )
                    .replace("SUBTITLES", &format!("{}", SUBTITLES.get().unwrap()))
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
        browser: &RustRefBrowser,
        title: String,
        silent: bool,
    ) {
        if self.last_title == title || title == "YouTube Loading" {
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

        if self.autoplay {
            let now = Instant::now();
            if let Some(create_time) = self.create_time {
                // if it took a long time to load
                let lag = now - create_time;
                debug!("video started playing after loading {:?}", lag);
                // TODO delay everyone a couple seconds then start playing video!
                if lag > Duration::from_secs(10) {
                    warn!("slow video load, seeking to {:?}", lag);
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
        Self::execute_function(browser, &format!("setCurrentTime({})", time.as_secs_f32()));
        self.time = time;

        Ok(())
    }

    fn get_volume(&self) -> Result<f32> {
        Ok(self.volume)
    }

    /// volume is a float between 0-1
    fn set_volume(&mut self, browser: &RustRefBrowser, volume: f32) -> Result<()> {
        if (volume - self.volume).abs() > 0.0001 {
            Self::execute_function(browser, &format!("setVolume({})", (volume * 100f32) as u32));
        }

        self.volume = volume;

        Ok(())
    }

    fn has_global_volume(&self) -> bool {
        self.global_volume
    }

    fn set_global_volume(&mut self, global_volume: bool) -> Result<()> {
        self.global_volume = global_volume;

        Ok(())
    }

    fn get_should_send(&self) -> bool {
        self.should_send
    }

    fn set_should_send(&mut self, should_send: bool) {
        self.should_send = should_send;
    }

    fn get_autoplay(&self) -> bool {
        self.autoplay
    }

    fn set_autoplay(&mut self, autoplay: bool) {
        self.autoplay = autoplay;
    }

    fn get_url(&self) -> String {
        let secs = self.time.as_secs();
        if secs == 0 {
            format!("https://youtu.be/{}", self.id)
        } else {
            format!("https://youtu.be/{}?t={}", self.id, secs)
        }
    }

    fn get_title(&self) -> String {
        self.last_title.clone()
    }

    fn is_finished_playing(&self) -> bool {
        self.finished
    }

    fn set_playing(&mut self, browser: &RustRefBrowser, playing: bool) -> Result<()> {
        Self::execute_function(browser, &format!("setPlaying({})", playing));

        Ok(())
    }
}

impl YoutubePlayer {
    pub async fn real_is_finished_playing(browser: &RustRefBrowser) -> Result<bool> {
        let ended = match Self::eval_method(browser, "playerEnded").await? {
            RustV8Value::Bool(ended) => ended,

            other => {
                bail!("non-bool js value {:?}", other);
            }
        };

        Ok(ended)
    }

    pub async fn get_real_time(browser: &RustRefBrowser) -> Result<Duration> {
        let seconds = match Self::eval_method(browser, "getCurrentTime()").await? {
            RustV8Value::Double(seconds) => seconds as f32,
            RustV8Value::Int(seconds) => seconds as f32,
            RustV8Value::UInt(seconds) => seconds as f32,

            other => {
                bail!("non-number js value {:?}", other);
            }
        };

        Ok(Duration::from_secs_f32(seconds))
    }

    #[allow(dead_code)]
    pub async fn get_real_volume(browser: &RustRefBrowser) -> Result<f32> {
        let volume = match Self::eval_method(browser, "getVolume()").await? {
            RustV8Value::Double(volume) => volume as f32,
            RustV8Value::Int(volume) => volume as f32,
            RustV8Value::UInt(volume) => volume as f32,

            _ => {
                bail!("non-number js value");
            }
        };

        let percent = volume as f32 / 100.0;
        Ok(percent)
    }

    fn execute_function(browser: &RustRefBrowser, method: &str) {
        let code = format!("window.{};", method);
        browser.execute_javascript(code).unwrap();
    }

    async fn eval_method(browser: &RustRefBrowser, method: &str) -> Result<RustV8Value> {
        let code = format!("window.{};", method);
        Ok(browser.eval_javascript(code).await?)
    }
}

impl YoutubePlayer {
    pub fn from_id(id: String) -> Option<Self> {
        let regex = Regex::new(r"^[A-Za-z0-9_\-]{11}$").unwrap();
        if regex.is_match(&id) {
            Some(Self {
                id,
                time: Duration::from_secs(0),
                ..Default::default()
            })
        } else if id.contains('%') {
            let id = id.split('%').next()?;
            Self::from_id(id.to_string())
        } else {
            None
        }
    }

    pub fn from_id_and_time(id: String, time: Duration) -> Option<Self> {
        let mut this = Self::from_id(id)?;
        this.time = time;

        Some(this)
    }

    pub fn from_url(url: &Url) -> Result<Self> {
        if url.scheme() != "http" && url.scheme() != "https" {
            Err("not http/https".into())
        } else if let Some(this) = Self::from_normal(&url) {
            Ok(this)
        } else if let Some(this) = Self::from_short(&url) {
            Ok(this)
        } else if let Some(this) = Self::from_embed(&url) {
            Ok(this)
        } else {
            Err("couldn't match url from input".into())
        }
    }

    fn from_normal(url: &Url) -> Option<Self> {
        let host_str = url.host_str()?;
        if host_str != "youtube.com" && host_str != "www.youtube.com" {
            return None;
        }

        let query: HashMap<_, _> = url.query_pairs().collect();
        let id = query.get("v")?.to_string();

        // checks "t" first then for "time_continue"
        let time = query
            .get("t")
            .and_then(|s| s.parse().ok())
            .map(Duration::from_secs)
            .or_else(|| {
                query
                    .get("time_continue")
                    .and_then(|s| s.parse().ok())
                    .map(Duration::from_secs)
            })
            .unwrap_or_default();

        Some(Self::from_id_and_time(id, time)?)
    }

    fn from_short(url: &Url) -> Option<Self> {
        let host_str = url.host_str()?;
        if host_str != "youtu.be" {
            return None;
        }

        let id = url.path_segments()?.next()?.to_string();

        let query: HashMap<_, _> = url.query_pairs().collect();
        let time = query
            .get("t")
            .and_then(|s| s.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or_default();

        Some(Self::from_id_and_time(id, time)?)
    }

    fn from_embed(url: &Url) -> Option<Self> {
        let host_str = url.host_str()?;
        if host_str != "youtube.com" && host_str != "www.youtube.com" {
            return None;
        }

        let mut path_segments = url.path_segments()?;
        if path_segments.next()? != "embed" {
            return None;
        }

        let id = path_segments.next()?.to_string();

        let query: HashMap<_, _> = url.query_pairs().collect();
        let time = query
            .get("start")
            .and_then(|s| s.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or_default();

        Some(Self::from_id_and_time(id, time)?)
    }
}

#[test]
fn test_youtube() {
    {
        let without_time = [
            "https://www.youtube.com/watch?v=gQngg8iQipk",
            "https://youtu.be/gQngg8iQipk",
            "https://www.youtube.com/embed/gQngg8iQipk",
            // test for cc replacing & with %
            "https://www.youtube.com/watch?v=gQngg8iQipk&list=ELG1JYZnaQbZc",
            "https://www.youtube.com/watch?v=gQngg8iQipk%list=ELG1JYZnaQbZc",
            "https://www.youtube.com/watch?v=gQngg8iQipk&feature=youtu.be",
            "https://www.youtube.com/watch?v=gQngg8iQipk%feature=youtu.be",
        ];

        let should = YoutubePlayer {
            id: "gQngg8iQipk".into(),
            time: Duration::from_secs(0),
            ..Default::default()
        };
        for &url in &without_time {
            let yt = YoutubePlayer::from_input(url).unwrap();
            assert_eq!(yt.id, should.id);
            assert_eq!(yt.time, should.time);
        }
    }

    {
        let with_time = [
            "https://www.youtube.com/watch?v=gQngg8iQipk&feature=youtu.be&t=36",
            "https://www.youtube.com/watch?v=gQngg8iQipk&t=36",
            "https://www.youtube.com/watch?time_continue=36&v=gQngg8iQipk&feature=emb_logo",
            "https://www.youtube.com/watch?t=36&time_continue=11&v=gQngg8iQipk&feature=emb_logo",
            "https://youtu.be/gQngg8iQipk?t=36",
            "https://www.youtube.com/embed/gQngg8iQipk?autoplay=1&start=36",
            "https://www.youtube.com/embed/gQngg8iQipk?start=36",
            /* TODO
             * "https://www.youtube.com/watch?v=gQngg8iQipk%t=827s", */
        ];

        let should = YoutubePlayer {
            id: "gQngg8iQipk".into(),
            time: Duration::from_secs(36),
            ..Default::default()
        };
        for &url in &with_time {
            let yt = YoutubePlayer::from_input(url).unwrap();
            assert_eq!(yt.id, should.id);
            assert_eq!(yt.time, should.time);
        }
    }

    let left = YoutubePlayer::from_input("gQngg8iQipk").unwrap();
    let right = YoutubePlayer {
        id: "gQngg8iQipk".into(),
        time: Duration::from_secs(0),
        ..Default::default()
    };
    assert_eq!(left.id, right.id);
    assert_eq!(left.time, right.time);

    // not 11 chars
    assert!(YoutubePlayer::from_input("gQngg8iQip").is_err());

    // blank input
    assert!(YoutubePlayer::from_input("").is_err());
}
