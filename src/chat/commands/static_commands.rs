use clap::{App, Arg, ArgMatches};
use classicube_sys::{Chat_Send, OwnedString};

use super::helpers::move_entity;
use crate::{
    chat::PlayerSnapshot,
    entity_manager::{EntityBuilder, EntityManager, TargetEntity},
    error::{bail, Result},
    player::{Player, PlayerBuilder, VolumeMode},
};
use classicube_helpers::async_manager;

// static commands not targetted at a specific entity
pub fn add_commands(app: App<'static, 'static>) -> App<'static, 'static> {
    // url has to be multiple because urls can be chopped in half by
    // line continuations, so we join the parts together as a hack

    app.subcommand(
        App::new("create")
            .about("Creates a new screen")
            .long_about(
                "Creates a new screen\nThis will wait for page load unless --no-wait is specified",
            )
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .help("Name the screen")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("insecure")
                    .long("insecure")
                    .short("i")
                    .hidden(true)
                    .help("Allow insecure https connections"),
            )
            .arg(
                Arg::with_name("no-autoplay")
                    .long("no-autoplay")
                    .short("a")
                    .help("Start paused"),
            )
            .arg(
                Arg::with_name("no-wait")
                    .long("no-wait")
                    .short("w")
                    .hidden(true)
                    .help("Don't await for page load"),
            )
            .arg(
                Arg::with_name("no-send")
                    .long("no-send")
                    .short("s")
                    .help("Don't sync this screen to other players"),
            )
            .arg(
                Arg::with_name("global")
                    .long("global")
                    .short("g")
                    .help("Hidden screen with global/unchanging volume")
                    .long_help(
                        "Hidden screen with global/unchanging volume.\nAlso sets resolution to \
                         1x1 and fps to 1 for performance",
                    ),
            )
            .arg(
                Arg::with_name("silent")
                    .long("silent")
                    .alias("quiet")
                    .short("q")
                    .help("Don't show Now Playing messages")
                    .long_help(
                        "Don't show Now Playing messages\n(may not be allowed on some urls)",
                    ),
            )
            .arg(
                Arg::with_name("loop")
                    .long("loop")
                    .short("l")
                    .help("Loop after track finishes playing"),
            )
            .arg(
                Arg::with_name("transparent")
                    .long("transparent")
                    .short("t")
                    .help("Use transparent background")
                    .long_help(
                        "Use transparent background\nNote that text can appear strange (pixels \
                         missing)",
                    ),
            )
            .arg(Arg::with_name("url").multiple(true)),
    )
    .subcommand(
        App::new("closeall")
            .aliases(&["removeall", "clearall"])
            .about("Close all screens"),
    )
    .subcommand(
        App::new("reply")
            .about("Run a \"/Reply\" command on the server")
            .long_about(
                "Run a \"/Reply\" command on the server\nThis is useful to use in scripts to know \
                 when cef commands have finished executing.  For example when screens are \
                 finished loading via \"cef create\".",
            )
            .arg(Arg::with_name("num").required(true)),
    )
    .subcommand(
        App::new("replytwo")
            .about("Run a \"/ReplyTwo\" command on the server")
            .long_about(
                "Run a \"/ReplyTwo\" command on the server\nThis is useful to use in scripts to \
                 know when cef commands have finished executing.  For example when screens are \
                 finished loading via \"cef create\".",
            )
            .arg(Arg::with_name("num").required(true)),
    )
}

pub async fn handle_command(
    player_snapshot: &PlayerSnapshot,
    matches: &ArgMatches<'static>,
) -> Result<bool> {
    match matches.subcommand() {
        ("create", Some(matches)) => {
            let parts = matches.values_of_lossy("url").unwrap_or_default();
            let url = if parts.is_empty() {
                "https://www.classicube.net/".to_string()
            } else {
                parts.join("")
            };

            let insecure = matches.is_present("insecure");
            let autoplay = !matches.is_present("no-autoplay");
            let wait_for_page_load = !matches.is_present("no-wait");
            let should_send = !matches.is_present("no-send");
            let global = matches.is_present("global");
            let should_loop = matches.is_present("loop");
            let silent = matches.is_present("silent");

            if let Some(name) = matches.value_of("name") {
                if let Ok(id) = name.get_entity_id() {
                    drop(EntityManager::remove_entity(id).await);
                }
            }

            let mut player_builder = PlayerBuilder::new()
                .autoplay(autoplay)
                .should_loop(should_loop)
                .silent(silent);

            if global {
                player_builder = player_builder.volume_mode(VolumeMode::Global);
            }

            let mut players = player_builder.build(&url).await?;
            let player = players.remove(0);

            let transparent = if let Player::Image(_) = &player {
                true
            } else {
                matches.is_present("transparent")
            };

            let mut entity_builder = EntityBuilder::new(player)
                .queue(players.into())
                .insecure(insecure)
                .should_send(should_send);

            if global {
                // 1 fps, 1x1 resolution
                entity_builder = entity_builder.resolution(1, 1).frame_rate(1).scale(0.0);
            }

            if transparent {
                entity_builder = entity_builder.background_color(0x00FF_FFFF);
            }

            if let Some(name) = matches.value_of("name") {
                entity_builder = entity_builder.name(name);
            }

            let entity_id = entity_builder.create().await?;

            if !global {
                EntityManager::with_entity(entity_id, |entity| {
                    move_entity(entity, player_snapshot);
                    Ok(())
                })?;
            }

            if wait_for_page_load {
                let page_load =
                    EntityManager::with_entity(
                        entity_id,
                        |entity| Ok(entity.wait_for_page_load()),
                    )?;

                // wait for browser to load
                if page_load.await.is_err() {
                    bail!("wait_for_page_load cancelled");
                }
            }

            Ok(true)
        }

        ("closeall", Some(_matches)) => {
            async_manager::spawn_local_on_main_thread(async {
                let _ignore_error = EntityManager::remove_all_entities().await;
            });

            Ok(true)
        }

        ("reply", Some(matches)) => {
            let num = matches.value_of_lossy("num").unwrap();
            let owned_string = OwnedString::new(format!("/Reply {num}"));
            unsafe {
                Chat_Send(owned_string.as_cc_string(), 0);
            }

            Ok(true)
        }

        ("replytwo", Some(matches)) => {
            let num = matches.value_of_lossy("num").unwrap();
            let owned_string = OwnedString::new(format!("/ReplyTwo {num}"));
            unsafe {
                Chat_Send(owned_string.as_cc_string(), 0);
            }

            Ok(true)
        }

        _ => Ok(false),
    }
}
