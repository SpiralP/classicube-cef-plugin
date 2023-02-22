use base64::{prelude::BASE64_STANDARD, Engine};
use classicube_helpers::color::{SILVER, TEAL};
use futures::{future::RemoteHandle, prelude::*};
use serde::{Deserialize, Serialize};
use tracing::debug;
use url::Url;

use super::{
    helpers::{get_ext, start_update_loop},
    PlayerTrait, VolumeMode, WebPlayer,
};
use crate::{
    async_manager,
    cef::{RustRefBrowser, RustV8Value},
    chat::Chat,
    error::{bail, Result},
    options,
};

const PAGE_HTML: &str = include_str!("page.html");

#[derive(Debug, Serialize, Deserialize)]
pub struct DashPlayer {
    pub url: String,

    // 0-1
    volume: f32,
    volume_mode: VolumeMode,

    #[serde(skip)]
    pub update_loop_handle: Option<RemoteHandle<()>>,

    #[serde(skip)]
    last_title: String,
}

impl Default for DashPlayer {
    fn default() -> Self {
        Self {
            url: String::new(),
            volume: 1.0,
            volume_mode: VolumeMode::Distance {
                multiplier: 1.0,
                distance: 28.0,
            },
            update_loop_handle: None,
            last_title: String::new(),
        }
    }
}

impl Clone for DashPlayer {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            volume: self.volume,
            volume_mode: self.volume_mode,
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
        match get_ext(&url)? {
            "mpd" | "dash" => Ok(Self {
                url: url.to_string(),
                ..Default::default()
            }),

            _ => Err("url didn't end with a dash .mpd file extension".into()),
        }
    }

    fn on_create(&mut self) -> Result<String> {
        let url = self.url.to_string();
        Self::from_input(&url)?;
        debug!("DashPlayer on_create {}", url);

        Ok(format!(
            "data:text/html;base64,{}",
            BASE64_STANDARD.encode(
                PAGE_HTML
                    .replace("DASH_URL", &self.url)
                    .replace("START_VOLUME", &format!("{}", self.volume))
            )
        ))
    }

    fn on_page_loaded(&mut self, entity_id: usize, _browser: &RustRefBrowser) {
        let (f, remote_handle) = start_update_loop(entity_id).remote_handle();
        self.update_loop_handle = Some(remote_handle);
        async_manager::spawn_local_on_main_thread(f);
    }

    fn on_title_change(&mut self, _entity_id: usize, _browser: &RustRefBrowser, title: String) {
        if self.last_title == title || title == "DASH Stream Loading" {
            return;
        }

        Chat::print(format!("{TEAL}Now playing {SILVER}{title}"));

        self.last_title = title;
    }

    fn get_volume(&self) -> f32 {
        self.volume
    }

    /// volume is a float between 0-1
    fn set_volume(&mut self, browser: Option<&RustRefBrowser>, volume: f32) -> Result<()> {
        if let Some(browser) = browser {
            if (volume - self.volume).abs() > 0.0001 {
                let volume_modifier = options::VOLUME.get()?;
                Self::get_player_field(browser, &format!("volume = {}", volume * volume_modifier));
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
        _browser: Option<&RustRefBrowser>,
        mode: VolumeMode,
    ) -> Result<()> {
        if let VolumeMode::Panning { .. } = mode {
            bail!("panning is TODO");
        }
        self.volume_mode = mode;
        Ok(())
    }

    fn get_url(&self) -> String {
        self.url.clone()
    }

    fn get_title(&self) -> String {
        self.last_title.clone()
    }

    fn is_finished_playing(&self) -> bool {
        // TODO determine when we 404 after host closes stream
        false
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
        let _ignore = browser.execute_javascript(format!(
            r#"
                if (typeof window.player !== "undefined") {{
                    window.player.{field};
                }}
            "#
        ));
    }

    async fn eval_player_field(browser: &RustRefBrowser, field: &str) -> Result<RustV8Value> {
        let code = format!(
            r#"
                (() => {{
                    if (typeof window.player !== "undefined") {{
                        return window.player.{field};
                    }}
                }})()
            "#
        );
        browser.eval_javascript(code).await
    }
}
