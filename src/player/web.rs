use classicube_helpers::color;
use serde::{Deserialize, Serialize};
use tracing::debug;
use url::Url;

use super::PlayerTrait;
use crate::{cef::RustRefBrowser, chat::Chat, error::Result};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebPlayer {
    url: String,

    #[serde(skip)]
    last_title: String,
}

impl PlayerTrait for WebPlayer {
    fn type_name(&self) -> &'static str {
        "Web"
    }

    fn from_input(url: &str) -> Result<Self> {
        let url = Url::parse(url)?;

        if url.scheme() != "http" && url.scheme() != "https" {
            Err("not http/https".into())
        } else if let Some(this) = Self::from_url(url) {
            Ok(this)
        } else {
            Err("not a normal url".into())
        }
    }

    fn on_create(&mut self) -> String {
        debug!("WebPlayer on_create {}", self.url);
        self.url.to_string()
    }

    fn on_title_change(&mut self, _entity_id: usize, _browser: &RustRefBrowser, title: String) {
        if self.last_title == title || title == Self::blank_page().url {
            return;
        }

        Chat::print(format!(
            "{}Now viewing {}{}",
            color::TEAL,
            color::SILVER,
            title,
        ));

        self.last_title = title;
    }

    fn get_url(&self) -> String {
        self.url.clone()
    }

    fn get_title(&self) -> String {
        self.last_title.clone()
    }

    fn is_finished_playing(&self) -> bool {
        // assume true because when someone does "cef play youtubething"
        // we want to skip the webpage for it
        true
    }
}

impl WebPlayer {
    pub fn from_url(url: Url) -> Option<Self> {
        let has_tld = url.host().map_or(false, |host| {
            if let url::Host::Domain(s) = host {
                s.contains('.')
            } else {
                // allow direct ips
                true
            }
        });

        if has_tld {
            Some(Self {
                url: url.to_string(),
                ..Default::default()
            })
        } else {
            None
        }
    }

    pub fn blank_page() -> Self {
        Self {
            url: "data:text/html,".to_string(),
            ..Default::default()
        }
    }
}

#[test]
fn test_web() {
    {
        let okay_urls = [
            "http://www.google.com/bap",
            "http://google.com/bap",
            "https://google.com/bap",
            "http://github.com/bap?okay=yes",
        ];

        for &url in &okay_urls {
            assert_eq!(
                WebPlayer::from_input(url).unwrap(),
                WebPlayer {
                    url: url.parse().unwrap(),
                    ..Default::default()
                }
            );
        }
    }

    {
        let bad_urls = [
            "file:///ohno.txt",
            "asdf",
            "ftp://google.com/file.txt",
            "data:text/html,<html>ohno</html>",
            "",
        ];

        for &url in &bad_urls {
            assert!(WebPlayer::from_input(url).is_err(), "{}", url);
        }
    }
}
