use super::Chat;
use crate::{
    chat::PlayerSnapshot,
    error::*,
    options::{get_mute_lose_focus, set_mute_lose_focus},
    players::IS_FOCUSED,
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

            _ => Ok(false),
        },

        _ => Ok(false),
    }
}
