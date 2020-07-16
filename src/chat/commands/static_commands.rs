use super::helpers::*;
use crate::{
    async_manager,
    chat::PlayerSnapshot,
    entity_manager::EntityManager,
    error::*,
    options::FRAME_RATE,
    players::{PlayerTrait, VolumeMode},
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
                Arg::with_name("insecure")
                    .long("insecure")
                    .short("i")
                    .help("allow insecure https connections"),
            )
            .arg(
                Arg::with_name("no-autoplay")
                    .long("no-autoplay")
                    .short("a")
                    .help("start paused"),
            )
            .arg(
                Arg::with_name("no-send")
                    .long("no-send")
                    .short("s")
                    .help("don't sync this screen to other players"),
            )
            .arg(
                Arg::with_name("global")
                    .long("global")
                    .short("g")
                    .help("hidden screen with unchanging volume"),
            )
            .arg(
                Arg::with_name("silent")
                    .long("silent")
                    .short("q")
                    .help("don't show Now Playing messages"),
            )
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .help("name the screen")
                    .takes_value(true),
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
    player: &PlayerSnapshot,
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
            let global = matches.is_present("global");

            let should_send = !matches.is_present("no-send");
            let silent = matches.is_present("silent");

            // 1 fps, 1x1 resolution
            let frame_rate = if global { 1 } else { FRAME_RATE.get()? };
            let resolution = if global { Some((1, 1)) } else { None };

            let entity_id = if let Some(name) = matches.value_of("name") {
                EntityManager::create_named_entity(
                    name,
                    &url,
                    frame_rate,
                    insecure,
                    resolution,
                    autoplay,
                    should_send,
                    silent,
                )?
            } else {
                EntityManager::create_entity(
                    &url,
                    frame_rate,
                    insecure,
                    resolution,
                    autoplay,
                    should_send,
                    silent,
                )?
            };

            EntityManager::with_entity(entity_id, |entity| {
                if global {
                    entity.set_scale(0.0);
                    entity.player.set_volume_mode(None, VolumeMode::Global)?;
                } else {
                    move_entity(entity, player);
                }

                Ok(())
            })?;

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
