mod helpers;
mod options_commands;
mod screen_commands;
mod self_commands;
mod static_commands;

use std::cell::RefCell;

use clap::{App, AppSettings, Arg, ArgMatches};
use classicube_helpers::WithInner;
use tracing::{debug, warn};

use super::{Chat, PlayerSnapshot};
use crate::{
    async_manager,
    error::{bail, Error, Result},
};

thread_local!(
    static COMMAND_APP: RefCell<Option<App<'static, 'static>>> = RefCell::default();
);

pub fn initialize() {
    let app = App::new("cef")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .global_setting(AppSettings::DisableVersion)
        .global_setting(AppSettings::ColoredHelp)
        .arg(
            Arg::with_name("background")
                .long("background")
                .short("b")
                .help("Run task in background/spawn it"),
        );

    #[cfg(not(test))]
    let app = app.global_setting(AppSettings::ColorAlways);

    #[cfg(test)]
    let app = app.global_setting(AppSettings::ColorNever);

    let app = static_commands::add_commands(app);
    let app = self_commands::add_commands(app);
    let app = options_commands::add_commands(app);
    let app = screen_commands::add_commands(app);

    COMMAND_APP.with(|cell| {
        let cell = &mut *cell.borrow_mut();
        *cell = Some(app);
    });
}

pub fn get_matches(args: &[String]) -> Result<ArgMatches<'static>> {
    // we MUST clone here or we get a strange bug where reusing the same App gives different output
    // for example doing "cef -- help" then "cef help" will give
    //
    // error: The subcommand 'help' wasn't recognized
    //        Did you mean 'help'?
    //
    Ok(COMMAND_APP
        .with_inner_mut(|app| app.clone().get_matches_from_safe(args))
        .unwrap()?)
}

#[tracing::instrument(name = "commands::run", fields(player, is_self, show_errors, args = args.join(" ").as_str()))]
pub async fn run(
    player: PlayerSnapshot,
    mut args: Vec<String>,
    is_self: bool,
    show_errors: bool,
) -> Result<()> {
    args.insert(0, "cef".to_string());

    debug!("command {:?}", args);

    match get_matches(&args) {
        Ok(matches) => {
            let background = matches.is_present("background");

            let fut = async move {
                if static_commands::handle_command(&player, &matches).await?
                    || (is_self && self_commands::handle_command(&player, &matches).await?)
                    || (is_self && options_commands::handle_command(&player, &matches).await?)
                    || screen_commands::handle_command(&player, &matches).await?
                {
                    Ok::<_, Error>(())
                } else {
                    bail!("command not handled? {:?}", args);
                }
            };

            if background {
                async_manager::spawn_local_on_main_thread(async move {
                    if let Err(e) = fut.await {
                        warn!("backgrounded command: {}", e);
                    }
                });
            } else {
                fut.await?;
            }
        }

        Err(e) => {
            warn!("{:#?}", e);

            if show_errors || is_self {
                chat_print_lines(&format!("{e}"));
            }

            // TODO
            // don't error here because we already printed the error
        }
    }

    Ok(())
}

/// needs to keep same color code from last line
fn chat_print_lines(s: &str) {
    let s = s.trim();

    let lines: Vec<String> = s.split('\n').map(String::from).collect();

    let mut last_color = None;
    for line in lines {
        let message = if let Some(last_color) = last_color.filter(|c| *c != 'f') {
            format!("&{last_color}{line}")
        } else {
            line
        };

        Chat::print(&message);

        last_color = get_last_color(&message);
    }
}

fn get_last_color(text: &str) -> Option<char> {
    let mut last_color = None;
    let mut found_ampersand = false;

    for c in text.chars() {
        if c == '&' {
            found_ampersand = true;
        } else if found_ampersand {
            found_ampersand = false;
            last_color = Some(c);
        }
    }

    last_color
}

#[test]
fn test_commands() {
    crate::logger::initialize(true, true, false);
    crate::async_manager::initialize();
    self::initialize();

    async_manager::spawn_local_on_main_thread(async {
        run(
            unsafe { std::mem::zeroed() },
            vec!["--".into(), "help".into()],
            true,
            true,
        )
        .await
        .unwrap();

        run(
            unsafe { std::mem::zeroed() },
            vec!["help".into()],
            true,
            true,
        )
        .await
        .unwrap();
    });

    crate::async_manager::run();
    crate::async_manager::shutdown();
}
