use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use classicube_helpers::{
    async_manager,
    color::{SILVER, TEAL},
};
use futures::{future::RemoteHandle, prelude::*};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use url::Url;

use super::{helpers::start_update_loop, PlayerTrait, VolumeMode};
use crate::{
    cef::{RustRefBrowser, RustV8Value},
    chat::Chat,
    error::{bail, Result},
    options,
    options::SUBTITLES,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct YouTubePlayer {
    pub id: String,
    pub time: Duration,

    // TODO syncing playlists based on video index and time?
    pub is_playlist: bool,

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

impl Default for YouTubePlayer {
    fn default() -> Self {
        Self {
            id: String::new(),
            is_playlist: false,
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

impl Clone for YouTubePlayer {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            is_playlist: self.is_playlist,
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

impl PlayerTrait for YouTubePlayer {
    fn type_name(&self) -> &'static str {
        "YouTube"
    }

    fn from_input(url_or_id: &str) -> Result<Self> {
        if let Ok(url) = Url::parse(url_or_id) {
            Self::from_url(&url).or_else(|e| {
                // try with % replaced with &
                // because & is sent as % from cc client
                if let Ok(url) = Url::parse(&url_or_id.replace('%', "&")) {
                    Self::from_url(&url).or(Err(e))
                } else {
                    Err(e)
                }
            })
        } else if let Some(this) = Self::from_id(url_or_id) {
            Ok(this)
        } else {
            Err("couldn't match id or url from input".into())
        }
    }

    fn on_create(&mut self) -> Result<String> {
        debug!("YouTubePlayer on_create {}", self.id);
        self.create_time = Some(Instant::now());

        let mut params = vec![
            ("id", self.id.to_string()),
            ("time", format!("{}", self.time.as_secs())),
            ("volume", format!("{}", self.volume)),
            ("speed", format!("{}", self.speed)),
        ];

        if SUBTITLES.get().unwrap() {
            params.push(("subtitles", "1".to_string()));
        }

        if self.autoplay {
            params.push(("autoplay", "1".to_string()));
        }

        if self.should_loop {
            params.push(("loop", "1".to_string()));
        }

        if self.is_playlist {
            params.push(("playlist", "1".to_string()));
        }

        Ok(Url::parse_with_params("local://youtube/", &params)?.into())
    }

    fn on_page_loaded(&mut self, entity_id: usize, _browser: &RustRefBrowser) {
        let (f, remote_handle) = start_update_loop(entity_id).remote_handle();
        self.update_loop_handle = Some(remote_handle);
        async_manager::spawn_local_on_main_thread(f);
    }

    fn on_title_change(&mut self, _entity_id: usize, browser: &RustRefBrowser, title: String) {
        if self.last_title == title || title == "YouTube Loading" {
            return;
        }

        if !self.silent {
            Chat::print(format!("{TEAL}Now playing {SILVER}{title}"));
        }

        self.last_title = title;

        // playlists will show multiple titles
        if self.autoplay && !self.is_playlist {
            let now = Instant::now();
            if let Some(create_time) = self.create_time {
                // if it took a long time to load
                let lag = now - create_time;
                debug!("video started playing after loading {:?}", lag);
                // TODO delay everyone a couple seconds then start playing video!
                if lag > Duration::from_secs(10) {
                    // TODO don't do this if longer than video duration
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
                // TODO less big string!
                let _ignore = browser.execute_javascript_on_frame(
                    "https://www.youtube.com",
                    format!(
                        r#"
                            if (typeof window.panner === "undefined") {{
                                var video = document.getElementsByTagName("video")[0];
                                var context = new AudioContext();
                                var source = context.createMediaElementSource(video);
                                var panner = context.createStereoPanner();
                                source.connect(panner);
                                panner.connect(context.destination);
                                window.panner = panner;
                                window.context = context;
                            }}
                            window.panner.pan.setTargetAtTime(
                                {pan},
                                window.context.currentTime,
                                0.02
                            );
                        "#
                    ),
                );
            } else {
                let _ignore = browser.execute_javascript_on_frame(
                    "https://www.youtube.com",
                    r#"
                        if (typeof window.panner !== "undefined") {
                            window.panner.pan.value = 0.0;
                        }
                    "#,
                );
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
        let secs = self.time.as_secs();
        if secs == 0 {
            format!("https://youtu.be/{}", self.id)
        } else {
            format!("https://youtu.be/{}?t={secs}", self.id)
        }
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

impl YouTubePlayer {
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

            other => {
                bail!("non-number js value {:?}", other);
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

impl YouTubePlayer {
    pub fn from_video_id(id: &str) -> Option<Self> {
        if id.len() == 11 && Regex::new(r"^[A-Za-z0-9_\-]+$").unwrap().is_match(id) {
            Some(Self {
                id: id.to_string(),
                ..Default::default()
            })
        } else {
            None
        }
    }

    pub fn from_playlist_id(id: &str) -> Option<Self> {
        if id.len() > 11 && Regex::new(r"^[A-Za-z0-9_\-]+$").unwrap().is_match(id) {
            Some(Self {
                id: id.to_string(),
                is_playlist: true,
                ..Default::default()
            })
        } else {
            None
        }
    }

    /// from either a video id or playlist id
    pub fn from_id(id: &str) -> Option<Self> {
        if id.len() >= 11 {
            Self::from_video_id(id).or_else(|| Self::from_playlist_id(id))
        } else {
            None
        }
    }

    pub fn from_id_and_time(id: &str, time: Duration) -> Option<Self> {
        let mut this = Self::from_id(id)?;
        this.time = time;

        Some(this)
    }

    pub fn from_url(url: &Url) -> Result<Self> {
        if url.scheme() != "http" && url.scheme() != "https" {
            Err("not http/https".into())
        } else if let Some(this) = Self::from_embed(url) {
            Ok(this)
        } else if let Some(this) = Self::from_short(url) {
            Ok(this)
        } else if let Some(this) = Self::from_normal(url) {
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
        let id = query.get("v").or_else(|| query.get("list"))?;

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

        Self::from_id_and_time(id, time)
    }

    fn from_short(url: &Url) -> Option<Self> {
        let host_str = url.host_str()?;
        if host_str != "youtu.be" {
            return None;
        }

        let id = url.path_segments()?.next()?;

        let query: HashMap<_, _> = url.query_pairs().collect();
        let time = query
            .get("t")
            .and_then(|s| s.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or_default();

        Self::from_id_and_time(id, time)
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

        let id = path_segments.next()?;

        let query: HashMap<_, _> = url.query_pairs().collect();
        let time = query
            .get("start")
            .and_then(|s| s.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or_default();

        Self::from_id_and_time(id, time)
    }
}

#[test]
fn test_youtube() {
    {
        let inputs = [
            "pNMRBTN1SGU",
            "https://www.youtube.com/watch?v=pNMRBTN1SGU",
            "https://youtu.be/pNMRBTN1SGU",
            "https://www.youtube.com/embed/pNMRBTN1SGU",
            // test for cc replacing & with %
            "https://www.youtube.com/watch?v=pNMRBTN1SGU&list=ELG1JYZnaQbZc",
            "https://www.youtube.com/watch?v=pNMRBTN1SGU%list=ELG1JYZnaQbZc",
            "https://www.youtube.com/watch?v=pNMRBTN1SGU&feature=youtu.be",
            "https://www.youtube.com/watch?v=pNMRBTN1SGU%feature=youtu.be",
            "https://www.youtube.com/watch?v=pNMRBTN1SGU&ab_channel=VvporTV",
            "https://www.youtube.com/watch?v=pNMRBTN1SGU%ab_channel=VvporTV",
        ];

        let should = YouTubePlayer {
            id: "pNMRBTN1SGU".into(),
            ..Default::default()
        };
        for input in &inputs {
            let yt = YouTubePlayer::from_input(input).expect(input);
            assert_eq!(yt.id, should.id);
        }
    }

    {
        let inputs = [
            "https://www.youtube.com/watch?v=pNMRBTN1SGU&feature=youtu.be&t=36",
            "https://www.youtube.com/watch?v=pNMRBTN1SGU&t=36",
            "https://www.youtube.com/watch?time_continue=36&v=pNMRBTN1SGU&feature=emb_logo",
            "https://www.youtube.com/watch?t=36&time_continue=11&v=pNMRBTN1SGU&feature=emb_logo",
            "https://youtu.be/pNMRBTN1SGU?t=36",
            "https://www.youtube.com/embed/pNMRBTN1SGU?autoplay=1&start=36",
            "https://www.youtube.com/embed/pNMRBTN1SGU?start=36",
            "https://www.youtube.com/watch?v=pNMRBTN1SGU%t=36",
            "https://www.youtube.com/watch?time_continue=36%v=pNMRBTN1SGU%feature=emb_logo",
            "https://www.youtube.com/watch?v=pNMRBTN1SGU%feature=youtu.be%t=36",
            // test for cc replacing & with %
        ];

        let should = YouTubePlayer {
            id: "pNMRBTN1SGU".into(),
            time: Duration::from_secs(36),
            ..Default::default()
        };
        for input in &inputs {
            let yt = YouTubePlayer::from_input(input).expect(input);
            assert_eq!(yt.id, should.id);
            assert_eq!(yt.time, should.time);
        }
    }

    {
        // not 11 chars
        assert!(YouTubePlayer::from_input("gQngg8iQip").is_err());

        // blank input
        assert!(YouTubePlayer::from_input("").is_err());
    }

    // playlists
    {
        // id parsing
        {
            let ids = [
                "PLspeOI0YmcdPQJWbvMhOCg5RhGkNUoVJR",
                "PLDfU1tT3TQ16cW3WdAKf2WicS6wrdgZxB",
                "PLbzoR-pLrL6qucl8-lOnzvhFc2UM1tcZA",
                "PLWwAypAcFRgKAIIFqBr9oy-ZYZnixa_Fj",
                "OLAK5uy_kNWirTkTpIwjLNlYorGFj8-GIa5yPHw1c",
                "PLpnh5xqG8PuONcW35KipnicwP4W3oyFMS",
                "PLF37D334894B07EEA",
            ];
            for id in &ids {
                let should = YouTubePlayer {
                    id: (*id).to_string(),
                    is_playlist: true,
                    ..Default::default()
                };
                {
                    let yt = YouTubePlayer::from_input(id).expect(id);
                    assert_eq!(yt.id, should.id);
                }
                {
                    // also try link to playlist
                    let id = format!("https://www.youtube.com/playlist?list={id}");
                    let yt = YouTubePlayer::from_input(&id).expect(&id);
                    assert_eq!(yt.id, should.id);
                }
            }
        }

        // link to video with playlist param
        // -> don't treat as playlist at all, just the video
        // if we ever want to handle both make sure to check "index" param
        {
            let ids = [
                "https://youtu.be/mZpa3nOLOa8?list=PLDfU1tT3TQ16cW3WdAKf2WicS6wrdgZxB",
                "https://www.youtube.com/watch?v=mZpa3nOLOa8&list=PLDfU1tT3TQ16cW3WdAKf2WicS6wrdgZxB&index=1",
            ];
            let should = YouTubePlayer {
                id: "mZpa3nOLOa8".to_string(),
                is_playlist: false,
                ..Default::default()
            };
            for id in &ids {
                let yt = YouTubePlayer::from_input(id).expect(id);
                assert_eq!(yt.id, should.id);
            }
        }

        // with time
        {
            let ids = [
                "https://youtu.be/mZpa3nOLOa8?list=PLDfU1tT3TQ16cW3WdAKf2WicS6wrdgZxB&t=69",
                "https://www.youtube.com/watch?v=mZpa3nOLOa8&list=PLDfU1tT3TQ16cW3WdAKf2WicS6wrdgZxB&index=1&t=69",
            ];
            let should = YouTubePlayer {
                id: "mZpa3nOLOa8".to_string(),
                is_playlist: false,
                time: Duration::from_secs(69),
                ..Default::default()
            };
            for id in &ids {
                let yt = YouTubePlayer::from_input(id).expect(id);
                assert_eq!(yt.id, should.id);
                assert_eq!(yt.time, should.time);
            }
        }
    }
}
