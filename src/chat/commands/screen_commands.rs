use super::helpers::*;
use crate::{
    async_manager::AsyncManager,
    cef::Cef,
    chat::PlayerSnapshot,
    entity_manager::{EntityManager, MODEL_HEIGHT, MODEL_WIDTH},
    error::*,
    players::PlayerTrait,
};
use clap::{App, Arg, ArgMatches};
use log::warn;
use nalgebra::*;
use ncollide3d::{query::*, shape::*};
use std::time::Duration;

// static commands not targetted at a specific entity
pub fn add_commands(app: App<'static, 'static>) -> App<'static, 'static> {
    app.subcommand(
        App::new("here")
            .alias("move")
            .about("Move the closest screen to you"),
    )
    .subcommand(
        App::new("play")
            .about("Play something on the closest screen")
            .arg(Arg::with_name("url").required(true).multiple(true)),
    )
    .subcommand(App::new("stop").about("Stop playing the closest screen"))
    .subcommand(
        App::new("close")
            .aliases(&["remove", "clear"])
            .about("Remove the closest screen"),
    )
    .subcommand(
        App::new("scale")
            .about("Scale the closest screen")
            .arg(Arg::with_name("scale").required(true)),
    )
    .subcommand(
        App::new("reload")
            .alias("refresh")
            .about("Reload the closest screen"),
    )
    .subcommand(
        App::new("angles")
            .alias("angle")
            .about("Change angles of the closest screen")
            .arg(Arg::with_name("yaw").required(true))
            .arg(Arg::with_name("pitch").required(false)),
    )
    .subcommand(
        App::new("click")
            .about("Click the closest screen")
            .long_about(
                "If x, y are specified click at that position, otherwise click where you are \
                 aiming",
            )
            .arg(Arg::with_name("x").required(false).requires("y"))
            .arg(Arg::with_name("y").required(false).requires("x")),
    )
    .subcommand(
        App::new("type")
            .about("Type text on the closest screen")
            .arg(Arg::with_name("words").required(true).multiple(true)),
    )
    .subcommand(
        App::new("resize")
            .alias("resolution")
            .about("Resize the resolution of the closest screen")
            .arg(Arg::with_name("width").required(true))
            .arg(Arg::with_name("height").required(true)),
    )
    .subcommand(
        App::new("volume")
            .about("Set volume of the closest screen")
            .arg(Arg::with_name("global").long("global").short("g"))
            .arg(Arg::with_name("volume").required(true)),
    )
    .subcommand(
        App::new("time")
            .alias("seek")
            .about("Seek time of the closest screen")
            .arg(Arg::with_name("time").required(true)),
    )
    .subcommand(
        App::new("at")
            .alias("tp")
            .about("Move the closest screen to coords x,y,z and optional yaw,pitch")
            .arg(Arg::with_name("x").required(true))
            .arg(Arg::with_name("y").required(true))
            .arg(Arg::with_name("z").required(true))
            .arg(Arg::with_name("yaw").required(false))
            .arg(Arg::with_name("pitch").required(false).requires("yaw")),
    )
}

pub async fn handle_command(
    player: &PlayerSnapshot,
    matches: &ArgMatches<'static>,
) -> Result<bool> {
    match matches.subcommand() {
        ("here", Some(_matches)) => {
            EntityManager::with_closest(player.eye_position, |entity| {
                move_entity(entity, player);

                Ok(())
            })?;

            Ok(true)
        }

        ("play", Some(matches)) => {
            let parts = matches.values_of_lossy("url").unwrap_or_default();
            let url: String = parts.join("");

            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;
            EntityManager::entity_play(&url, entity_id)?;

            Ok(true)
        }

        ("stop", Some(_matches)) => {
            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;

            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            browser.load_url("data:text/html,")?;

            Ok(true)
        }

        ("close", Some(_matches)) => {
            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;

            AsyncManager::spawn_local_on_main_thread(async move {
                if let Err(e) = EntityManager::remove_entity(entity_id).await {
                    warn!("{}", e);
                }
            });

            Ok(true)
        }

        ("scale", Some(matches)) => {
            let scale = matches.value_of("scale").unwrap().parse()?;

            EntityManager::with_closest(player.eye_position, move |entity| {
                entity.set_scale(scale);

                Ok(())
            })?;

            Ok(true)
        }

        ("reload", Some(_matches)) => {
            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;
            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            browser.reload()?;

            Ok(true)
        }

        ("angles", Some(matches)) => {
            EntityManager::with_closest(player.eye_position, |entity| {
                let yaw = matches.value_of("yaw").unwrap().parse()?;
                entity.entity.RotY = yaw;

                if let Some(pitch) = matches.value_of("pitch") {
                    let pitch = pitch.parse()?;
                    entity.entity.RotX = pitch;
                }

                Ok(())
            })?;

            Ok(true)
        }

        ("click", Some(matches)) => {
            if let Some(x) = matches.value_of("x") {
                let x = x.parse()?;

                if let Some(y) = matches.value_of("y") {
                    let y = y.parse()?;

                    let entity_id =
                        EntityManager::with_closest(player.eye_position, |closest_entity| {
                            Ok(closest_entity.id)
                        })?;

                    let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
                    browser.send_click(x, y)?;
                }
            } else {
                click_from_player(player)?;
            }

            Ok(true)
        }

        ("type", Some(matches)) => {
            let parts = matches.values_of_lossy("url").unwrap_or_default();
            let text: String = parts.join(" ");
            let text = (*text).to_string();

            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;

            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            browser.send_text(text)?;

            Ok(true)
        }

        ("resize", Some(matches)) => {
            let width = matches.value_of("width").unwrap().parse()?;
            let height = matches.value_of("height").unwrap().parse()?;

            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;

            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            Cef::resize_browser(&browser, width, height)?;

            Ok(true)
        }

        ("volume", Some(matches)) => {
            let volume = matches.value_of("volume").unwrap().parse()?;
            let global = matches.is_present("global");

            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;

            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            EntityManager::with_closest(player.eye_position, |entity| {
                entity.player.set_volume(&browser, volume)?;
                entity.player.set_global_volume(global)?;
                Ok(())
            })?;

            Ok(true)
        }

        ("time", Some(matches)) => {
            let time = matches.value_of("time").unwrap();

            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;

            let seconds: u64 = if let Ok(seconds) = time.parse() {
                seconds
            } else {
                // try 12:34 mm:ss format

                let parts: Vec<_> = time.split(':').collect();
                match parts.as_slice() {
                    [hours, minutes, seconds] => {
                        let hours: u64 = hours.parse()?;
                        let minutes: u64 = minutes.parse()?;
                        let seconds: u64 = seconds.parse()?;

                        seconds + minutes * 60 + hours * 60 * 60
                    }

                    [minutes, seconds] => {
                        let minutes: u64 = minutes.parse()?;
                        let seconds: u64 = seconds.parse()?;

                        seconds + minutes * 60
                    }

                    _ => {
                        // let parts:Vec<_> = time.split("%").collect();
                        // TODO 20%

                        bail!("bad format");
                    }
                }
            };

            EntityManager::with_by_entity_id(entity_id, |entity| {
                let browser = entity.browser.as_ref().chain_err(|| "no browser")?;

                entity
                    .player
                    .set_current_time(&browser, Duration::from_secs(seconds))?;

                Ok(())
            })?;

            Ok(true)
        }

        ("at", Some(matches)) => {
            EntityManager::with_closest(player.eye_position, |entity| {
                let x = matches.value_of("x").unwrap().parse()?;
                let y = matches.value_of("y").unwrap().parse()?;
                let z = matches.value_of("z").unwrap().parse()?;

                entity.entity.Position.set(x, y, z);

                if let Some(yaw) = matches.value_of("yaw") {
                    let yaw = yaw.parse()?;
                    entity.entity.RotY = yaw;

                    if let Some(pitch) = matches.value_of("pitch") {
                        let pitch = pitch.parse()?;
                        entity.entity.RotX = pitch;
                    }
                }

                Ok(())
            })?;

            Ok(true)
        }

        _ => Ok(false),
    }
}

