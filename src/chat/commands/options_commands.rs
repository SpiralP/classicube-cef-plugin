use super::Chat;
use crate::{
    chat::{hidden_communication::CURRENT_MAP_THEME, PlayerSnapshot},
    entity_manager::EntityManager,
    error::*,
    options::{
        get_autoplay_map_themes, get_frame_rate, get_map_theme_volume, get_mute_lose_focus,
        set_autoplay_map_themes, set_frame_rate, set_map_theme_volume, set_mute_lose_focus,
        AUTOPLAY_MAP_THEMES_DEFAULT, FRAME_RATE_DEFAULT, MAP_THEME_VOLUME_DEFAULT,
        MUTE_LOSE_FOCUS_DEFAULT,
    },
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
                    .arg(Arg::with_name("bool").default_value(MUTE_LOSE_FOCUS_DEFAULT)),
            )
            .subcommand(
                App::new("autoplay-map-themes")
                    .about("Auto-play map themes")
                    .arg(Arg::with_name("bool").default_value(AUTOPLAY_MAP_THEMES_DEFAULT)),
            )
            .subcommand(
                App::new("map-theme-volume")
                    .about("Map theme volume")
                    .arg(Arg::with_name("percent").default_value(MAP_THEME_VOLUME_DEFAULT)),
            )
            .subcommand(
                App::new("frame-rate")
                    .about("Changes default frame rate of newly created browsers")
                    .arg(Arg::with_name("fps").default_value(FRAME_RATE_DEFAULT)),
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
                if matches.occurrences_of("bool") > 0 {
                    let new_value = matches.value_of("bool").unwrap();
                    let new_value = new_value.parse()?;

                    let old_value = get_mute_lose_focus();
                    set_mute_lose_focus(new_value);
                    Chat::print(format!(
                        "mute-lose-focus: {} -> {}",
                        old_value,
                        get_mute_lose_focus()
                    ));

                    IS_FOCUSED.set(true);
                } else {
                    Chat::print(format!("mute-lose-focus: {}", get_mute_lose_focus()));
                }

                Ok(true)
            }

            ("autoplay-map-themes", Some(matches)) => {
                if matches.occurrences_of("bool") > 0 {
                    let new_value = matches.value_of("bool").unwrap();
                    let new_value = new_value.parse()?;

                    let old_value = get_autoplay_map_themes();
                    set_autoplay_map_themes(new_value);
                    Chat::print(format!(
                        "autoplay-map-themes: {} -> {}",
                        old_value,
                        get_autoplay_map_themes()
                    ));

                    if let Some(entity_id) = CURRENT_MAP_THEME.get() {
                        CURRENT_MAP_THEME.set(None);
                        let _ignore = EntityManager::remove_entity(entity_id).await;
                    }
                } else {
                    Chat::print(format!(
                        "autoplay-map-themes: {}",
                        get_autoplay_map_themes()
                    ));
                }

                Ok(true)
            }

            ("map-theme-volume", Some(matches)) => {
                if matches.occurrences_of("percent") > 0 {
                    let volume = matches.value_of("percent").unwrap();
                    let volume = volume.parse()?;

                    let old_value = get_map_theme_volume();
                    set_map_theme_volume(volume);
                    Chat::print(format!(
                        "map-theme-volume: {} -> {}",
                        old_value,
                        get_map_theme_volume()
                    ));

                    if let Some(entity_id) = CURRENT_MAP_THEME.get() {
                        let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
                        EntityManager::with_by_entity_id(entity_id, |entity| {
                            entity.player.set_volume(&browser, volume)?;

                            Ok(())
                        })?;
                    }
                } else {
                    Chat::print(format!("map-theme-volume: {}", get_map_theme_volume()));
                }

                Ok(true)
            }

            ("frame-rate", Some(matches)) => {
                if matches.occurrences_of("fps") > 0 {
                    let fps = matches.value_of("fps").unwrap();
                    let fps = fps.parse()?;

                    let old_value = get_frame_rate();
                    set_frame_rate(fps);
                    Chat::print(format!("frame-rate: {} -> {}", old_value, get_frame_rate()));
                } else {
                    Chat::print(format!("frame-rate: {}", get_frame_rate()));
                }

                Ok(true)
            }

            _ => Ok(false),
        },

        _ => Ok(false),
    }
}
