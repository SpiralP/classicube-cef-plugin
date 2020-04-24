use super::{Player, PlayerTrait};
use crate::{
    async_manager::AsyncManager, cef::RustRefBrowser, entity_manager::EntityManager, error::*,
};
use classicube_sys::{Entities, ENTITIES_SELF_ID};
use futures::{future::RemoteHandle, prelude::*};
use log::debug;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct YoutubePlayer {
    pub id: String,
    pub time: Duration,

    // 0-1
    pub volume: f32,

    #[serde(skip)]
    pub start_time: Option<Instant>,

    #[serde(skip)]
    volume_loop_handle: Option<RemoteHandle<()>>,
}

impl Clone for YoutubePlayer {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            time: self.time,
            volume: self.volume,
            start_time: None,
            volume_loop_handle: None,
        }
    }
}

const PAGE_HTML: &str = include_str!("page.html");

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
}

async fn start_volume_loop(entity_id: usize) {
    loop {
        AsyncManager::sleep(Duration::from_millis(32)).await;

        EntityManager::with_by_entity_id(entity_id, |entity| {
            if let Player::Youtube(yt) = &entity.player {
                if yt.start_time.is_some() {
                    // if we're loaded

                    let me = unsafe { &*Entities.List[ENTITIES_SELF_ID as usize] };
                    let entity_pos = entity.entity.Position;

                    let percent = (entity_pos - me.Position).length_squared().sqrt() / 10f32;
                    let percent = (1.0 - percent).max(0.0).min(1.0);
                    let percent = (percent * 100f32) as u32;

                    let code = format!(
                        r#"if (
                            typeof window.player !== "undefined" &&
                                typeof window.player.setVolume !== "undefined"
                            ) {{
                                window.player.setVolume({});
                            }}"#,
                        percent
                    );

                    if let Some(browser) = &mut entity.browser {
                        browser.execute_javascript(code).unwrap();
                    }
                }
            }

            Ok(())
        })
        .unwrap();
    }
}

impl YoutubePlayer {
    pub fn from_id(id: String) -> Option<Self> {
        let regex = Regex::new(r"^[A-Za-z0-9_\-]{11}$").ok()?;
        if !regex.is_match(&id) {
            return None;
        }

        Some(Self {
            id,
            time: Duration::from_secs(0),
            start_time: None,
            volume: 0.0,
            volume_loop_handle: None,
        })
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
        ];

        let should = YoutubePlayer {
            id: "gQngg8iQipk".into(),
            time: Duration::from_secs(0),
            start_time: None,
            volume: 0.0,
            volume_loop_handle: None,
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
            start_time: None,
            volume: 0.0,
            volume_loop_handle: None,
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
        start_time: None,
        volume: 0.0,
        volume_loop_handle: None,
    };
    assert_eq!(left.id, right.id);
    assert_eq!(left.time, right.time);

    // not 11 chars
    assert!(YoutubePlayer::from_input("gQngg8iQip").is_err());

    // blank input
    assert!(YoutubePlayer::from_input("").is_err());
}
