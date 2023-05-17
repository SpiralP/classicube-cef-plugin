//! commands targetting a specific entity

use std::{
    os::raw::{c_float, c_int},
    time::{Duration, Instant},
};

use async_recursion::async_recursion;
use clap::Subcommand;
use classicube_helpers::color::{GOLD, SILVER, TEAL};

use super::helpers::{get_click_coords, move_entity};
use crate::{
    cef::Cef,
    chat::{Chat, PlayerSnapshot},
    entity_manager::{CefEntity, EntityManager, TargetEntity},
    error::{bail, ensure, Error, Result, ResultExt},
    helpers::format_duration,
    player::{PlayerBuilder, PlayerTrait, VolumeMode},
};
use classicube_helpers::async_manager;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Move the screen to you
    #[command(aliases(["move", "summon"]))]
    Here {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,
    },

    /// Play or queue something
    #[command(aliases(["play", "load"]))]
    Queue {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,

        /// Skip currently playing song and go to the next
        #[arg(long, short)]
        skip: bool,

        /// Start paused
        #[arg(long, short('a'))]
        no_autoplay: bool,

        /// Loop after track finishes playing
        #[arg(long, short)]
        r#loop: bool,

        /// Don't show Now Playing messages
        ///
        /// (may not be allowed on some urls)
        #[arg(long, short('q'), alias("quiet"))]
        silent: bool,

        // url has to be multiple because urls can be chopped in half by
        // line continuations, so we join the parts together as a hack
        #[arg(required(true), allow_hyphen_values(true))]
        url: Vec<String>,
    },

    /// Skip to the next video in the queue
    #[command(alias("next"))]
    Skip {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,
    },

    /// Stop playing
    Stop {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,
    },

    /// Remove screen
    #[command(aliases(["remove", "clear"]))]
    Close {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,
    },

    /// Scale screen
    Scale {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,

        #[arg(allow_hyphen_values(true))]
        scale: f32,
    },

    /// Resize screen
    #[command(alias("resize"))]
    Size {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,

        width: u16,

        height: u16,
    },

    /// Reload screen
    #[command(alias("refresh"))]
    Reload {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,
    },

    /// Change angles of screen
    #[command(alias("angle"))]
    Angles {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,

        #[arg(allow_hyphen_values(true))]
        yaw: f32,

        #[arg(allow_hyphen_values(true))]
        pitch: Option<f32>,
    },

    /// Click on screen
    ///
    /// If x, y are specified click at that position, otherwise click where you are aiming
    Click {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,

        #[arg(requires("y"))]
        x: Option<c_int>,

        #[arg(requires("x"))]
        y: Option<c_int>,
    },

    /// Type text on screen
    Type {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,

        #[arg(required(true), allow_hyphen_values(true))]
        words: Vec<String>,
    },

    /// Set the resolution of a screen
    Resolution {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,

        width: u16,

        height: u16,
    },

    /// Set audio volume of a screen
    ///
    /// If --global is specified, distance acts as volume
    Volume {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,

        /// Use global (not based on head angles) volume
        #[arg(long, short)]
        global: bool,

        /// Use panning/3D volume
        #[arg(long, short)]
        panning: bool,

        distance: f32,

        #[arg(conflicts_with("global"))]
        multiplier: Option<f32>,
    },

    /// Seek to time on a screen
    ///
    /// Also resumes if paused by default
    #[command(alias("seek"))]
    Time {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,

        /// Don't resume after setting time
        #[arg(long, short('a'))]
        no_autoplay: bool,

        time: String,
    },

    /// Move to coords x,y,z and optional yaw,pitch
    #[command(alias("tp"))]
    At {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,

        #[arg(allow_hyphen_values(true))]
        x: c_float,

        #[arg(allow_hyphen_values(true))]
        y: c_float,

        #[arg(allow_hyphen_values(true))]
        z: c_float,

        #[arg(allow_hyphen_values(true))]
        yaw: Option<f32>,

        #[arg(requires("yaw"), allow_hyphen_values(true))]
        pitch: Option<f32>,

        #[arg(requires("pitch"), allow_hyphen_values(true))]
        scale: Option<f32>,
    },

    /// Show what's playing
    Info {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,
    },

    /// Resume paused screen
    Resume {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,
    },

    /// Pause screen
    Pause {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,
    },

    /// Set playback rate of screen
    #[command(alias("rate"))]
    Speed {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,

        speed: f32,
    },

    /// Fade volume of screen
    #[command(override_usage("cef fade [OPTIONS] [FROM] <TO> <SECONDS>"))]
    Fade {
        /// Name of screen
        #[arg(long, short)]
        name: Option<String>,

        #[arg(name("FROM"))]
        from_or_to: f32,

        #[arg(name("TO"))]
        to_or_seconds: f32,

        #[arg(name("SECONDS"))]
        maybe_seconds: Option<f32>,
    },
}

