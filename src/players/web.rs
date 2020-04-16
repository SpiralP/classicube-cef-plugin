use super::Player;
use crate::error::*;
use log::debug;
use url::Url;

#[derive(Debug, PartialEq)]
pub struct WebPlayer {
    url: Url,
}

impl Player for WebPlayer {
    fn from_input(url: &str) -> Result<Self> {
        let url = Url::parse(url)?;

        if url.scheme() != "http" && url.scheme() != "https" {
            Err("not http/https".into())
        } else if let Some(this) = Self::from_url(url) {
            Ok(this)
        } else {
            Err("url didn't match whitelisted domains".into())
        }
    }

    fn on_create(&mut self) -> String {
        debug!("WebPlayer on_create {}", self.url);
        self.url.to_string()
    }
}

const WHITELISTED_HOSTS: &[&str] = &[
    "google.com",
    "www.google.com",
    "i.imgur.com",
    "imgur.com",
    "github.com",
    "classicube.net",
    "www.classicube.net",
];

impl WebPlayer {
    fn from_url(url: Url) -> Option<Self> {
        if WHITELISTED_HOSTS.contains(&url.host_str()?) {
            Some(Self { url })
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
                    url: url.parse().unwrap()
                }
            );
        }
    }

    {
        let bad_urls = [
            "file:///ohno.txt",
            "asdf",
            "ftp://google.com/file.txt",
            "http://virus.com",
            "",
        ];

        for &url in &bad_urls {
            assert!(WebPlayer::from_input(url).is_err());
        }
    }
}
