use super::{helpers::*, Chat};
use crate::{
    chat::{hidden_communication::whispers, PlayerSnapshot},
    entity_manager::EntityManager,
    error::*,
    search,
};
use clap::{App, Arg, ArgMatches};
use classicube_sys::{
    Entities, Vec3, ENTITIES_SELF_ID, FACE_CONSTS, FACE_CONSTS_FACE_XMAX, FACE_CONSTS_FACE_XMIN,
    FACE_CONSTS_FACE_YMAX, FACE_CONSTS_FACE_YMIN, FACE_CONSTS_FACE_ZMAX, FACE_CONSTS_FACE_ZMIN,
};

// commands that should only run on the person who said them
pub fn add_commands(app: App<'static, 'static>) -> App<'static, 'static> {
    app.subcommand(
        App::new("search")
            .about("Search youtube and play the first result")
            .arg(Arg::with_name("search").required(true).multiple(true)),
    )
    .subcommand(App::new("there").about("Move the closest screen to the block you are aiming at"))
    .subcommand(App::new("devtools").alias("devtool").about("Open devtools"))
    .subcommand(
        App::new("sync")
            .about("Re-sync all screens from someone else")
            .arg(Arg::with_name("player-name")),
    )
}

pub async fn handle_command(
    player: &PlayerSnapshot,
    matches: &ArgMatches<'static>,
) -> Result<bool> {
    match matches.subcommand() {
        ("search", Some(matches)) => {
            let args = matches.values_of_lossy("search").unwrap();
            let input = args.join(" ");
            let input = (*input).to_string();
            let id = search::youtube::search(&input).await?;

            Chat::send(format!("cef play {}", id));

            Ok(true)
        }

        ("there", Some(_matches)) => {
            let trace = get_camera_trace().chain_err(|| "no picked block")?;

            // the block's hit face
            let (mult, yaw) = match trace.Closest as FACE_CONSTS {
                FACE_CONSTS_FACE_XMIN => (Vec3::new(-0.01, 0.0, 0.0), 270.0),
                FACE_CONSTS_FACE_XMAX => (Vec3::new(1.01, 0.0, 0.0), 90.0),
                FACE_CONSTS_FACE_ZMIN => (Vec3::new(0.0, 0.0, -0.01), 0.0),
                FACE_CONSTS_FACE_ZMAX => (Vec3::new(0.0, 0.0, 1.01), 180.0),
                FACE_CONSTS_FACE_YMIN | FACE_CONSTS_FACE_YMAX => {
                    let me = unsafe { &*Entities.List[ENTITIES_SELF_ID as usize] };
                    let snap = (me.Yaw + 45.0 / 2.0) / 45.0;
                    let snap = snap as u32 * 45;
                    let snap = snap as f32 + 180f32;

                    (Vec3::new(0.5, 1.0, 0.5), snap)
                }

                _ => {
                    return Err("oh no".into());
                }
            };

            // let middle = Vec3::from(trace.pos) + Vec3::new(0.5, 0.0, 0.5);
            let position = Vec3::from(trace.pos) + mult;
            // let position = position - Vec3::new(0.5, 0.0, 0.5);

            Chat::send(format!(
                "cef at {} {} {} {} {}",
                position.X, position.Y, position.Z, yaw, 0.0
            ));

            Ok(true)
        }

        ("devtools", Some(_matches)) => {
            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;

            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            browser.open_dev_tools()?;

            Ok(true)
        }

        ("sync", Some(matches)) => {
            // only remove synced browsers
            let mut entity_ids: Vec<usize> = EntityManager::with_all_entities(|entities| {
                entities
                    .iter()
                    .filter(|(_, entity)| entity.should_send())
                    .map(|(&id, _)| id)
                    .collect()
            });

            for id in entity_ids.drain(..) {
                EntityManager::remove_entity(id).await?;
            }

            // TODO realname search
            if let Some(name) = matches.value_of("player-name") {
                let had_data = whispers::outgoing::query_whisper(name).await?;
                if had_data {
                    Chat::print("sync yes");
                } else {
                    Chat::print("sync no");
                }
            } else {
                // TODO randomly chosen
                bail!("0 args TODO");
            }

            Ok(true)
        }

        _ => Ok(false),
    }
}
