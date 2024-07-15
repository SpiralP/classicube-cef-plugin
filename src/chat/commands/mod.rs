mod global;
mod helpers;
mod local;
mod options;
mod screen;

use super::{Chat, PlayerSnapshot};
use crate::error::{Error, Result};
use clap::{Parser, Subcommand};
use classicube_helpers::async_manager;
use tracing::{debug, warn};

/// Cef video player
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    subcommand_required(true),
    arg_required_else_help(true),
    disable_version_flag(true)
)]
pub struct CefArgs {
    /// Run command in background/spawn it
    #[arg(short, long)]
    background: bool,

    #[command(subcommand)]
    sub: CefArgsSub,
}

#[derive(Subcommand, Debug)]
pub enum CefArgsSub {
    #[command(flatten)]
    Options(options::Commands),

    #[command(flatten)]
    Local(local::Commands),

    #[command(flatten)]
    Global(global::Commands),

    #[command(flatten)]
    Screen(screen::Commands),
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

    match CefArgs::try_parse_from(args) {
        Ok(args) => {
            debug!(?args, "CefArgs::try_parse_from");
            let fut = async move {
                match args.sub {
                    CefArgsSub::Global(args) => {
                        global::run(player, args).await?;
                    }
                    CefArgsSub::Local(args) => {
                        if is_self {
                            local::run(player, args).await?;
                        }
                    }
                    CefArgsSub::Options(args) => {
                        if is_self {
                            options::run(args).await?;
                        }
                    }
                    CefArgsSub::Screen(args) => {
                        screen::run(player, args).await?;
                    }
                }

                Ok::<_, Error>(())
            };

            if args.background {
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
    crate::logger::initialize(true, None, false);
    async_manager::initialize();

    async_manager::spawn_local_on_main_thread(async {
        run(
            unsafe { std::mem::zeroed() },
            "-- help".split(' ').map(str::to_string).collect(),
            true,
            true,
        )
        .await
        .unwrap();

        run(
            unsafe { std::mem::zeroed() },
            "-b".split(' ').map(str::to_string).collect(),
            true,
            true,
        )
        .await
        .unwrap();

        run(
            unsafe { std::mem::zeroed() },
            "help config mute-lose-focus"
                .split(' ')
                .map(str::to_string)
                .collect(),
            true,
            true,
        )
        .await
        .unwrap();
    });

    async_manager::run();
    async_manager::shutdown();
}
