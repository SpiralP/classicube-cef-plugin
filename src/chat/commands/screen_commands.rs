use super::helpers::*;
use crate::{
    async_manager,
    cef::Cef,
    chat::{Chat, PlayerSnapshot},
    entity_manager::EntityManager,
    error::*,
    helpers::format_duration,
    player::{PlayerBuilder, PlayerTrait, VolumeMode},
};
use async_recursion::async_recursion;
use clap::{App, AppSettings, Arg, ArgMatches};
use classicube_helpers::color;
use log::*;
use std::time::{Duration, Instant};

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
    .subcommand(App::new("stop").about("Stop playing"))
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
    .subcommand(App::new("reload").alias("refresh").about("Reload"))
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
            .usage("cef at <x> <y> <z> [yaw] [pitch] [scale]")
            .alias("tp")
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
        App::new("info")
            .alias("link")
            .about("Get what's playing on the current screen")
            .arg(
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
            .about("Fade volume")
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true),
            )
            .arg(Arg::with_name("from").required(true))
            .arg(Arg::with_name("to").required(true))
            .arg(Arg::with_name("seconds").required(true)),
    )
    // .subcommand(
    //     App::new("test_time")
    //         .about("the hacks")
    //         .setting(AppSettings::AllowLeadingHyphen)
    //         .arg(Arg::with_name("hack").required(true)),
    // )
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

            let p = PlayerBuilder::new()
                .autoplay(autoplay)
                .should_loop(should_loop)
                .build(&url)?;

            EntityManager::with_entity((matches, player), move |entity| {
                if let Some(kind) = entity.queue(p)? {
                    Chat::print(format!(
                        "{}Queued {}{} {}",
                        color::TEAL,
                        kind,
                        color::SILVER,
                        url
                    ));

                    if skip {
                        entity.skip()?;
                    }
                }

                Ok(())
            })?;

            Ok(true)
        }

        ("skip", Some(matches)) => {
            EntityManager::with_entity((matches, player), |entity| entity.skip())?;

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
            EntityManager::with_entity((matches, player), |entity| entity.stop())?;

            Ok(true)
        }

        ("close", Some(matches)) => {
            let entity_id = EntityManager::with_entity((matches, player), |entity| Ok(entity.id))?;

            async_manager::spawn_local_on_main_thread(async move {
                if let Err(e) = EntityManager::remove_entity(entity_id).await {
                    warn!("{}", e);
                }
            });

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
                    .set_current_time(&browser, Duration::from_secs_f32(seconds))?;

                if !matches.is_present("no-autoplay") {
                    entity.player.set_playing(&browser, true)?;
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
                }

                Ok::<_, Error>(())
            })?;

            Ok(true)
        }

        ("resume", Some(matches)) => {
            EntityManager::with_entity((matches, player), |entity| {
                let browser = entity.browser.as_ref().chain_err(|| "no browser")?;

                entity.player.set_playing(&browser, true)?;
                Ok(())
            })?;

            Ok(true)
        }

        ("pause", Some(matches)) => {
            EntityManager::with_entity((matches, player), |entity| {
                let browser = entity.browser.as_ref().chain_err(|| "no browser")?;

                entity.player.set_playing(&browser, false)?;
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
            let from: f32 = matches.value_of("from").unwrap().parse()?;
            ensure!(from.is_finite(), "from not finite");
            ensure!(from.is_sign_positive(), "from not positive");

            let to: f32 = matches.value_of("to").unwrap().parse()?;
            ensure!(to.is_finite(), "to not finite");
            ensure!(to.is_sign_positive(), "to not positive");

            let seconds: f32 = matches.value_of("seconds").unwrap().parse()?;
            ensure!(seconds.is_finite(), "seconds not finite");
            ensure!(seconds.is_sign_positive(), "seconds not positive");

            let entity_id =
                EntityManager::with_entity((matches, player), move |entity| Ok(entity.id))?;

            EntityManager::with_entity(entity_id, move |entity| {
                entity.player.set_volume(entity.browser.as_ref(), from)?;
                entity
                    .player
                    .set_volume_mode(entity.browser.as_ref(), VolumeMode::Global)
            })?;

            let start_time = Instant::now();
            loop {
                let now = Instant::now();
                let secs_from_start = (now - start_time).as_secs_f32();

                let percent = secs_from_start / seconds;
                if percent > 1.0 {
                    break;
                }

                let volume = from + (to - from) * percent;
                EntityManager::with_entity(entity_id, move |entity| {
                    entity.player.set_volume(entity.browser.as_ref(), volume)
                })?;

                async_manager::sleep(Duration::from_millis(32)).await;
            }

            EntityManager::with_entity(entity_id, move |entity| {
                entity.player.set_volume(entity.browser.as_ref(), to)
            })?;

            Ok(true)
        }

        _ => Ok(false),
    }
}
