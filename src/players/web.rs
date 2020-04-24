use super::PlayerTrait;
use crate::error::*;
use log::debug;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct WebPlayer {
    url: String,
}

impl PlayerTrait for WebPlayer {
    fn from_input(url: &str) -> Result<Self> {
        let url = Url::parse(url)?;

        if url.scheme() != "http" && url.scheme() != "https" {
            Err("not http/https".into())
        } else if let Some(this) = Self::from_url(url) {
            Ok(this)
        } else {
            Err("url didn't BAP".into())
        }
    }

    fn on_create(&mut self) -> String {
        debug!("WebPlayer on_create {}", self.url);
        self.url.to_string()
    }
}

impl WebPlayer {
    pub fn from_url(url: Url) -> Option<Self> {
        Some(Self {
            url: url.to_string(),
        })
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
            "data:text/html,<html>ohno</html>",
            "",
        ];

        for &url in &bad_urls {
            assert!(WebPlayer::from_input(url).is_err(), url);
        }
    }
}
