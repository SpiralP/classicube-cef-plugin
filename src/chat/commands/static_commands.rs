use super::helpers::*;
use crate::{
    async_manager,
    chat::PlayerSnapshot,
    entity_manager::{EntityBuilder, EntityManager, TargetEntity},
    error::*,
    player::{PlayerBuilder, VolumeMode},
};
use clap::{App, Arg, ArgMatches};

// static commands not targetted at a specific entity
pub fn add_commands(app: App<'static, 'static>) -> App<'static, 'static> {
    // url has to be multiple because urls can be chopped in half by
    // line continuations, so we join the parts together as a hack

    app.subcommand(
        App::new("create")
            .about("Creates a new screen")
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
                    .help("Hidden screen with unchanging volume"),
            )
            .arg(
                Arg::with_name("silent")
                    .long("silent")
                    .short("q")
                    .help("Don't show Now Playing messages"),
            )
            .arg(
                Arg::with_name("loop")
                    .long("loop")
                    .short("l")
                    .help("Loop after track finishes playing"),
            )
            .arg(Arg::with_name("url").multiple(true)),
    )
    .subcommand(
        App::new("closeall")
            .aliases(&["removeall", "clearall"])
            .about("Close all screens"),
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
            let global = matches.is_present("global");
            let should_loop = matches.is_present("loop");

            let should_send = !matches.is_present("no-send");
            let silent = matches.is_present("silent");

            if let Some(name) = matches.value_of("name") {
                if let Ok(id) = name.get_entity_id() {
                    let _ = EntityManager::remove_entity(id).await;
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

            let mut entity_builder = EntityBuilder::new(player)
                .queue(players.into())
                .insecure(insecure)
                .should_send(should_send);

            if global {
                // 1 fps, 1x1 resolution
                entity_builder = entity_builder.resolution(1, 1).frame_rate(1).scale(0.0);
            }

            if let Some(name) = matches.value_of("name") {
                entity_builder = entity_builder.name(name);
            }

            let entity_id = entity_builder.create()?;

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

        _ => Ok(false),
    }
}