#[async_recursion(?Send)]
pub async fn run(player: PlayerSnapshot, commands: Commands) -> Result<()> {
    match commands {
        Commands::Here { name } => {
            EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                |entity| {
                    move_entity(entity, &player);

                    Ok(())
                },
            )?;
        }

        Commands::Queue {
            name,
            skip,
            no_autoplay,
            r#loop,
            silent,
            url,
        } => {
            // hack so that newline continuation messages are concated
            let url = url.join("");

            let autoplay = !no_autoplay;
            let should_loop = r#loop;

            let mut players = PlayerBuilder::new()
                .autoplay(autoplay)
                .should_loop(should_loop)
                .silent(silent)
                .build(&url)
                .await?;

            for p in players.drain(..) {
                let kind = p.type_name();
                let url = p.get_url();
                EntityManager::with_entity(
                    name.as_ref().map_or_else(
                        || player.eye_position.get_entity_id(),
                        TargetEntity::get_entity_id,
                    )?,
                    |entity| {
                        if let Some(queue_size) = entity.queue(p)? {
                            Chat::print(format!(
                                "{TEAL}Queued {GOLD}{queue_size} {TEAL}{kind} {SILVER}{url}"
                            ));

                            if skip {
                                entity.skip()?;
                            }
                        }

                        Ok(())
                    },
                )?;
            }
        }

        Commands::Skip { name } => {
            EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                CefEntity::skip,
            )?;
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
        Commands::Stop { name } => {
            EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                CefEntity::stop,
            )?;
        }

        Commands::Close { name } => {
            let entity_id = EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                |entity| Ok(entity.id),
            )?;

            EntityManager::remove_entity(entity_id).await?;
        }

        Commands::Scale { name, scale } => {
            EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                move |entity| {
                    entity.set_scale(scale);

                    Ok(())
                },
            )?;
        }

        Commands::Size {
            name,
            width,
            height,
        } => {
            EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                move |entity| {
                    entity.set_size(width, height);

                    Ok(())
                },
            )?;
        }

        Commands::Reload { name } => {
            let entity_id = EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                |entity| Ok(entity.id),
            )?;
            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            browser.reload()?;
        }

        Commands::Angles { name, yaw, pitch } => {
            EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                |entity| {
                    entity.entity.RotY = yaw;

                    if let Some(pitch) = pitch {
                        entity.entity.RotX = pitch;
                    }

                    Ok(())
                },
            )?;
        }

        Commands::Click { name, x, y } => {
            if let Some(x) = x {
                if let Some(y) = y {
                    let entity_id = EntityManager::with_entity(
                        name.map_or_else(
                            || player.eye_position.get_entity_id(),
                            |name| name.get_entity_id(),
                        )?,
                        |entity| Ok(entity.id),
                    )?;

                    let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
                    browser.send_click(x, y)?;
                }
            } else {
                let (entity_id, entity_pos, [entity_pitch, entity_yaw], entity_scale, entity_size) =
                    EntityManager::with_entity(
                        name.map_or_else(
                            || player.eye_position.get_entity_id(),
                            |name| name.get_entity_id(),
                        )?,
                        |entity| {
                            Ok((
                                entity.id,
                                entity.entity.Position,
                                [entity.entity.RotX, entity.entity.RotY],
                                entity.entity.ModelScale,
                                entity.get_size(),
                            ))
                        },
                    )?;

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
        }

        Commands::Type { name, words } => {
            let text = words.join(" ");

            let entity_id = EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                |entity| Ok(entity.id),
            )?;

            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            browser.send_text(text)?;
        }

        Commands::Resolution {
            name,
            width,
            height,
        } => {
            let entity_id = EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                |entity| Ok(entity.id),
            )?;

            let browser = EntityManager::get_browser_by_entity_id(entity_id)?;
            Cef::resize_browser(&browser, width, height)?;
        }

        Commands::Volume {
            name,
            global,
            panning,
            distance,
            multiplier,
        } => {
            EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                |entity| {
                    if global {
                        entity
                            .player
                            .set_volume(entity.browser.as_ref(), distance)?;
                        entity
                            .player
                            .set_volume_mode(entity.browser.as_ref(), VolumeMode::Global)?;
                    } else {
                        let multiplier = if let Some(volume) = multiplier {
                            volume
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
                },
            )?;
        }

        Commands::Time {
            name,
            no_autoplay,
            time,
        } => {
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

            EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                |entity| {
                    let browser = entity.browser.as_ref().chain_err(|| "no browser")?;

                    entity
                        .player
                        .set_current_time(browser, Duration::from_secs_f32(seconds))?;

                    if !no_autoplay {
                        entity.player.set_playing(browser, true)?;
                    }

                    Ok(())
                },
            )?;
        }

        Commands::At {
            name,
            x,
            y,
            z,
            yaw,
            pitch,
            scale,
        } => {
            if EntityManager::with_entity(
                name.as_ref().map_or_else(
                    || player.eye_position.get_entity_id(),
                    TargetEntity::get_entity_id,
                )?,
                |_| Ok(()),
            )
            .is_err()
            {
                let mut args = vec!["create".to_string(), "--no-wait".to_string()];

                if let Some(name) = &name {
                    args.push("--name".to_string());
                    args.push(name.to_string());
                }

                super::run(player.clone(), args, true, true).await?;
            }

            EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                |entity| {
                    entity.entity.Position.set(x, y, z);

                    if let Some(yaw) = yaw {
                        entity.entity.RotY = yaw;

                        if let Some(pitch) = pitch {
                            entity.entity.RotX = pitch;

                            if let Some(scale) = scale {
                                entity.set_scale(scale);
                            }
                        }
                    }

                    Ok(())
                },
            )?;
        }

        Commands::Info { name: _ } => {
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
        }

        Commands::Resume { name } => {
            EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                |entity| {
                    let browser = entity.browser.as_ref().chain_err(|| "no browser")?;

                    entity.player.set_playing(browser, true)?;
                    Ok(())
                },
            )?;
        }
        Commands::Pause { name } => {
            EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                |entity| {
                    let browser = entity.browser.as_ref().chain_err(|| "no browser")?;

                    entity.player.set_playing(browser, false)?;
                    Ok(())
                },
            )?;
        }
        Commands::Speed { name, speed } => {
            EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                |entity| {
                    entity.player.set_speed(entity.browser.as_ref(), speed)?;
                    Ok(())
                },
            )?;
        }

        Commands::Fade {
            name,
            from_or_to,
            to_or_seconds,
            maybe_seconds,
        } => {
            fn check_f32(name: &str, n: f32) -> Result<f32> {
                ensure!(n.is_finite(), "{} not finite", name);
                ensure!(n.is_sign_positive(), "{} not positive", name);
                Ok(n)
            }

            let (maybe_from, to, seconds) = if let Some(maybe_seconds) = maybe_seconds {
                // fade from to seconds
                let seconds: f32 = check_f32("seconds", maybe_seconds)?;

                let from: f32 = check_f32("from", from_or_to)?;
                let to: f32 = check_f32("to", to_or_seconds)?;
                (Some(from), to, seconds)
            } else {
                // fade to seconds
                let to: f32 = check_f32("to", from_or_to)?;
                let seconds: f32 = check_f32("seconds", to_or_seconds)?;
                (None, to, seconds)
            };

            let entity_id = EntityManager::with_entity(
                name.map_or_else(
                    || player.eye_position.get_entity_id(),
                    |name| name.get_entity_id(),
                )?,
                move |entity| Ok(entity.id),
            )?;

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
        }
    }

    Ok(())
}
