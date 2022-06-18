use tracing::*;

use super::{Player, PlayerTrait, VolumeMode, YouTubePlayer};
use crate::{api, error::*};

#[derive(Debug, Default)]
pub struct PlayerBuilder {
    autoplay: Option<bool>,
    should_loop: Option<bool>,
    silent: Option<bool>,
    volume: Option<f32>,
    volume_mode: Option<VolumeMode>,
    use_youtube_playlist: bool,
}

impl PlayerBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    /// will always return at least 1 Player, or else Err about no results
    pub async fn build(self, input: &str) -> Result<Vec<Player>> {
        let mut players = Vec::new();

        let mut ids = vec![input.to_string()];
        if !self.use_youtube_playlist {
            if let Ok(player) = YouTubePlayer::from_input(input) {
                if player.is_playlist {
                    match api::youtube::playlist(&player.id).await {
                        Ok(video_ids) => {
                            if !video_ids.is_empty() {
                                ids = video_ids;
                            } else {
                                warn!("playlist gave 0 results?!");
                            }
                        }
                        Err(e) => {
                            warn!("couldn't fetch playlist videos: {}", e);
                        }
                    }
                }
            }
        }

        for input in ids {
            let mut player = Player::from_input(&input)?;

            if let Some(autoplay) = self.autoplay {
                player.set_autoplay(None, autoplay)?;
            }
            if let Some(should_loop) = self.should_loop {
                player.set_loop(None, should_loop)?;
            }
            if let Some(silent) = self.silent {
                player.set_silent(silent)?;
            }
            if let Some(volume) = self.volume {
                player.set_volume(None, volume)?;
            }
            if let Some(volume_mode) = self.volume_mode {
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

    /// if use_youtube_playlist is true, use YouTube's playlist instead of
    /// breaking up the playlist into individual players/videos
    pub fn use_youtube_playlist(mut self, use_youtube_playlist: bool) -> Self {
        self.use_youtube_playlist = use_youtube_playlist;
        self
    }
}
