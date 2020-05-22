use super::Chat;
use crate::{
    chat::{hidden_communication::CURRENT_MAP_THEME, PlayerSnapshot},
    entity_manager::EntityManager,
    error::*,
    options::{AUTOPLAY_MAP_THEMES, FRAME_RATE, MAP_THEME_VOLUME, MUTE_LOSE_FOCUS, SUBTITLES},
    players::{PlayerTrait, IS_FOCUSED},
};
use clap::{App, AppSettings, Arg, ArgMatches};
use classicube_helpers::CellGetSet;

// static commands not targetted at a specific entity
pub fn add_commands(app: App<'static, 'static>) -> App<'static, 'static> {
    app.subcommand(
        App::new("config")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .aliases(&["options", "settings", "option", "setting"])
            .about("Set or get config options")
            .subcommand(
                App::new("mute-lose-focus")
                    .about("Mute cef when you alt-tab out of the game")
                    .arg(Arg::with_name("bool").default_value(MUTE_LOSE_FOCUS.default())),
            )
            .subcommand(
                App::new("autoplay-map-themes")
                    .about("Auto-play map themes")
                    .arg(Arg::with_name("bool").default_value(AUTOPLAY_MAP_THEMES.default())),
            )
            .subcommand(
                App::new("subtitles")
                    .about("Show subtitles/closed captions on YouTube videos")
                    .arg(Arg::with_name("bool").default_value(SUBTITLES.default())),
            )
            .subcommand(
                App::new("map-theme-volume")
                    .about("Map theme volume")
                    .arg(Arg::with_name("percent").default_value(MAP_THEME_VOLUME.default())),
            )
            .subcommand(
                App::new("frame-rate")
                    .about("Changes default frame rate of newly created browsers")
                    .arg(Arg::with_name("fps").default_value(FRAME_RATE.default())),
            ),
    )
}

pub async fn handle_command(
    _player: &PlayerSnapshot,
    matches: &ArgMatches<'static>,
) -> Result<bool> {
    match matches.subcommand() {
        ("config", Some(matches)) => match matches.subcommand() {
            ("mute-lose-focus", Some(matches)) => {
                let value = MUTE_LOSE_FOCUS.get()?;
                if matches.occurrences_of("bool") > 0 {
                    let new_value = matches.value_of("bool").unwrap();
                    let new_value = new_value.parse()?;

                    MUTE_LOSE_FOCUS.set(new_value);
                    Chat::print(format!(
                        "mute-lose-focus: {} -> {}",
                        value,
                        MUTE_LOSE_FOCUS.get()?
                    ));

                    IS_FOCUSED.set(true);
                } else {
                    Chat::print(format!("mute-lose-focus: {}", value));
                }

                Ok(true)
            }

            ("autoplay-map-themes", Some(matches)) => {
                let value = AUTOPLAY_MAP_THEMES.get()?;
                if matches.occurrences_of("bool") > 0 {
                    let new_value = matches.value_of("bool").unwrap();
                    let new_value = new_value.parse()?;

                    AUTOPLAY_MAP_THEMES.set(new_value);
                    Chat::print(format!(
                        "autoplay-map-themes: {} -> {}",
                        value,
                        AUTOPLAY_MAP_THEMES.get()?
                    ));

                    if let Some(entity_id) = CURRENT_MAP_THEME.get() {
                        CURRENT_MAP_THEME.set(None);
                        let _ignore = EntityManager::remove_entity(entity_id).await;
                    }
                } else {
                    Chat::print(format!("autoplay-map-themes: {}", value));
                }

                Ok(true)
            }

            ("subtitles", Some(matches)) => {
                let value = SUBTITLES.get()?;
                if matches.occurrences_of("bool") > 0 {
                    let new_value = matches.value_of("bool").unwrap();
                    let new_value = new_value.parse()?;

                    SUBTITLES.set(new_value);
                    Chat::print(format!("subtitles: {} -> {}", value, SUBTITLES.get()?));
                } else {
                    Chat::print(format!("subtitles: {}", value));
                }

                Ok(true)
            }

            ("map-theme-volume", Some(matches)) => {
                let value = MAP_THEME_VOLUME.get()?;
                if matches.occurrences_of("percent") > 0 {
                    let volume = matches.value_of("percent").unwrap();
                    let volume = volume.parse()?;

                    MAP_THEME_VOLUME.set(volume);
                    Chat::print(format!(
                        "map-theme-volume: {} -> {}",
                        value,
                        MAP_THEME_VOLUME.get()?
                    ));

                    if let Some(entity_id) = CURRENT_MAP_THEME.get() {
                        let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
                        EntityManager::with_by_entity_id(entity_id, |entity| {
                            entity.player.set_volume(&browser, volume)?;

                            Ok(())
                        })?;
                    }
                } else {
                    Chat::print(format!("map-theme-volume: {}", value));
                }

                Ok(true)
            }

            ("frame-rate", Some(matches)) => {
                let value = FRAME_RATE.get()?;
                if matches.occurrences_of("fps") > 0 {
                    let fps = matches.value_of("fps").unwrap();
                    let fps = fps.parse()?;

                    FRAME_RATE.set(fps);
                    Chat::print(format!("frame-rate: {} -> {}", value, FRAME_RATE.get()?));
                } else {
                    Chat::print(format!("frame-rate: {}", value));
                }

                Ok(true)
            }

            _ => Ok(false),
        },

        _ => Ok(false),
    }
}
