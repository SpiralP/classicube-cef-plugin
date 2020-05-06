use super::PlayerTrait;
use crate::{cef::RustRefBrowser, chat::Chat, error::*};
use classicube_helpers::color;
use log::debug;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebPlayer {
    url: String,

    #[serde(skip)]
    last_title: String,

    #[serde(skip)]
    should_send: bool,
}

impl Default for WebPlayer {
    fn default() -> Self {
        Self {
            url: String::new(),
            last_title: String::new(),
            should_send: true,
        }
    }
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
        if self.last_title == title {
            return;
        }
        self.last_title = title.clone();

        Chat::print(format!(
            "{}Now viewing {}{}",
            color::TEAL,
            color::SILVER,
            title,
        ));
    }

    fn get_should_send(&self) -> bool {
        self.should_send
    }

    fn set_should_send(&mut self, should_send: bool) {
        self.should_send = should_send;
    }
}

impl WebPlayer {
    pub fn from_url(url: Url) -> Option<Self> {
        let has_tld = url
            .host()
            .map(|host| {
                if let url::Host::Domain(s) = host {
                    s.contains('.')
                } else {
                    // allow direct ips
                    true
                }
            })
            .unwrap_or(false);

        if has_tld {
            Some(Self {
                url: url.to_string(),
                ..Default::default()
            })
        } else {
            None
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
            assert!(WebPlayer::from_input(url).is_err(), url);
        }
    }
}
