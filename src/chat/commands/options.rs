//! commands for setting local config options

use super::Chat;
use crate::{
    chat::hidden_communication::CURRENT_MAP_THEME,
    entity_manager::EntityManager,
    error::{Error, Result},
    options,
    player::PlayerTrait,
};
use clap::Subcommand;
use classicube_helpers::CellGetSet;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Set or get config options
    #[command(
        subcommand,
        aliases(["options", "settings", "option", "setting"]),
        subcommand_required(true),
        arg_required_else_help(true),
    )]
    Config(ConfigCommands),
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommands {
    /// Mute cef when you alt-tab out of the game
    MuteLoseFocus {
        #[arg(help(format!("[default: {}]", options::MUTE_LOSE_FOCUS.default())))]
        enabled: Option<bool>,
    },

    /// Auto-play map themes
    AutoplayMapThemes {
        #[arg(help(format!("[default: {}]", options::AUTOPLAY_MAP_THEMES.default())))]
        enabled: Option<bool>,
    },

    /// Show subtitles/closed captions on YouTube videos
    Subtitles {
        #[arg(help(format!("[default: {}]", options::SUBTITLES.default())))]
        enabled: Option<bool>,
    },

    /// Global volume modifier
    Volume {
        #[arg(help(format!("[default: {}]", options::VOLUME.default())))]
        percent: Option<f32>,
    },

    /// Map theme volume
    MapThemeVolume {
        #[arg(help(format!("[default: {}]", options::MAP_THEME_VOLUME.default())))]
        percent: Option<f32>,
    },

    /// Changes default frame rate of newly created browsers
    FrameRate {
        #[arg(help(format!("[default: {}]", options::FRAME_RATE.default())))]
        fps: Option<u16>,
    },
}

pub async fn run(commands: Commands) -> Result<()> {
    let Commands::Config(commands) = commands;

    match commands {
        ConfigCommands::MuteLoseFocus { enabled } => {
            let value = options::MUTE_LOSE_FOCUS.get()?;
            if let Some(enabled) = enabled {
                options::MUTE_LOSE_FOCUS.set(enabled);
                Chat::print(format!(
                    "mute-lose-focus: {} -> {}",
                    value,
                    options::MUTE_LOSE_FOCUS.get()?
                ));
            } else {
                Chat::print(format!("mute-lose-focus: {value}"));
            }
        }

        ConfigCommands::AutoplayMapThemes { enabled } => {
            let value = options::AUTOPLAY_MAP_THEMES.get()?;
            if let Some(enabled) = enabled {
                options::AUTOPLAY_MAP_THEMES.set(enabled);
                Chat::print(format!(
                    "autoplay-map-themes: {} -> {}",
                    value,
                    options::AUTOPLAY_MAP_THEMES.get()?
                ));

                if let Some(entity_id) = CURRENT_MAP_THEME.get() {
                    CURRENT_MAP_THEME.set(None);
                    let _ignore = EntityManager::remove_entity(entity_id).await;
                }
            } else {
                Chat::print(format!("autoplay-map-themes: {value}"));
            }
        }

        ConfigCommands::Subtitles { enabled } => {
            let value = options::SUBTITLES.get()?;
            if let Some(enabled) = enabled {
                options::SUBTITLES.set(enabled);
                Chat::print(format!(
                    "subtitles: {} -> {}",
                    value,
                    options::SUBTITLES.get()?
                ));
            } else {
                Chat::print(format!("subtitles: {value}"));
            }
        }

        ConfigCommands::Volume { percent } => {
            let value = options::VOLUME.get()?;
            if let Some(volume) = percent {
                options::VOLUME.set(volume);
                Chat::print(format!("volume: {value} -> {}", options::VOLUME.get()?));

                if let Some(entity_id) = CURRENT_MAP_THEME.get() {
                    EntityManager::with_entity(entity_id, |entity| {
                        entity.player.set_volume(entity.browser.as_ref(), volume)?;

                        Ok(())
                    })?;
                }

                EntityManager::with_all_entities(|entities| {
                    for entity in entities.values_mut() {
                        let volume = entity.player.get_volume();

                        // bad hacks because we only run javascript setVolume
                        // when screen volume has changed
                        let _ignore = entity.player.set_volume(entity.browser.as_ref(), 0.0);
                        let _ignore = entity.player.set_volume(entity.browser.as_ref(), volume);
                    }

                    Ok::<_, Error>(())
                })?;
            } else {
                Chat::print(format!("volume: {value}"));
            }
        }

        ConfigCommands::MapThemeVolume { percent } => {
            let value = options::MAP_THEME_VOLUME.get()?;
            if let Some(volume) = percent {
                options::MAP_THEME_VOLUME.set(volume);
                Chat::print(format!(
                    "map-theme-volume: {} -> {}",
                    value,
                    options::MAP_THEME_VOLUME.get()?
                ));

                if let Some(entity_id) = CURRENT_MAP_THEME.get() {
                    EntityManager::with_entity(entity_id, |entity| {
                        entity.player.set_volume(entity.browser.as_ref(), volume)?;

                        Ok(())
                    })?;
                }

                EntityManager::with_all_entities(|entities| {
                    for entity in entities.values_mut() {
                        let volume = entity.player.get_volume();

                        // bad hacks because we only run javascript setVolume
                        // when screen volume has changed
                        let _ignore = entity.player.set_volume(entity.browser.as_ref(), 0.0);
                        let _ignore = entity.player.set_volume(entity.browser.as_ref(), volume);
                    }

                    Ok::<_, Error>(())
                })?;
            } else {
                Chat::print(format!("map-theme-volume: {value}"));
            }
        }

        ConfigCommands::FrameRate { fps } => {
            let value = options::FRAME_RATE.get()?;
            if let Some(fps) = fps {
                options::FRAME_RATE.set(fps);
                Chat::print(format!(
                    "frame-rate: {} -> {}",
                    value,
                    options::FRAME_RATE.get()?
                ));
            } else {
                Chat::print(format!("frame-rate: {value}"));
            }
        }
    }

    Ok(())
}
