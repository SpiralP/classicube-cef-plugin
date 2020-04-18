use super::Player;
use crate::error::*;
use log::debug;
use regex::Regex;
use std::{collections::HashMap, time::Duration};
use url::Url;

#[derive(Debug, PartialEq)]
pub struct YoutubePlayer {
    id: String,
    time: Duration,
}

const PAGE_HTML: &str = include_str!("page.html");

impl Player for YoutubePlayer {
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

    fn on_create(&mut self) -> String {
        debug!("YoutubePlayer on_create {}", self.id);

        format!(
            "data:text/html;base64,{}",
            base64::encode(
                PAGE_HTML
                    .replace("VIDEO_ID", &self.id)
                    .replace("START_TIME", &format!("{}", self.time.as_secs()))
            )
        )
    }

    // fn on_page_loaded(&mut self, browser: &mut RustRefBrowser) {
    //     debug!("YoutubePlayer on_page_loaded {}", self.id);

    //     // ["play", id] => {
    //     //     cef.run_script(format!("player.loadVideoById(\"{}\");", id));
    //     // }

    //     // ["volume", percent] => {
    //     //     // 0 to 100
    //     //     cef.run_script(format!("player.setVolume({});", percent));
    //     // }

    //     browser
    //         .execute_javascript(format!(
    //             "player.loadVideoById(\"{}\", {});",
    //             self.id,
    //             self.time.as_secs()
    //         ))
    //         .unwrap();
    // }

    // TODO spawn a timer, remove it on Drop
    // fn on_tick(&mut self, _browser: RustRefBrowser) {
    // self.tokio_runtime.as_mut().unwrap().spawn(async {
    //     // :(
    //     tokio::time::delay_for(Duration::from_millis(2000)).await;

    //     loop {
    //         tokio::time::delay_for(Duration::from_millis(100)).await;

    //         Self::run_on_main_thread(async {
    //             let me = unsafe { &*Entities.List[ENTITIES_SELF_ID as usize] };
    //             let player_pos = Vec3 {
    //                 X: 64.0 - 4.0,
    //                 Y: 48.0,
    //                 Z: 64.0,
    //             };

    //             let percent = (player_pos - me.Position).length_squared() * 0.4;
    //             let percent = (100.0 - percent).max(0.0).min(100.0);

    //             let code = format!(
    //                 r#"if (window.player && window.player.setVolume) {{
    //                     window.player.setVolume({});
    //                 }}"#,
    //                 percent
    //             );
    //             let c_str = CString::new(code).unwrap();
    //             unsafe {
    //                 assert_eq!(crate::bindings::cef_run_script(c_str.as_ptr()), 0);
    //             }
    //         })
    //         .await;
    //     }
    // });
    // }
}

impl YoutubePlayer {
    fn from_id(id: String) -> Option<Self> {
        let regex = Regex::new(r"^[A-Za-z0-9_\-]{11}$").ok()?;
        if !regex.is_match(&id) {
            return None;
        }

        Some(Self {
            id,
            time: Duration::from_secs(0),
        })
    }

    fn from_id_and_time(id: String, time: Duration) -> Option<Self> {
        let mut this = Self::from_id(id)?;
        this.time = time;

        Some(this)
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
        ];

        let should = YoutubePlayer {
            id: "gQngg8iQipk".into(),
            time: Duration::from_secs(0),
        };
        for &url in &without_time {
            assert_eq!(YoutubePlayer::from_input(url).unwrap(), should);
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
        };
        for &url in &with_time {
            assert_eq!(YoutubePlayer::from_input(url).unwrap(), should);
        }
    }

    assert_eq!(
        YoutubePlayer::from_input("gQngg8iQipk").unwrap(),
        YoutubePlayer {
            id: "gQngg8iQipk".into(),
            time: Duration::from_secs(0),
        }
    );

    // not 11 chars
    assert!(YoutubePlayer::from_input("gQngg8iQip").is_err());

    // blank input
    assert!(YoutubePlayer::from_input("").is_err());
}