fn click_from_player(player: &PlayerSnapshot) -> Result<()> {
    let (entity_id, entity_pos, [entity_pitch, entity_yaw], entity_scale) =
        EntityManager::with_closest(player.eye_position, |closest_entity| {
            Ok((
                closest_entity.id,
                closest_entity.entity.Position,
                [closest_entity.entity.RotX, closest_entity.entity.RotY],
                closest_entity.entity.ModelScale,
            ))
        })?;

    fn intersect(
        eye_pos: Point3<f32>,
        [aim_pitch, aim_yaw]: [f32; 2],
        screen_pos: Point3<f32>,
        [screen_pitch, screen_yaw]: [f32; 2],
    ) -> Option<(Ray<f32>, Plane<f32>, RayIntersection<f32>)> {
        // when angles 0 0, aiming towards -z
        let normal = -Vector3::<f32>::z_axis();

        let aim_dir =
            Rotation3::from_euler_angles(-aim_pitch.to_radians(), -aim_yaw.to_radians(), 0.0)
                .transform_vector(&normal);

        // positive pitch is clockwise on the -x axis
        // positive yaw is clockwise on the -y axis
        let rot = UnitQuaternion::from_euler_angles(
            -screen_pitch.to_radians(),
            -screen_yaw.to_radians(),
            0.0,
        );
        let iso = Isometry3::from_parts(screen_pos.coords.into(), rot);

        let ray = Ray::new(eye_pos, aim_dir);
        let plane = Plane::new(normal);
        if let Some(intersection) = plane.toi_and_normal_with_ray(&iso, &ray, 10.0, true) {
            if intersection.toi == 0.0 {
                // 0 if aiming from wrong side
                None
            } else {
                Some((ray, plane, intersection))
            }
        } else {
            None
        }
    }

    let eye_pos = vec3_to_vector3(&player.eye_position);
    let screen_pos = vec3_to_vector3(&entity_pos);

    if let Some((ray, _plane, intersection)) = intersect(
        eye_pos.into(),
        [player.Pitch, player.Yaw],
        screen_pos.into(),
        [entity_pitch, entity_yaw],
    ) {
        let intersection_point = ray.point_at(intersection.toi).coords;

        let forward = intersection.normal;

        let tmp = Vector3::y();
        let right = Vector3::cross(&forward, &tmp);
        let right = right.normalize();
        let up = Vector3::cross(&right, &forward);
        let up = up.normalize();
        let right = -right;

        let width = entity_scale.X * MODEL_WIDTH as f32;
        let height = entity_scale.Y * MODEL_HEIGHT as f32;

        let top_left = screen_pos - 0.5 * right * width + up * height;

        let diff = intersection_point - top_left;
        let x = diff.dot(&right) / width;
        let y = -(diff.dot(&up) / height);

        if x < 0.0 || x > 1.0 || y < 0.0 || y > 1.0 {
            return Err("not looking at a screen".into());
        }

        let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
        let (browser_width, browser_height) = Cef::get_browser_size(&browser);

        let (x, y) = (x * browser_width as f32, y * browser_height as f32);

        browser.send_click(x as _, y as _)?;
    }

    Ok(())
}
