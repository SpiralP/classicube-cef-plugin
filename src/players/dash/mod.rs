use super::{helpers::start_update_loop, mute_lose_focus::IS_FOCUSED, PlayerTrait, WebPlayer};
use crate::{
    async_manager::AsyncManager,
    cef::{RustRefBrowser, RustV8Value},
    chat::Chat,
    error::*,
};
use classicube_helpers::{color, CellGetSet};
use futures::{future::RemoteHandle, prelude::*};
use log::debug;
use serde::{Deserialize, Serialize};
use std::path::Path;
use url::Url;

const PAGE_HTML: &str = include_str!("page.html");

#[derive(Debug, Serialize, Deserialize)]
pub struct DashPlayer {
    pub url: String,

    // 0-1
    pub volume: f32,

    #[serde(skip)]
    real_volume: f32,

    pub global_volume: bool,

    #[serde(skip)]
    should_send: bool,

    #[serde(skip)]
    volume_loop_handle: Option<RemoteHandle<()>>,

    #[serde(skip)]
    last_title: String,
}

impl Default for DashPlayer {
    fn default() -> Self {
        Self {
            url: String::new(),
            volume: 1.0,
            real_volume: 1.0,
            global_volume: false,
            should_send: true,
            volume_loop_handle: None,
            last_title: String::new(),
        }
    }
}

impl Clone for DashPlayer {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            volume: self.volume,
            real_volume: self.real_volume,
            global_volume: self.global_volume,
            should_send: self.should_send,
            ..Default::default()
        }
    }
}

impl PlayerTrait for DashPlayer {
    fn type_name(&self) -> &'static str {
        "Dash"
    }

    fn from_input(url: &str) -> Result<Self> {
        // make sure it's a normal url
        WebPlayer::from_input(url)?;

        let url = Url::parse(url)?;
        Ok(Self::from_url(&url)?)
    }

    fn on_create(&mut self) -> String {
        debug!("DashPlayer on_create {}", self.url);

        let real_volume = if IS_FOCUSED.get() { self.volume } else { 0.0 };
        self.real_volume = real_volume;

        format!(
            "data:text/html;base64,{}",
            base64::encode(
                PAGE_HTML
                    .replace("DASH_URL", &self.url)
                    .replace("START_VOLUME", &format!("{}", real_volume))
            )
        )
    }

    fn on_page_loaded(&mut self, entity_id: usize, _browser: &RustRefBrowser) {
        let (f, remote_handle) = start_update_loop(entity_id).remote_handle();
        self.volume_loop_handle = Some(remote_handle);

        AsyncManager::spawn_local_on_main_thread(f);
    }

    fn on_title_change(&mut self, _entity_id: usize, _browser: &RustRefBrowser, title: String) {
        if self.last_title == title || title == "Dash Loading" {
            return;
        }

        Chat::print(format!(
            "{}Now playing {}{}",
            color::TEAL,
            color::SILVER,
            title,
        ));

        self.last_title = title;
    }

    fn get_volume(&self, _browser: &RustRefBrowser) -> Result<f32> {
        Ok(self.volume)
    }

    /// volume is a float between 0-1
    fn set_volume(&mut self, browser: &RustRefBrowser, percent: f32) -> Result<()> {
        let real_volume = if IS_FOCUSED.get() { percent } else { 0.0 };

        if (real_volume - self.real_volume).abs() > 0.0001 {
            Self::get_player_field(browser, &format!("volume = {}", real_volume));

            self.real_volume = real_volume;
        }

        self.volume = percent;

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

    fn get_url(&self) -> String {
        self.url.clone()
    }

    fn get_title(&self) -> String {
        self.last_title.clone()
    }
}

impl DashPlayer {
    #[allow(dead_code)]
    pub async fn get_real_volume(browser: &RustRefBrowser) -> Result<f32> {
        let percent = match Self::eval_player_field(browser, "volume").await? {
            RustV8Value::Double(percent) => percent as f32,
            RustV8Value::Int(percent) => percent as f32,
            RustV8Value::UInt(percent) => percent as f32,

            _ => {
                bail!("non-number js value");
            }
        };

        Ok(percent)
    }

    fn get_player_field(browser: &RustRefBrowser, field: &str) {
        let code = format!(
            r#"if (typeof window.player !== "undefined") {{
                window.player.{};
            }}"#,
            field
        );
        browser.execute_javascript(code).unwrap();
    }

    async fn eval_player_field(browser: &RustRefBrowser, field: &str) -> Result<RustV8Value> {
        let code = format!(
            r#"
                (() => {{
                    if (typeof window.player !== "undefined") {{
                        return window.player.{};
                    }}
                }})()
            "#,
            field
        );
        Ok(browser.eval_javascript(code).await?)
    }
}

impl DashPlayer {
    pub fn from_url(url: &Url) -> Result<Self> {
        if url.scheme() != "http" && url.scheme() != "https" {
            Err("not http/https".into())
        } else {
            let parts = url.path_segments().chain_err(|| "no path segments")?;
            let last_part = parts.last().chain_err(|| "no last_part")?;

            let path = Path::new(last_part);
            let ext = path
                .extension()
                .chain_err(|| "no extension")?
                .to_str()
                .chain_err(|| "to_str")?;

            match ext {
                "mpd" => Ok(Self {
                    url: url.to_string(),
                    ..Default::default()
                }),

                _ => Err("url didn't end with a dash .mpd file extension".into()),
            }
        }
    }
}
