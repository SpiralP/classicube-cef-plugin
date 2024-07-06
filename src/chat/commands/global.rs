//! global commands not targetted at a specific entity

use clap::Subcommand;
use classicube_sys::{Chat_Send, OwnedString};

use super::helpers::move_entity;
use crate::{
    chat::PlayerSnapshot,
    entity_manager::{EntityBuilder, EntityManager, TargetEntity},
    error::{bail, Result},
    player::url_aliases,
    player::{Player, PlayerBuilder, VolumeMode},
};
use classicube_helpers::async_manager;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Creates a URL alias
    ///
    /// Allows using short-hand strings like "yt:LXb3EKWsInQ" in
    /// "cef create" or "cef play".
    ///
    /// For example, "cef alias yt https://youtu.be/" makes the above possible.
    ///
    /// All aliases are cleared on map change.
    #[command(name("alias"), alias("urlalias"), alias("url-alias"))]
    Alias { alias: String, url: String },

    /// Creates a new screen
    ///
    /// This will wait for page load unless --no-wait is specified
    Create {
        /// Name the screen
        #[arg(long, short)]
        name: Option<String>,

        /// Allow insecure https connections
        #[arg(long, short, hide(true))]
        insecure: bool,

        /// Start paused
        #[arg(long, short('a'))]
        no_autoplay: bool,

        /// Don't await for page load
        #[arg(long, short('w'), hide(true))]
        no_wait: bool,

        /// Don't sync this screen to other players
        #[arg(long, short('s'))]
        no_send: bool,

        /// Hidden screen with global/unchanging volume
        ///
        /// Also sets resolution to 1x1 and fps to 1 for performance
        #[arg(long, short)]
        global: bool,

        /// Don't show Now Playing messages
        ///
        /// (may not be allowed on some urls)
        #[arg(long, short('q'), alias("quiet"))]
        silent: bool,

        /// Loop after track finishes playing
        #[arg(long, short)]
        r#loop: bool,

        /// Use transparent background
        ///
        /// Note that text can appear strange (pixels missing)
        #[arg(long, short)]
        transparent: bool,

        // url has to be multiple because urls can be chopped in half by
        // line continuations, so we join the parts together as a hack
        #[arg(allow_hyphen_values(true))]
        url: Vec<String>,
    },

    /// Close all screens
    #[command(
        name("closeall"),
        aliases(["close-all", "removeall", "clearall"])
    )]
    CloseAll,

    /// Run a "/Reply" command on the server
    ///
    /// This is useful to use in scripts to know when cef commands have finished executing.
    /// For example when screens are finished loading via "cef create".
    Reply { num: String },

    /// Run a "/ReplyTwo" command on the server
    ///
    /// This is useful to use in scripts to know when cef commands have finished executing.
    /// For example when screens are finished loading via "cef create".
    #[command(name("replytwo"), alias("reply-two"))]
    ReplyTwo { num: String },
}

pub async fn run(player_snapshot: PlayerSnapshot, commands: Commands) -> Result<()> {
    match commands {
        Commands::Alias { alias, url } => {
            url_aliases::add_alias(&alias, &url)?;
        }

        Commands::Create {
            global,
            insecure,
            r#loop,
            name,
            no_autoplay,
            no_send,
            no_wait,
            silent,
            transparent,
            url,
        } => {
            let url = if url.is_empty() {
                "https://www.classicube.net/".to_string()
            } else {
                url.join("")
            };

            let autoplay = !no_autoplay;
            let wait_for_page_load = !no_wait;
            let should_send = !no_send;
            let should_loop = r#loop;

            if let Some(id) = name.as_ref().and_then(|name| name.get_entity_id().ok()) {
                drop(EntityManager::remove_entity(id).await);
            }

            let mut player_builder = PlayerBuilder::new()
                .autoplay(autoplay)
                .should_loop(should_loop)
                .silent(silent);

            if global {
                player_builder = player_builder.volume_mode(VolumeMode::Global);
            }

            let mut players = player_builder.build(&url).await?;
            let player = players.remove(0);

            let transparent = transparent || matches!(&player, Player::Image(_));

            let mut entity_builder = EntityBuilder::new(player)
                .queue(players.into())
                .insecure(insecure)
                .should_send(should_send);

            if global {
                // 1 fps, 1x1 resolution
                entity_builder = entity_builder.resolution(1, 1).frame_rate(1).scale(0.0);
            }

            if transparent {
                entity_builder = entity_builder.background_color(0x00FF_FFFF);
            }

            if let Some(name) = name {
                entity_builder = entity_builder.name(name);
            }

            let entity_id = entity_builder.create().await?;

            if !global {
                EntityManager::with_entity(entity_id, |entity| {
                    move_entity(entity, &player_snapshot);
                    Ok(())
                })?;
            }

            if wait_for_page_load {
                let page_load =
                    EntityManager::with_entity(
                        entity_id,
                        |entity| Ok(entity.wait_for_page_load()),
                    )?;

                // wait for browser to load
                if page_load.await.is_err() {
                    bail!("wait_for_page_load cancelled");
                }
            }
        }

        Commands::CloseAll => {
            async_manager::spawn_local_on_main_thread(async {
                let _ignore_error = EntityManager::remove_all_entities().await;
            });
        }

        Commands::Reply { num } => {
            let owned_string = OwnedString::new(format!("/Reply {num}"));
            unsafe {
                Chat_Send(owned_string.as_cc_string(), 0);
            }
        }

        Commands::ReplyTwo { num } => {
            let owned_string = OwnedString::new(format!("/ReplyTwo {num}"));
            unsafe {
                Chat_Send(owned_string.as_cc_string(), 0);
            }
        }
    }

    Ok(())
}
