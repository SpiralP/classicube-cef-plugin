use std::time::{Duration, Instant};

use async_recursion::async_recursion;
use clap::{App, AppSettings, Arg, ArgMatches};
use classicube_helpers::color::{GOLD, SILVER, TEAL};

use super::helpers::{get_click_coords, move_entity};
use crate::{
    cef::Cef,
    chat::{Chat, PlayerSnapshot},
    entity_manager::{CefEntity, EntityManager},
    error::{bail, ensure, Error, Result, ResultExt},
    helpers::format_duration,
    player::{PlayerBuilder, PlayerTrait, VolumeMode},
};
use classicube_helpers::async_manager;

// static commands not targetted at a specific entity
pub fn add_commands(app: App<'static, 'static>) -> App<'static, 'static> {
    app.subcommand(
        App::new("here")
            .alias("move")
            .alias("summon")
            .about("Move to you")
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            ),
    )
    .subcommand(
        App::new("queue")
            .alias("play")
            .alias("load")
            .about("Play or queue something")
            .setting(AppSettings::AllowLeadingHyphen)
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("skip")
                    .short("s")
                    .long("skip")
                    .help("Skip currently playing song and go to the next"),
            )
            .arg(
                Arg::with_name("no-autoplay")
                    .long("no-autoplay")
                    .short("a")
                    .help("Don't resume after setting time"),
            )
            .arg(
                Arg::with_name("loop")
                    .long("loop")
                    .short("l")
                    .help("Loop after track finishes playing"),
            )
            .arg(
                Arg::with_name("silent")
                    .long("silent")
                    .alias("quiet")
                    .short("q")
                    .help("Don't show Now Playing messages"),
            )
            .arg(Arg::with_name("url").required(true).multiple(true)),
    )
    .subcommand(
        App::new("skip")
            .alias("next")
            .about("Skip to the next video in the queue")
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            ),
    )
    .subcommand(
        App::new("stop").about("Stop playing").arg(
            Arg::with_name("name")
                .long("name")
                .short("n")
                .takes_value(true),
        ),
    )
    .subcommand(
        App::new("close")
            .aliases(&["remove", "clear"])
            .about("Remove")
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            ),
    )
    .subcommand(
        App::new("scale")
            .about("Scale")
            .setting(AppSettings::AllowLeadingHyphen)
            .arg(Arg::with_name("scale").required(true))
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            ),
    )
    .subcommand(
        App::new("size")
            .alias("resize")
            .about("Resizes")
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            )
            .arg(Arg::with_name("width").required(true))
            .arg(Arg::with_name("height").required(true)),
    )
    .subcommand(
        App::new("reload").alias("refresh").about("Reload").arg(
            Arg::with_name("name")
                .long("name")
                .short("n")
                .takes_value(true),
        ),
    )
    .subcommand(
        App::new("angles")
            .alias("angle")
            .about("Change angles")
            .setting(AppSettings::AllowLeadingHyphen)
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            )
            .arg(Arg::with_name("yaw").required(true))
            .arg(Arg::with_name("pitch")),
    )
    .subcommand(
        App::new("click")
            .about("Click")
            .long_about(
                "If x, y are specified click at that position, otherwise click where you are \
                 aiming",
            )
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            )
            .arg(Arg::with_name("x").requires("y"))
            .arg(Arg::with_name("y").requires("x")),
    )
    .subcommand(
        App::new("type")
            .about("Type text")
            .setting(AppSettings::AllowLeadingHyphen)
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            )
            .arg(Arg::with_name("words").required(true).multiple(true)),
    )
    .subcommand(
        App::new("resolution")
            .about("Set the resolution")
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            )
            .arg(Arg::with_name("width").required(true))
            .arg(Arg::with_name("height").required(true)),
    )
    .subcommand(
        App::new("volume")
            .about("Set volume")
            .long_about("If --global is specified, distance acts as volume.")
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("global")
                    .long("global")
                    .short("g")
                    .help("Use global (consistant) volume"),
            )
            .arg(
                Arg::with_name("panning")
                    .long("panning")
                    .short("p")
                    .help("Use panning/3D volume"),
            )
            .arg(Arg::with_name("distance").required(true))
            .arg(Arg::with_name("multiplier").conflicts_with("global")),
    )
    .subcommand(
        App::new("time")
            .alias("seek")
            .about("Seek time")
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("no-autoplay")
                    .long("no-autoplay")
                    .short("a")
                    .help("Don't resume after setting time"),
            )
            .arg(Arg::with_name("time").required(true)),
    )
    .subcommand(
        App::new("at")
            .alias("tp")
            .usage("cef at <x> <y> <z> [yaw] [pitch] [scale]")
            .about("Move to coords x,y,z and optional yaw,pitch")
            .setting(AppSettings::AllowLeadingHyphen)
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            )
            .arg(Arg::with_name("x").required(true))
            .arg(Arg::with_name("y").required(true))
            .arg(Arg::with_name("z").required(true))
            .arg(Arg::with_name("yaw"))
            .arg(Arg::with_name("pitch").requires("yaw"))
            .arg(Arg::with_name("scale").requires("pitch")),
    )
    .subcommand(
        App::new("info").about("Get what's playing").arg(
            Arg::with_name("name")
                .long("name")
                .short("n")
                .takes_value(true),
        ),
    )
    .subcommand(
        App::new("resume").about("Resume paused video").arg(
            Arg::with_name("name")
                .long("name")
                .short("n")
                .takes_value(true),
        ),
    )
    .subcommand(
        App::new("pause").about("Pause video").arg(
            Arg::with_name("name")
                .long("name")
                .short("n")
                .takes_value(true),
        ),
    )
    .subcommand(
        App::new("speed")
            .alias("rate")
            .about("Set playback rate")
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            )
            .arg(Arg::with_name("speed").required(true)),
    )
    .subcommand(
        App::new("fade")
            .usage("cef fade [from] <to> <seconds>")
            .about("Fade volume")
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            )
            .arg(Arg::with_name("from_or_to").required(true))
            .arg(Arg::with_name("to_or_seconds").required(true))
            .arg(Arg::with_name("maybe_seconds").required(false)),
    )
}

