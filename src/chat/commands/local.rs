//! commands that should only run on the person who said them

use super::{helpers::get_camera_trace, Chat};
use crate::{
    api,
    chat::{hidden_communication::whispers, PlayerSnapshot},
    entity_manager::{EntityManager, TargetEntity},
    error::{Result, ResultExt},
    helpers::format_duration,
};
use clap::Subcommand;
use classicube_helpers::async_manager;
use classicube_helpers::color::SILVER;
use classicube_sys::{
    Entities, Vec3, ENTITIES_SELF_ID, FACE_CONSTS, FACE_CONSTS_FACE_XMAX, FACE_CONSTS_FACE_XMIN,
    FACE_CONSTS_FACE_YMAX, FACE_CONSTS_FACE_YMIN, FACE_CONSTS_FACE_ZMAX, FACE_CONSTS_FACE_ZMIN,
};
use std::time::Duration;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Search youtube and play the first result
    Search {
        #[arg(required(true))]
        search: Vec<String>,
    },

    /// Move screen to the block you are aiming at
    There {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,
    },

    /// Open devtools
    #[command(alias("devtool"))]
    Devtools {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,
    },

    /// Re-sync all screens from someone else
    Sync { player_name: String },

    /// Fake a crash via panic!()
    #[command(alias("panic"), hide(true))]
    Crash,
}

pub async fn run(player: PlayerSnapshot, commands: Commands) -> Result<()> {
    match commands {
        Commands::Search { search } => {
            let input = search.join(" ");
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
            Chat::print(format!("{SILVER}{title}"));

            let video_id = video.id;
            Chat::send(format!("cef play {video_id}"));
        }

        Commands::There { name } => {
            let trace = get_camera_trace().chain_err(|| "no picked block")?;

            // the block's hit face
            let (mult, yaw) = match trace.Closest as FACE_CONSTS {
                FACE_CONSTS_FACE_XMIN => (Vec3::new(-0.01, 0.0, 0.0), 270.0),
                FACE_CONSTS_FACE_XMAX => (Vec3::new(1.01, 0.0, 1.0), 90.0),
                FACE_CONSTS_FACE_ZMIN => (Vec3::new(1.0, 0.0, -0.01), 0.0),
                FACE_CONSTS_FACE_ZMAX => (Vec3::new(0.0, 0.0, 1.01), 180.0),
                FACE_CONSTS_FACE_YMIN | FACE_CONSTS_FACE_YMAX => {
                    let me = unsafe { &*Entities.List[ENTITIES_SELF_ID as usize] };
                    let snap = (me.Yaw + 45.0 / 2.0) / 45.0;
                    let snap = snap.abs().floor() * 45.0;
                    let snap = snap + 180f32;

                    (Vec3::new(0.5, 1.0, 0.5), snap)
                }

                _ => {
                    return Err("oh no".into());
                }
            };

            // let middle = Vec3::from(trace.pos) + Vec3::new(0.5, 0.0, 0.5);
            let position = Vec3::from(trace.pos) + mult;
            // let position = position - Vec3::new(0.5, 0.0, 0.5);

            let maybe_name = name.map(|name| format!(" -n {name}")).unwrap_or_default();
            let Vec3 { X, Y, Z } = position;
            let pitch = 0.0;
            Chat::send(format!("cef at{maybe_name} {X} {Y} {Z} {yaw} {pitch}"));
        }

        Commands::Devtools { name } => {
            EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                |entity| {
                    entity
                        .browser
                        .as_ref()
                        .chain_err(|| "no browser")?
                        .open_dev_tools()
                },
            )?;
        }

        Commands::Sync { player_name } => {
            // TODO realname search
            let had_data = whispers::outgoing::query_whisper(&player_name).await?;
            if had_data {
                Chat::print("sync OK");
            } else {
                Chat::print("sync failed");
            }

            // TODO 0 args, randomly chosen? maybe everyone like map join?
        }

        Commands::Crash => {
            panic!("here's your crash!");
        }
    }

    Ok(())
}
