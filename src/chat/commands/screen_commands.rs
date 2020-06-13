use super::helpers::*;
use crate::{
    async_manager,
    cef::Cef,
    chat::{Chat, PlayerSnapshot},
    entity_manager::EntityManager,
    error::*,
    helpers::format_duration,
    players::PlayerTrait,
};
use async_recursion::async_recursion;
use chrono::{offset::FixedOffset, DateTime, NaiveDateTime, Utc};
use clap::{App, AppSettings, Arg, ArgMatches};
use classicube_helpers::color;
use log::*;
use sntpc::NtpResult;
use std::time::Duration;

// static commands not targetted at a specific entity
pub fn add_commands(app: App<'static, 'static>) -> App<'static, 'static> {
    app.subcommand(
        App::new("here")
            .alias("move")
            .alias("summon")
            .about("Move the closest screen to you"),
    )
    .subcommand(
        App::new("queue")
            .alias("play")
            .alias("load")
            .about("Play or queue something on the closest screen")
            .setting(AppSettings::AllowLeadingHyphen)
            .arg(
                Arg::with_name("skip")
                    .short("s")
                    .long("skip")
                    .help("Skip currently playing song and go to the next"),
            )
            .arg(Arg::with_name("url").required(true).multiple(true)),
    )
    .subcommand(
        App::new("skip")
            .alias("next")
            .about("Skip to the next video in the queue of the closest screen"),
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
            .setting(AppSettings::AllowLeadingHyphen)
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
            .setting(AppSettings::AllowLeadingHyphen)
            .arg(Arg::with_name("yaw").required(true))
            .arg(Arg::with_name("pitch")),
    )
    .subcommand(
        App::new("click")
            .about("Click the closest screen")
            .long_about(
                "If x, y are specified click at that position, otherwise click where you are \
                 aiming",
            )
            .arg(Arg::with_name("x").requires("y"))
            .arg(Arg::with_name("y").requires("x")),
    )
    .subcommand(
        App::new("type")
            .about("Type text on the closest screen")
            .setting(AppSettings::AllowLeadingHyphen)
            .arg(Arg::with_name("words").required(true).multiple(true)),
    )
    .subcommand(
        App::new("resolution")
            .about("Set the resolution of the closest screen")
            .arg(Arg::with_name("width").required(true))
            .arg(Arg::with_name("height").required(true)),
    )
    .subcommand(
        App::new("volume")
            .about("Set volume of the closest screen")
            .arg(
                Arg::with_name("global")
                    .long("global")
                    .short("g")
                    .help("Use global volume, don't use distance for volume"),
            )
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
            .usage("cef at <x> <y> <z> [yaw] [pitch] [scale]")
            .alias("tp")
            .about("Move the closest screen to coords x,y,z and optional yaw,pitch")
            .setting(AppSettings::AllowLeadingHyphen)
            .arg(Arg::with_name("x").required(true))
            .arg(Arg::with_name("y").required(true))
            .arg(Arg::with_name("z").required(true))
            .arg(Arg::with_name("yaw"))
            .arg(Arg::with_name("pitch").requires("yaw"))
            .arg(Arg::with_name("scale").requires("pitch")),
    )
    .subcommand(
        App::new("info")
            .alias("link")
            .about("Get what's playing on the current screen"),
    )
    .subcommand(
        App::new("test_time")
            .about("the hacks")
            .setting(AppSettings::AllowLeadingHyphen)
            .arg(Arg::with_name("hack").required(true)),
    )
}

