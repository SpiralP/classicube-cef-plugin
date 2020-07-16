use super::{Player, PlayerTrait, VolumeMode};
use crate::error::*;

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

    pub fn build(mut self, input: &str) -> Result<Player> {
        let mut player = Player::from_input(input)?;

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

        Ok(player)
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
