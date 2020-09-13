use super::{wait_for_message, SHOULD_BLOCK};
use crate::{
    async_manager,
    chat::{hidden_communication::whispers::start_whispering, Chat, TAB_LIST},
    error::*,
    plugin::APP_NAME,
};
use classicube_helpers::{tab_list::remove_color, CellGetSet, OptionWithInner};
use classicube_sys::ENTITIES_SELF_ID;
use futures::{future::RemoteHandle, prelude::*};
use log::{debug, warn};
use std::{cell::RefCell, collections::HashSet, sync::Once, time::Duration};

thread_local!(
    static CURRENT_RUNNING: RefCell<Option<RemoteHandle<()>>> = Default::default();
);

pub fn query() {
    let (f, remote_handle) = async {
        // hack so that when query() is called a bunch after Init() from loader,
        // we won't run /clients more than once
        // TODO gross
        async_manager::sleep(Duration::from_millis(100)).await;

        // whole query shouldn't take more than 30 seconds
        // includes whispering and browser creation
        match async_manager::timeout_local(Duration::from_secs(30), do_query()).await {
            Some(result) => {
                if let Err(e) = result {
                    warn!("clients query failed: {}", e);
                }
            }

            None => {
                warn!("clients query timed out");
            }
        }

        CURRENT_RUNNING.with(move |cell| {
            let opt = &mut *cell.borrow_mut();
            *opt = None;
        });
    }
    .remote_handle();

    async_manager::spawn_local_on_main_thread(f);

    CURRENT_RUNNING.with(move |cell| {
        let opt = &mut *cell.borrow_mut();
        *opt = Some(remote_handle);
    });
}

pub fn stop_query() {
    // CURRENT_RUNNING.with(move |cell| {
    //     let opt = &mut *cell.borrow_mut();
    //     *opt = None;
    // });
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

    fn is_clients_start_message(bytes: &[u8]) -> bool {
        bytes.len() >= 14 && (&bytes[0..1] == b"&" && &bytes[2..] == b"Players using:")
    }

    async_manager::timeout(Duration::from_secs(3), async {
        loop {
            let message = wait_for_message().await;
            if is_clients_start_message(message.as_bytes()) {
                SHOULD_BLOCK.set(true);
                break;
            }
            // keep checking other messages until we find ^
        }
    })
    .await
    .chain_err(|| "never found start of clients response")?;

    let mut was_clients_message = false;
    let mut is_clients_message = move |bytes: &[u8]| -> bool {
        // &7  ClassiCube 1.1.6 + cef0.9.4 + Ponies v2.1: &f¿ Mew, ┌ Glim
        // &7  ClassiCube 1.1.6 + cef0.9.4 +cs3.4.5 + More Models v1.2.4 +
        // > &7Poni: &fSpiralP
        // &7  ClassiCraft 1.1.3: &fFaeEmpress
        let is = if bytes.len() >= 5 && (&bytes[0..1] == b"&" && &bytes[2..4] == b"  ") {
            true
        } else {
            was_clients_message && bytes.len() >= 3 && &bytes[0..3] == b"> &"
        };

        was_clients_message = is;

        is
    };

    let mut messages = Vec::new();
    let timeout_result = async_manager::timeout(Duration::from_secs(3), async {
        loop {
            let message = wait_for_message().await;
            if is_clients_message(message.as_bytes()) {
                // probably a /clients response
                messages.push(message.to_string());

                // don't show this message!
                SHOULD_BLOCK.set(true);
            }
            // keep checking because other messages can be shown in the middle of
            // the /clients response
        }
    })
    .await;

    if timeout_result.is_none() {
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

    // cef0.13.2-alpha.0
    let app_name_without_last_number = format!(
        "{}.",
        APP_NAME
            .splitn(3, '.')
            .take(2)
            .collect::<Vec<_>>()
            .join(".")
    );
    debug!("{:#?} {:?}", full_lines, app_name_without_last_number);

    let mut names_with_cef: HashSet<String> = HashSet::new();
    for message in &full_lines {
        let pos = message.find(": &f").chain_err(|| "couldn't find colon")?;

        let (left, right) = message.split_at(pos);
        // skip ": &f"
        let right = &right[4..];

        let left = remove_color(left);
        let right = remove_color(right);

        let names: HashSet<String> = right.split(", ").map(|a| a.to_string()).collect();

        if left.contains(&app_name_without_last_number) {
            for name in names {
                names_with_cef.insert(name);
            }
        }
    }

    debug!("{:#?}", names_with_cef);

    let players_with_cef: Vec<(u8, String)> = names_with_cef
        .drain()
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
        // let names: Vec<&str> = players_with_cef
        //     .iter()
        //     .map(|(_id, name)| name.as_str())
        //     .collect();

        let len = players_with_cef.len();
        static ONCE: Once = Once::new();
        ONCE.call_once(move || {
            Chat::print(format!("{} other players with cef!", len));
        });

        start_whispering(players_with_cef).await?;
    }

    Ok(())
}
