use super::{Player, PlayerTrait, VolumeMode};
use crate::{api, error::*};
use std::collections::HashMap;
use url::Url;

fn get_playlist_id(url_or_id: &str) -> Option<String> {
    fn from_url(url: &Url) -> Option<String> {
        let host_str = url.host_str()?;
        if host_str == "youtube.com" || host_str == "www.youtube.com" || host_str == "youtu.be" {
            let query: HashMap<_, _> = url.query_pairs().collect();
            let id = query.get("list")?.to_string();

            Some(id)
        } else {
            None
        }
    }

    fn from_id(id: &str) -> Option<String> {
        // list= PLspeOI0YmcdPQJWbvMhOCg5RhGkNUoVJR
        if id.starts_with("PL") && (id.len() == 34 || id.len() == 18) {
            Some(id.to_string())
        } else {
            None
        }
    }

    if let Ok(url) = Url::parse(&url_or_id) {
        from_url(&url)
    } else if let Some(this) = from_id(url_or_id) {
        Some(this)
    } else {
        None
    }
}

#[test]
fn test_get_playlist_id() {
    for input in &[
        "https://youtu.be/F_HoMkkRHv8?list=PLpnh5xqG8PuONcW35KipnicwP4W3oyFMS",
        "https://www.youtube.com/watch?v=F_HoMkkRHv8&list=PLpnh5xqG8PuONcW35KipnicwP4W3oyFMS",
        "PLpnh5xqG8PuONcW35KipnicwP4W3oyFMS",
    ] {
        assert_eq!(
            get_playlist_id(input).unwrap(),
            "PLpnh5xqG8PuONcW35KipnicwP4W3oyFMS"
        );
    }

    for input in &[
        "https://youtu.be/F_HoMkkRHv8?list=PLF37D334894B07EEA",
        "https://www.youtube.com/watch?v=F_HoMkkRHv8&list=PLF37D334894B07EEA",
        "PLF37D334894B07EEA",
    ] {
        assert_eq!(get_playlist_id(input).unwrap(), "PLF37D334894B07EEA");
    }
}

#[derive(Debug, Default)]
pub struct PlayerBuilder {
    autoplay: Option<bool>,
    should_loop: Option<bool>,
    silent: Option<bool>,
    volume: Option<f32>,
    volume_mode: Option<VolumeMode>,
}

impl PlayerBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn build(mut self, input: &str) -> Result<Vec<Player>> {
        let mut players = Vec::new();

        let mut ids = vec![input.to_string()];
        if let Some(id) = get_playlist_id(input) {
            ids = api::youtube::playlist(&id).await?
        }

        for input in ids {
            let mut player = Player::from_input(&input)?;

            if let Some(autoplay) = self.autoplay.take() {
                player.set_autoplay(None, autoplay)?;
            }
            if let Some(should_loop) = self.should_loop.take() {
                player.set_loop(None, should_loop)?;
            }
            if let Some(silent) = self.silent.take() {
                player.set_silent(silent)?;
            }
            if let Some(volume) = self.volume.take() {
                player.set_volume(None, volume)?;
            }
            if let Some(volume_mode) = self.volume_mode.take() {
                player.set_volume_mode(None, volume_mode)?;
            }
            players.push(player);
        }

        Ok(players)
    }

    pub fn autoplay(mut self, autoplay: bool) -> Self {
        self.autoplay = Some(autoplay);
        self
    }

    pub fn should_loop(mut self, should_loop: bool) -> Self {
        self.should_loop = Some(should_loop);
        self
    }

    pub fn silent(mut self, silent: bool) -> Self {
        self.silent = Some(silent);
        self
    }

    #[allow(dead_code)]
    pub fn volume(mut self, volume: f32) -> Self {
        self.volume = Some(volume);
        self
    }

    pub fn volume_mode(mut self, volume_mode: VolumeMode) -> Self {
        self.volume_mode = Some(volume_mode);
        self
    }
}
