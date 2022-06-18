use std::time::Duration;

use clap::{App, AppSettings, Arg, ArgMatches};
use classicube_helpers::color;
use classicube_sys::{
    Entities, Vec3, ENTITIES_SELF_ID, FACE_CONSTS, FACE_CONSTS_FACE_XMAX, FACE_CONSTS_FACE_XMIN,
    FACE_CONSTS_FACE_YMAX, FACE_CONSTS_FACE_YMIN, FACE_CONSTS_FACE_ZMAX, FACE_CONSTS_FACE_ZMIN,
};

use super::{helpers::get_camera_trace, Chat};
use crate::{
    api, async_manager,
    chat::{hidden_communication::whispers, PlayerSnapshot},
    entity_manager::EntityManager,
    error::{bail, Result, ResultExt},
    helpers::format_duration,
};

// commands that should only run on the person who said them
pub fn add_commands(app: App<'static, 'static>) -> App<'static, 'static> {
    app.subcommand(
        App::new("search")
            .about("Search youtube and play the first result")
            .arg(Arg::with_name("search").required(true).multiple(true)),
    )
    .subcommand(
        App::new("there")
            .about("Move screen to the block you are aiming at")
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            ),
    )
    .subcommand(
        App::new("devtools")
            .alias("devtool")
            .about("Open devtools")
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            ),
    )
    .subcommand(
        App::new("sync")
            .about("Re-sync all screens from someone else")
            .arg(Arg::with_name("player-name")),
    )
    .subcommand(
        App::new("crash")
            .setting(AppSettings::Hidden)
            .alias("panic"),
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
            let video =
                async_manager::timeout(Duration::from_secs(5), api::youtube::search(&input))
                    .await
                    .chain_err(|| "timed out")??;

            // Justice - Cross (Full Album) (49:21)
            let title = format!(
                "{} ({})",
                video.title,
                format_duration(Duration::from_secs(video.duration_seconds as _))
            );
            Chat::print(format!("{}{}", color::SILVER, title));

            Chat::send(format!("cef play {}", video.id));

            Ok(true)
        }

        ("there", Some(matches)) => {
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
                "cef at{} {} {} {} {} {}",
                matches
                    .value_of("name")
                    .map(|name| format!(" -n {}", name))
                    .unwrap_or_default(),
                position.X,
                position.Y,
                position.Z,
                yaw,
                0.0
            ));

            Ok(true)
        }

        ("devtools", Some(matches)) => {
            EntityManager::with_entity((matches, player), |entity| {
                entity
                    .browser
                    .as_ref()
                    .chain_err(|| "no browser")?
                    .open_dev_tools()
            })?;

            Ok(true)
        }

        ("sync", Some(matches)) => {
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

        ("crash", Some(_matches)) => {
            panic!("here's your crash!");
        }

        _ => Ok(false),
    }
}