#[async_recursion(?Send)]
pub async fn handle_command(
    player: &PlayerSnapshot,
    matches: &ArgMatches<'static>,
) -> Result<bool> {
    match matches.subcommand() {
        ("here", Some(matches)) => {
            EntityManager::with_entity((matches, player), |entity| {
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
            let autoplay = !matches.is_present("no-autoplay");
            let should_loop = matches.is_present("loop");
            let silent = matches.is_present("silent");

            let mut players = PlayerBuilder::new()
                .autoplay(autoplay)
                .should_loop(should_loop)
                .silent(silent)
                .build(&url)
                .await?;

            for p in players.drain(..) {
                let kind = p.type_name();
                let url = p.get_url();
                EntityManager::with_entity((matches, player), |entity| {
                    if let Some(queue_size) = entity.queue(p)? {
                        Chat::print(format!(
                            "{TEAL}Queued {GOLD}{queue_size} {TEAL}{kind} {SILVER}{url}"
                        ));

                        if skip {
                            entity.skip()?;
                        }
                    }

                    Ok(())
                })?;
            }

            Ok(true)
        }

        ("skip", Some(matches)) => {
            EntityManager::with_entity((matches, player), CefEntity::skip)?;

            Ok(true)
        }

        // ("test_time", Some(matches)) => {
        // let future_dt = DateTime::parse_from_rfc3339(matches.value_of("hack").unwrap())?;
        //
        // async_manager::spawn_blocking(move || {
        // let NtpResult {
        // sec, nsec, offset, ..
        // } = sntpc::request("time.google.com", 123)?;
        // let dt: DateTime<FixedOffset> =
        // DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(sec.into(), nsec), Utc)
        // .into();
        //
        // debug!("ntp {}", dt);
        // debug!(
        // "offset {:?}",
        // std::time::Duration::from_micros(offset as u64)
        // );
        //
        // async_manager::spawn_on_main_thread(async move {
        // Chat::print(format!("{:?}", (future_dt - dt).to_std()));
        //
        // let a = (future_dt - dt).to_std();
        // let a = a.map(|a| format!("{:?}", a)).unwrap_or_else(|e| {
        // warn!("{:#?}", e);
        // "??".to_string()
        // });
        // Chat::send(format!("@SpiralP+ {}", a));
        // });
        //
        // Ok::<_, Error>(())
        // })
        // .await???;
        //
        // bail!("unimplemented");
        // }
        ("stop", Some(matches)) => {
            EntityManager::with_entity((matches, player), CefEntity::stop)?;

            Ok(true)
        }

        ("close", Some(matches)) => {
            let entity_id = EntityManager::with_entity((matches, player), |entity| Ok(entity.id))?;

            EntityManager::remove_entity(entity_id).await?;

            Ok(true)
        }

        ("scale", Some(matches)) => {
            let scale = matches.value_of("scale").unwrap().parse()?;

            EntityManager::with_entity((matches, player), move |entity| {
                entity.set_scale(scale);

                Ok(())
            })?;

            Ok(true)
        }

        ("size", Some(matches)) => {
            let width = matches.value_of("width").unwrap().parse()?;
            let height = matches.value_of("height").unwrap().parse()?;

            EntityManager::with_entity((matches, player), move |entity| {
                entity.set_size(width, height);

                Ok(())
            })?;

            Ok(true)
        }

        ("reload", Some(matches)) => {
            let entity_id = EntityManager::with_entity((matches, player), |entity| Ok(entity.id))?;
            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            browser.reload()?;

            Ok(true)
        }

        ("angles", Some(matches)) => {
            EntityManager::with_entity((matches, player), |entity| {
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
                        EntityManager::with_entity((matches, player), |entity| Ok(entity.id))?;

                    let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
                    browser.send_click(x, y)?;
                }
            } else {
                let (entity_id, entity_pos, [entity_pitch, entity_yaw], entity_scale, entity_size) =
                    EntityManager::with_entity((matches, player), |entity| {
                        Ok((
                            entity.id,
                            entity.entity.Position,
                            [entity.entity.RotX, entity.entity.RotY],
                            entity.entity.ModelScale,
                            entity.get_size(),
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
                    entity_size,
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

            let entity_id = EntityManager::with_entity((matches, player), |entity| Ok(entity.id))?;

            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            browser.send_text(text)?;

            Ok(true)
        }

        ("resolution", Some(matches)) => {
            let width = matches.value_of("width").unwrap().parse()?;
            let height = matches.value_of("height").unwrap().parse()?;

            let entity_id = EntityManager::with_entity((matches, player), |entity| Ok(entity.id))?;

            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            Cef::resize_browser(&browser, width, height)?;

            Ok(true)
        }

        ("volume", Some(matches)) => {
            let global = matches.is_present("global");
            let panning = matches.is_present("panning");

            let distance = matches.value_of("distance").unwrap().parse()?;

            EntityManager::with_entity((matches, player), |entity| {
                if global {
                    entity
                        .player
                        .set_volume(entity.browser.as_ref(), distance)?;
                    entity
                        .player
                        .set_volume_mode(entity.browser.as_ref(), VolumeMode::Global)?;
                } else {
                    let multiplier = if let Some(volume) = matches.value_of("multiplier") {
                        volume.parse()?
                    } else {
                        1.0
                    };

                    if panning {
                        entity.player.set_volume_mode(
                            entity.browser.as_ref(),
                            VolumeMode::Panning {
                                multiplier,
                                distance,
                                pan: 0.0,
                            },
                        )?;
                    } else {
                        entity.player.set_volume_mode(
                            entity.browser.as_ref(),
                            VolumeMode::Distance {
                                multiplier,
                                distance,
                            },
                        )?;
                    }
                }
                Ok(())
            })?;

            Ok(true)
        }

        ("time", Some(matches)) => {
            let time = matches.value_of("time").unwrap();

            let seconds: f32 = if let Ok(seconds) = time.parse() {
                seconds
            } else {
                // try 12:34 mm:ss format

                let parts: Vec<_> = time.split(':').collect();
                match parts.as_slice() {
                    [hours, minutes, seconds] => {
                        let hours: f32 = hours.parse()?;
                        let minutes: f32 = minutes.parse()?;
                        let seconds: f32 = seconds.parse()?;

                        seconds + minutes * 60.0 + hours * 60.0 * 60.0
                    }

                    [minutes, seconds] => {
                        let minutes: f32 = minutes.parse()?;
                        let seconds: f32 = seconds.parse()?;

                        seconds + minutes * 60.0
                    }

                    _ => {
                        // let parts:Vec<_> = time.split("%").collect();
                        // TODO 20%

                        bail!("bad format");
                    }
                }
            };

            ensure!(seconds.is_finite(), "not finite");
            ensure!(seconds.is_sign_positive(), "not positive");

            EntityManager::with_entity((matches, player), |entity| {
                let browser = entity.browser.as_ref().chain_err(|| "no browser")?;

                entity
                    .player
                    .set_current_time(browser, Duration::from_secs_f32(seconds))?;

                if !matches.is_present("no-autoplay") {
                    entity.player.set_playing(browser, true)?;
                }

                Ok(())
            })?;

            Ok(true)
        }

        ("at", Some(matches)) => {
            let x = matches.value_of("x").unwrap().parse()?;
            let y = matches.value_of("y").unwrap().parse()?;
            let z = matches.value_of("z").unwrap().parse()?;

            if EntityManager::with_entity((matches, player), |_| Ok(())).is_err() {
                let mut args = vec!["create".to_string(), "--no-wait".to_string()];

                if let Some(name) = matches.value_of("name") {
                    args.push("--name".to_string());
                    args.push(name.to_string());
                }

                super::run(player.clone(), args, true, true).await?;
            }

            EntityManager::with_entity((matches, player), |entity| {
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
            EntityManager::with_all_entities(|entities| {
                for entity in entities.values() {
                    let url = entity.player.get_url();
                    let title = entity.player.get_title();

                    if !title.is_empty() {
                        Chat::print(format!("{TEAL}Playing {SILVER}{title}"));
                    }

                    if let Ok(time) = entity.player.get_current_time() {
                        let time = format_duration(time);
                        Chat::print(format!("At time {time}"));
                    }

                    Chat::print(url);

                    if !entity.queue.is_empty() {
                        let len = entity.queue.len();
                        Chat::print(format!("{GOLD}{len} {TEAL}items in queue:"));

                        for (i, (player, title)) in entity.queue.iter().enumerate() {
                            let url = player.get_url();

                            let index = i + 1;
                            let type_name = player.type_name();
                            Chat::print(format!("{GOLD}{index} {TEAL}{type_name} {SILVER}{url}"));

                            let title = title.lock().unwrap();
                            if let Some(title) = &*title {
                                Chat::print(format!("{SILVER}{title}"));
                            }
                        }
                    }
                }

                Ok::<_, Error>(())
            })?;

            Ok(true)
        }

        ("resume", Some(matches)) => {
            EntityManager::with_entity((matches, player), |entity| {
                let browser = entity.browser.as_ref().chain_err(|| "no browser")?;

                entity.player.set_playing(browser, true)?;
                Ok(())
            })?;

            Ok(true)
        }

        ("pause", Some(matches)) => {
            EntityManager::with_entity((matches, player), |entity| {
                let browser = entity.browser.as_ref().chain_err(|| "no browser")?;

                entity.player.set_playing(browser, false)?;
                Ok(())
            })?;

            Ok(true)
        }

        ("speed", Some(matches)) => {
            let speed = matches.value_of("speed").unwrap().parse()?;
            EntityManager::with_entity((matches, player), |entity| {
                entity.player.set_speed(entity.browser.as_ref(), speed)?;
                Ok(())
            })?;

            Ok(true)
        }

        ("fade", Some(matches)) => {
            fn check_f32(name: &str, n: f32) -> Result<f32> {
                ensure!(n.is_finite(), "{} not finite", name);
                ensure!(n.is_sign_positive(), "{} not positive", name);
                Ok(n)
            }

            let (maybe_from, to, seconds) = if let Some(maybe_seconds) =
                matches.value_of("maybe_seconds")
            {
                // fade from to seconds
                let seconds: f32 = check_f32("seconds", maybe_seconds.parse()?)?;

                let from: f32 =
                    check_f32("from", matches.value_of("from_or_to").unwrap().parse()?)?;
                let to: f32 = check_f32("to", matches.value_of("to_or_seconds").unwrap().parse()?)?;
                (Some(from), to, seconds)
            } else {
                // fade to seconds
                let to: f32 = check_f32("to", matches.value_of("from_or_to").unwrap().parse()?)?;
                let seconds: f32 = check_f32(
                    "seconds",
                    matches.value_of("to_or_seconds").unwrap().parse()?,
                )?;
                (None, to, seconds)
            };

            let entity_id =
                EntityManager::with_entity((matches, player), move |entity| Ok(entity.id))?;

            let from = if let Some(from) = maybe_from {
                from
            } else {
                EntityManager::with_entity(entity_id, move |entity| {
                    Ok(match entity.player.get_volume_mode() {
                        VolumeMode::Global => entity.player.get_volume(),
                        VolumeMode::Distance { multiplier, .. }
                        | VolumeMode::Panning { multiplier, .. } => multiplier,
                    })
                })?
            };

            let set_volume = move |volume: f32| {
                EntityManager::with_entity(entity_id, move |entity| {
                    match entity.player.get_volume_mode() {
                        VolumeMode::Global => {
                            entity.player.set_volume(entity.browser.as_ref(), volume)
                        }
                        VolumeMode::Distance {
                            multiplier: _,
                            distance,
                        } => entity.player.set_volume_mode(
                            entity.browser.as_ref(),
                            VolumeMode::Distance {
                                multiplier: volume,
                                distance,
                            },
                        ),
                        VolumeMode::Panning {
                            multiplier: _,
                            distance,
                            pan,
                        } => entity.player.set_volume_mode(
                            entity.browser.as_ref(),
                            VolumeMode::Panning {
                                multiplier: volume,
                                distance,
                                pan,
                            },
                        ),
                    }
                })
            };

            set_volume(from)?;

            let start_time = Instant::now();
            loop {
                let now = Instant::now();
                let secs_from_start = (now - start_time).as_secs_f32();

                let percent = secs_from_start / seconds;
                if percent > 1.0 {
                    break;
                }

                let volume = from + (to - from) * percent;
                set_volume(volume)?;

                async_manager::sleep(Duration::from_millis(32)).await;
            }

            set_volume(to)?;

            Ok(true)
        }

        _ => Ok(false),
    }
}