#[async_recursion(?Send)]
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

        ("queue", Some(matches)) => {
            // hack so that newline continuation messages are concated
            let parts = matches.values_of_lossy("url").unwrap_or_default();
            let url: String = parts.join("");

            let skip = matches.is_present("skip");

            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;

            if skip {
                EntityManager::entity_skip(entity_id)?;
            }

            if let Some(kind) = EntityManager::entity_queue(&url, entity_id)? {
                Chat::print(format!(
                    "{}Queued {}{} {}",
                    color::TEAL,
                    kind,
                    color::SILVER,
                    url
                ));
            }

            Ok(true)
        }

        ("skip", Some(_matches)) => {
            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;
            EntityManager::entity_skip(entity_id)?;

            Ok(true)
        }

        ("test_time", Some(matches)) => {
            let future_dt = DateTime::parse_from_rfc3339(matches.value_of("hack").unwrap())?;

            async_manager::spawn_blocking(move || {
                let NtpResult {
                    sec, nsec, offset, ..
                } = sntpc::request("time.google.com", 123)?;
                let dt: DateTime<FixedOffset> =
                    DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(sec.into(), nsec), Utc)
                        .into();

                debug!("ntp {}", dt);
                debug!(
                    "offset {:?}",
                    std::time::Duration::from_micros(offset as u64)
                );

                async_manager::spawn_on_main_thread(async move {
                    Chat::print(format!("{:?}", (future_dt - dt).to_std()));

                    let a = (future_dt - dt).to_std();
                    let a = a.map(|a| format!("{:?}", a)).unwrap_or_else(|e| {
                        warn!("{:#?}", e);
                        "??".to_string()
                    });
                    Chat::send(format!("@SpiralP+ {}", a));
                });

                Ok::<_, Error>(())
            })
            .await???;

            bail!("unimplemented");
        }

        ("stop", Some(_matches)) => {
            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;

            EntityManager::entity_stop(entity_id)?;

            Ok(true)
        }

        ("close", Some(_matches)) => {
            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;

            async_manager::spawn_local_on_main_thread(async move {
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
                let (entity_id, entity_pos, [entity_pitch, entity_yaw], entity_scale) =
                    EntityManager::with_closest(player.eye_position, |closest_entity| {
                        Ok((
                            closest_entity.id,
                            closest_entity.entity.Position,
                            [closest_entity.entity.RotX, closest_entity.entity.RotY],
                            closest_entity.entity.ModelScale,
                        ))
                    })?;

                let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
                let (browser_width, browser_height) = Cef::get_browser_size(&browser);

                if let Some((x, y)) = get_click_coords(
                    player.eye_position,
                    entity_pos,
                    player.Pitch,
                    player.Yaw,
                    entity_pitch,
                    entity_yaw,
                    entity_scale,
                    browser_width as u32,
                    browser_height as u32,
                )? {
                    browser.send_click(x as _, y as _)?;
                }
            }

            Ok(true)
        }

        ("type", Some(matches)) => {
            let parts = matches.values_of_lossy("words").unwrap_or_default();
            let text: String = parts.join(" ");
            let text = (*text).to_string();

            let entity_id = EntityManager::with_closest(player.eye_position, |closest_entity| {
                Ok(closest_entity.id)
            })?;

            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            browser.send_text(text)?;

            Ok(true)
        }

        ("resolution", Some(matches)) => {
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
            if EntityManager::with_closest(player.eye_position, |_| Ok(())).is_err() {
                super::run(player.clone(), vec!["create".to_string()], false).await?;
            }

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

                        if let Some(scale) = matches.value_of("scale") {
                            let scale = scale.parse()?;
                            entity.set_scale(scale);
                        }
                    }
                }

                Ok(())
            })?;

            Ok(true)
        }

        ("info", Some(_matches)) => {
            // let's have it print for everyone
            EntityManager::with_closest(player.eye_position, |entity| {
                let url = entity.player.get_url();
                let title = entity.player.get_title();

                if !title.is_empty() {
                    Chat::print(format!(
                        "{}Playing {}{} ",
                        color::TEAL,
                        color::SILVER,
                        title
                    ));
                }

                if let Ok(time) = entity.player.get_current_time() {
                    Chat::print(format!("At time {}", format_duration(time)));
                }

                Chat::print(url);

                if !entity.queue.is_empty() {
                    Chat::print(format!(
                        "{}{} {}items in queue:",
                        color::GOLD,
                        entity.queue.len(),
                        color::TEAL,
                    ));

                    for (i, player) in entity.queue.iter().enumerate() {
                        let url = player.get_url();

                        Chat::print(format!(
                            "{}{} {}{} {}{}",
                            color::GOLD,
                            i + 1,
                            color::TEAL,
                            player.type_name(),
                            color::SILVER,
                            url
                        ));
                    }
                }

                Ok(())
            })?;

            Ok(true)
        }

        _ => Ok(false),
    }
}
