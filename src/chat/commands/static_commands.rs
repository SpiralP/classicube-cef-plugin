use super::helpers::*;
use crate::{
    async_manager, chat::PlayerSnapshot, entity_manager::EntityManager, error::*,
    options::FRAME_RATE,
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

            let entity_id = EntityManager::create_entity(
                &url,
                FRAME_RATE.get()?,
                matches.is_present("insecure"),
                None,
            )?;
            EntityManager::with_by_entity_id(entity_id, |entity| {
                move_entity(entity, player);

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
