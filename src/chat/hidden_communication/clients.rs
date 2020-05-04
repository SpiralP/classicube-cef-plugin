use super::{wait_for_message, SHOULD_BLOCK};
use crate::{
    async_manager::AsyncManager,
    chat::{hidden_communication::whispers::start_whispering, Chat, TAB_LIST},
    error::*,
    plugin::APP_NAME,
};
use async_std::future::timeout;
use classicube_helpers::{tab_list::remove_color, CellGetSet, OptionWithInner};
use classicube_sys::ENTITIES_SELF_ID;
use futures::{future::RemoteHandle, prelude::*};
use log::{debug, warn};
use std::{cell::Cell, sync::Once, time::Duration};

thread_local!(
    static CURRENT_RUNNING: Cell<Option<RemoteHandle<()>>> = Default::default();
);

pub fn query() {
    let (f, remote_handle) = async {
        // whole query shouldn't take more than 30 seconds
        // includes whispering and browser creation
        match timeout(Duration::from_secs(30), do_query()).await {
            Ok(result) => {
                if let Err(e) = result {
                    warn!("clients query failed: {}", e);
                }
            }

            Err(_timeout) => {
                warn!("clients query timed out");
            }
        }
    }
    .remote_handle();

    AsyncManager::spawn_local_on_main_thread(f);

    CURRENT_RUNNING.with(move |cell| {
        cell.set(Some(remote_handle));
    });
}

async fn do_query() -> Result<()> {
    // TODO check for "Server software: MCGalaxy 1.9.2.0"

    // Server software: ProCraft
    // &e  ClassiCube 1.1.3 +cef0.5.6 +cs3.4.2:&7 SpiralP
    // &e  ClassiCube 1.1.4 + More Models v1.2.4:&c cybertoon
    // >&8 [&9Owner&8][&cm&4a&6p&ep&ay&20&b1&8]&f,&c cybertoon,
    // &e  ClassiCube 1.1.4 + Ponies v2.1:&1 *&bgemsgem&1*

    debug!("querying /clients");
    Chat::send("/clients");

    timeout(Duration::from_secs(3), async {
        loop {
            let message = wait_for_message().await;
            if message.len() >= 2
                && (&message.as_bytes()[0..1] == b"&"
                    && &message.as_bytes()[2..] == b"Players using:")
            {
                SHOULD_BLOCK.set(true);
                break;
            }
            // keep checking other messages until we find ^
        }
    })
    .await
    .chain_err(|| "never found start of clients response")?;

    let mut messages = Vec::new();

    let timeout_result = timeout(Duration::from_secs(3), async {
        loop {
            let message = wait_for_message().await;
            if message.len() >= 4
                && ((&message.as_bytes()[0..1] == b"&" && &message.as_bytes()[2..4] == b"  ")
                    || &message.as_bytes()[0..3] == b"> &")
            {
                // probably a /clients response
                messages.push(message.to_string());

                // don't show this message!
                SHOULD_BLOCK.set(true);
            } else {
                debug!("stopping because of other message {:?}", message);
                break;
            }
        }
    })
    .await;

    if timeout_result.is_err() {
        debug!("stopping because of timeout");
    }

    process_clients_response(messages).await?;

    Ok(())
}

async fn process_clients_response(messages: Vec<String>) -> Result<()> {
    let mut full_lines = Vec::new();

    for message in &messages {
        // if we start with "&f  "
        if &message.as_bytes()[0..1] == b"&" && &message.as_bytes()[2..4] == b"  " {
            // start of line

            // "&7  "
            full_lines.push(message[4..].to_string());
        } else {
            // a continuation message

            let last = full_lines.last_mut().chain_err(|| "no last?")?;

            // "> &f" or "> &7"
            *last = format!("{} {}", last, message[2..].to_string());
        }
    }

    debug!("{:#?}", full_lines);

    let mut names_with_cef: Vec<String> = Vec::new();
    for message in &full_lines {
        let pos = message.find(": &f").chain_err(|| "couldn't find colon")?;

        let (left, right) = message.split_at(pos);
        // skip ": &f"
        let right = &right[4..];

        let left = remove_color(left);
        let right = remove_color(right);

        let mut names: Vec<String> = right.split(", ").map(|a| a.to_string()).collect();

        let app_name_without_last_number = APP_NAME.rsplitn(2, '.').nth(1).unwrap();
        if left.contains(app_name_without_last_number) {
            names_with_cef.append(&mut names);
        }
    }

    debug!("{:#?}", names_with_cef);

    let players_with_cef: Vec<(u8, String)> = names_with_cef
        .drain(..)
        .filter_map(|name| {
            TAB_LIST.with_inner(|tab_list| {
                tab_list.find_entry_by_nick_name(&name).and_then(|entry| {
                    let id = entry.get_id();
                    if id == ENTITIES_SELF_ID as u8 {
                        None
                    } else {
                        let real_name = entry.get_real_name()?;
                        Some((id, real_name))
                    }
                })
            })?
        })
        .collect();

    if !players_with_cef.is_empty() {
        let names: Vec<&str> = players_with_cef
            .iter()
            .map(|(_id, name)| name.as_str())
            .collect();

        static ONCE: Once = Once::new();
        ONCE.call_once(move || {
            Chat::print(format!("Other players with cef: {}", names.join(", ")));
        });

        start_whispering(players_with_cef).await?;
    }

    Ok(())
}
