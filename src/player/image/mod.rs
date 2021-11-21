use super::{helpers::get_ext, PlayerTrait};
use crate::{cef::RustRefBrowser, chat::Chat, error::*, player::WebPlayer};
use classicube_helpers::color;
use serde::{Deserialize, Serialize};
use url::Url;

const PAGE_HTML: &str = include_str!("page.html");

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ImagePlayer {
    url: String,

    silent: bool,

    #[serde(skip)]
    last_title: String,
}

impl Clone for ImagePlayer {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            silent: self.silent,
            ..Default::default()
        }
    }
}

impl PlayerTrait for ImagePlayer {
    fn type_name(&self) -> &'static str {
        "Web"
    }

    fn from_input(url: &str) -> Result<Self> {
        // make sure it's a normal url
        WebPlayer::from_input(url)?;

        let url = Url::parse(url)?;
        Self::from_url(&url)
    }

    fn on_create(&mut self) -> String {
        format!(
            "data:text/html;base64,{}",
            base64::encode(PAGE_HTML.replace("IMAGE_URL", &self.url))
        )
    }

    fn on_title_change(&mut self, _entity_id: usize, _browser: &RustRefBrowser, title: String) {
        if self.last_title == title || title == "Image Loading" {
            return;
        }

        if !self.silent {
            Chat::print(format!(
                "{}Now showing {}{}",
                color::TEAL,
                color::SILVER,
                title,
            ));
        }

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

    fn set_silent(&mut self, silent: bool) -> Result<()> {
        self.silent = silent;
        Ok(())
    }
}

impl ImagePlayer {
    pub fn from_url(url: &Url) -> Result<Self> {
        if url.scheme() != "http" && url.scheme() != "https" {
            Err("not http/https".into())
        } else {
            match get_ext(url)? {
                "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "apng" | "avif" | "jfif"
                | "pjpeg" | "pjp" | "image" => Ok(Self {
                    url: url.to_string(),
                    ..Default::default()
                }),

                _ => Err("url didn't end with an image file extension".into()),
            }
        }
    }
}
