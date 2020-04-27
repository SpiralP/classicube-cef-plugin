use super::{Player, PlayerTrait};
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

    #[serde(skip)]
    pub start_time: Option<Instant>,

    // 0-1
    pub volume: f32,

    #[serde(skip)]
    volume_loop_handle: Option<RemoteHandle<()>>,

    #[serde(skip)]
    last_title: String,
}

impl Default for YoutubePlayer {
    fn default() -> Self {
        Self {
            id: String::new(),
            time: Duration::from_millis(0),
            volume: 1.0,
            start_time: None,
            volume_loop_handle: None,
            last_title: String::new(),
        }
    }
}

impl Clone for YoutubePlayer {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            time: self.time,
            volume: self.volume,
            // we need start_time because we use clone in encoding.rs
            start_time: self.start_time,
            ..Default::default()
        }
    }
}

impl PlayerTrait for YoutubePlayer {
    fn from_input(url_or_id: &str) -> Result<Self> {
        if let Ok(url) = Url::parse(url_or_id) {
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
        } else if let Some(this) = Self::from_id(url_or_id.to_string()) {
            Ok(this)
        } else {
            Err("couldn't match id or url from input".into())
        }
    }

    fn on_create(&mut self, entity_id: usize) -> String {
        debug!("YoutubePlayer on_create {}", self.id);

        let (f, remote_handle) = start_volume_loop(entity_id).remote_handle();
        self.volume_loop_handle = Some(remote_handle);

        AsyncManager::spawn_local_on_main_thread(f);

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

        if title == "YouTube Loading" {
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
        Self::seek_to(browser, time.as_secs());

        Ok(())
    }
}

async fn start_volume_loop(entity_id: usize) {
    loop {
        AsyncManager::sleep(Duration::from_millis(32)).await;

        let maybe_entity_pos = EntityManager::with_by_entity_id(entity_id, |entity| {
            if let Player::Youtube(yt) = &entity.player {
                if yt.start_time.is_some() {
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

                YoutubePlayer::set_volume(&browser, percent);
            }
        }
    }
}

impl YoutubePlayer {
    fn execute_player_method(browser: &RustRefBrowser, method_with_args: &str) {
        let code = format!(
            r#"if (
            typeof window.player !== "undefined" &&
                typeof window.player.setVolume !== "undefined"
            ) {{
                window.player.{};
            }}"#,
            method_with_args
        );
        browser.execute_javascript(code).unwrap();
    }

    /// volume is a float between 0-1
    fn set_volume(browser: &RustRefBrowser, percent: f32) {
        let percent = (percent * 100f32) as u32;

        Self::execute_player_method(browser, &format!("setVolume({})", percent))
    }

    fn seek_to(browser: &RustRefBrowser, seconds: u64) {
        // second arg true because:
        //
        // The allowSeekAhead parameter determines whether the player will
        // make a new request to the server if the seconds parameter specifies
        // a time outside of the currently buffered video data.
        Self::execute_player_method(browser, &format!("seekTo({}, true)", seconds))
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

    pub fn from_normal(url: &Url) -> Option<Self> {
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

    pub fn from_short(url: &Url) -> Option<Self> {
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

    pub fn from_embed(url: &Url) -> Option<Self> {
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
            "https://www.youtube.com/watch?v=gQngg8iQipk%list=ELG1JYZnaQbZc",
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
