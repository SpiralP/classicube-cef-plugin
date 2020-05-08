use super::Chat;
use crate::{
    chat::{hidden_communication::CURRENT_MAP_THEME, PlayerSnapshot},
    entity_manager::EntityManager,
    error::*,
    options::{
        get_autoplay_map_themes, get_map_theme_volume, get_mute_lose_focus,
        set_autoplay_map_themes, set_map_theme_volume, set_mute_lose_focus,
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
                    .arg(Arg::with_name("bool").required(false).default_value("true")),
            )
            .subcommand(
                App::new("autoplay-map-themes")
                    .about("Auto-play map themes")
                    .arg(Arg::with_name("bool").required(false).default_value("true")),
            )
            .subcommand(
                App::new("map-theme-volume").about("Map theme volume").arg(
                    Arg::with_name("percent")
                        .required(false)
                        .default_value("0.5"),
                ),
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

                    set_mute_lose_focus(new_value);
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

                    set_autoplay_map_themes(new_value);

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
                    let new_value = matches.value_of("percent").unwrap();
                    let new_value = new_value.parse()?;

                    set_map_theme_volume(new_value);

                    if let Some(entity_id) = CURRENT_MAP_THEME.get() {
                        let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
                        EntityManager::with_by_entity_id(entity_id, |entity| {
                            entity.player.set_volume(&browser, new_value)?;

                            Ok(())
                        })?;
                    }
                } else {
                    Chat::print(format!("map-theme-volume: {}", get_map_theme_volume()));
                }

                Ok(true)
            }

            _ => Ok(false),
        },

        _ => Ok(false),
    }
}
