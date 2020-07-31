use super::Chat;
use crate::{
    chat::{hidden_communication::CURRENT_MAP_THEME, PlayerSnapshot},
    entity_manager::EntityManager,
    error::*,
    options,
    player::PlayerTrait,
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
                    .arg(Arg::with_name("bool").default_value(options::MUTE_LOSE_FOCUS.default())),
            )
            .subcommand(
                App::new("autoplay-map-themes")
                    .about("Auto-play map themes")
                    .arg(
                        Arg::with_name("bool")
                            .default_value(options::AUTOPLAY_MAP_THEMES.default()),
                    ),
            )
            .subcommand(
                App::new("subtitles")
                    .about("Show subtitles/closed captions on YouTube videos")
                    .arg(Arg::with_name("bool").default_value(options::SUBTITLES.default())),
            )
            .subcommand(
                App::new("volume")
                    .about("Global volume modifier")
                    .arg(Arg::with_name("percent").default_value(options::VOLUME.default())),
            )
            .subcommand(
                App::new("frame-rate")
                    .about("Changes default frame rate of newly created browsers")
                    .arg(Arg::with_name("fps").default_value(options::FRAME_RATE.default())),
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
                let value = options::MUTE_LOSE_FOCUS.get()?;
                if matches.occurrences_of("bool") > 0 {
                    let new_value = matches.value_of("bool").unwrap();
                    let new_value = new_value.parse()?;

                    options::MUTE_LOSE_FOCUS.set(new_value);
                    Chat::print(format!(
                        "mute-lose-focus: {} -> {}",
                        value,
                        options::MUTE_LOSE_FOCUS.get()?
                    ));
                } else {
                    Chat::print(format!("mute-lose-focus: {}", value));
                }

                Ok(true)
            }

            ("autoplay-map-themes", Some(matches)) => {
                let value = options::AUTOPLAY_MAP_THEMES.get()?;
                if matches.occurrences_of("bool") > 0 {
                    let new_value = matches.value_of("bool").unwrap();
                    let new_value = new_value.parse()?;

                    options::AUTOPLAY_MAP_THEMES.set(new_value);
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
                    Chat::print(format!("autoplay-map-themes: {}", value));
                }

                Ok(true)
            }

            ("subtitles", Some(matches)) => {
                let value = options::SUBTITLES.get()?;
                if matches.occurrences_of("bool") > 0 {
                    let new_value = matches.value_of("bool").unwrap();
                    let new_value = new_value.parse()?;

                    options::SUBTITLES.set(new_value);
                    Chat::print(format!(
                        "subtitles: {} -> {}",
                        value,
                        options::SUBTITLES.get()?
                    ));
                } else {
                    Chat::print(format!("subtitles: {}", value));
                }

                Ok(true)
            }

            ("volume", Some(matches)) => {
                let value = options::VOLUME.get()?;
                if matches.occurrences_of("percent") > 0 {
                    let volume = matches.value_of("percent").unwrap();
                    let volume = volume.parse()?;

                    options::VOLUME.set(volume);
                    Chat::print(format!("volume: {} -> {}", value, options::VOLUME.get()?));

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
                    Chat::print(format!("volume: {}", value));
                }

                Ok(true)
            }

            ("frame-rate", Some(matches)) => {
                let value = options::FRAME_RATE.get()?;
                if matches.occurrences_of("fps") > 0 {
                    let fps = matches.value_of("fps").unwrap();
                    let fps = fps.parse()?;

                    options::FRAME_RATE.set(fps);
                    Chat::print(format!(
                        "frame-rate: {} -> {}",
                        value,
                        options::FRAME_RATE.get()?
                    ));
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
