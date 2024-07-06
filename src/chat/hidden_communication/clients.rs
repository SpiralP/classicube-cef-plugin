use super::{wait_for_message, SHOULD_BLOCK};
use crate::{
    chat::{
        helpers::{is_clients_message, is_clients_start_message},
        hidden_communication::whispers::start_whispering,
        is_continuation_message, Chat, TAB_LIST,
    },
    error::{Result, ResultExt},
    plugin::APP_NAME,
};
use classicube_helpers::async_manager;
use classicube_helpers::{tab_list::remove_color, WithInner};
use classicube_sys::ENTITIES_SELF_ID;
use futures::{future::RemoteHandle, prelude::*};
use std::{cell::RefCell, collections::HashSet, time::Duration};
use tracing::{debug, warn};

thread_local!(
    static CURRENT_RUNNING: RefCell<Option<RemoteHandle<()>>> = RefCell::default();
);

pub fn query() {
    let (f, remote_handle) = async {
        // hack so that when query() is called a bunch after Init() from loader,
        // we won't run /clients more than once
        // TODO gross
        async_manager::sleep(Duration::from_millis(100)).await;

        // whole query shouldn't take more than 30 seconds
        // includes whispering and browser creation
        let result = async_manager::timeout_local(Duration::from_secs(30), async {
            let messages = get_clients().await?;
            process_clients_response(messages).await
        })
        .await;
        match result {
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

async fn get_clients() -> Result<Vec<String>> {
    // TODO check for "Server software: MCGalaxy 1.9.2.0"

    // Server software: ProCraft
    // &e  ClassiCube 1.1.3 +cef0.5.6 +cs3.4.2:&7 SpiralP
    // &e  ClassiCube 1.1.4 + More Models v1.2.4:&c cybertoon
    // >&8 [&9Owner&8][&cm&4a&6p&ep&ay&20&b1&8]&f,&c cybertoon,
    // &e  ClassiCube 1.1.4 + Ponies v2.1:&1 *&bgemsgem&1*

    debug!("querying /clients");

    async_manager::timeout(Duration::from_secs(3), async {
        Chat::send("/clients");

        loop {
            let message = wait_for_message().await;
            if is_clients_start_message(&message) {
                SHOULD_BLOCK.set(true);
                break;
            }
            // keep checking other messages until we find ^
        }
    })
    .await
    .chain_err(|| "never found start of clients response")?;

    let mut messages = Vec::new();
    let timeout_result = async_manager::timeout(Duration::from_secs(3), async {
        let mut was_clients_message = false;

        loop {
            let message = wait_for_message().await;

            if was_clients_message {
                if let Some(message) = is_continuation_message(&message) {
                    let message = remove_color(message);
                    SHOULD_BLOCK.set(true);

                    let last_message = messages.last_mut().unwrap();
                    *last_message = format!("{last_message} {message}");
                    continue;
                }
            }
            if let Some(message) = is_clients_message(&message) {
                let message = remove_color(message);
                SHOULD_BLOCK.set(true);

                // a /clients response
                messages.push(message);

                was_clients_message = true;
            } else {
                was_clients_message = false;
            }
            // keep checking because other messages can be shown in the middle of
            // the /clients response
        }
    })
    .await;

    if timeout_result.is_none() {
        debug!("stopping because of timeout");
    }

    Ok(messages)
}

fn get_names_with_cef(messages: &[String]) -> Result<HashSet<String>> {
    // cef0.13.2-alpha.0
    let app_name_without_last_number = format!(
        "{}.",
        APP_NAME
            .splitn(3, '.')
            .take(2)
            .collect::<Vec<_>>()
            .join(".")
    );
    debug!("{:#?} {:?}", messages, app_name_without_last_number);

    let mut names_with_cef: HashSet<String> = HashSet::new();
    for message in messages {
        let pos = message.find(": ").chain_err(|| "couldn't find colon")?;

        let (left, right) = message.split_at(pos);

        // skip ": "
        if let Some(right) = right.get(2..) {
            let names: HashSet<String> = right.split(", ").map(ToString::to_string).collect();

            if left.contains(&app_name_without_last_number) {
                for name in names {
                    names_with_cef.insert(name);
                }
            }
        }
    }

    Ok(names_with_cef)
}

async fn process_clients_response(messages: Vec<String>) -> Result<()> {
    let mut names_with_cef = get_names_with_cef(&messages)?;

    debug!("{:#?}", names_with_cef);

    let players_with_cef: Vec<(u8, String)> = names_with_cef
        .drain()
        .filter_map(|name| {
            TAB_LIST.with_inner(|tab_list| {
                tab_list.find_entry_by_nick_name(&name).and_then(|entry| {
                    let entry = entry.upgrade()?;
                    let id = entry.get_id();
                    if id == ENTITIES_SELF_ID as u8 {
                        None
                    } else {
                        let real_name = entry.get_real_name();
                        Some((id, real_name))
                    }
                })
            })?
        })
        .collect();

    if !players_with_cef.is_empty() {
        start_whispering(players_with_cef).await?;
    }

    Ok(())
}

#[test]
fn test_get_names_with_cef() {
    let parts = APP_NAME
        .get(3..)
        .unwrap()
        .splitn(3, '.')
        .collect::<Vec<_>>();
    let without_last_number = format!(
        "cef{}",
        parts.iter().copied().take(2).collect::<Vec<_>>().join(".")
    );

    {
        let lines = vec![
            format!("ClassiCube 1.2.4 + {APP_NAME}: name1"),
            format!("ClassiCube 1.2.4 {APP_NAME} +cs3.5.15 + Ponies v2.1: name2"),
            format!("ClassiCube 1.2.4 + {APP_NAME} +cs3.5.15 + Ponies v2.1: name3"),
            format!("ClassiCube 1.2.4 {APP_NAME} cs3.5.15 + Ponies v2.1: name4"),
            format!("ClassiCube 1.2.4 {}.0: name5", without_last_number),
            format!("ClassiCube 1.2.4 {}.: name6", without_last_number),
        ];

        let r = get_names_with_cef(&lines).unwrap();
        assert!(r.contains("name1"));
        assert!(r.contains("name2"));
        assert!(r.contains("name3"));
        assert!(r.contains("name4"));
        assert!(r.contains("name5"));
        assert!(r.contains("name6"));
    }

    {
        let with_next_minor = format!("cef{}.99.0", parts[0]);
        let lines = vec![
            format!("ClassiCube 1.2.4 {}: name7", with_next_minor),
            format!("ClassiCube 1.2.4 + {}: name8", with_next_minor),
            format!("ClassiCube 1.2.4 cef0.0.0: name9"),
            format!("ClassiCube 1.2.4 + cef0.0.0: name10"),
        ];

        let r = get_names_with_cef(&lines).unwrap();
        assert!(!r.contains("name7"));
        assert!(!r.contains("name8"));
        assert!(!r.contains("name9"));
        assert!(!r.contains("name10"));
    }
}

#[test]
fn test_get_clients() {
    crate::logger::initialize(true, false, false);
    async_manager::initialize();

    async_manager::spawn_local_on_main_thread(async {
        let r = get_clients().await.unwrap();

        let mut iter = r.iter();
        assert_eq!(
            iter.next().unwrap(),
            "ClassiCube 1.2.4 + cef1.3.0 +cs3.5.15 + Ponies v2.1: name"
        );
        assert_eq!(
            iter.next().unwrap(),
            "ClassiCube 1.2.4 + cef1.3.0 +cs3.6.0 + MM 1.2.5 + Ponies v2.1: SpiralP"
        );
    });

    let messages = vec![
        ("hi", false),
        ("&7Players using:", true),
        (
            "&7  ClassiCube 1.2.4 + cef1.3.0 +cs3.5.15 + Ponies v2.1:",
            true,
        ),
        ("> &7&fname", true),
        ("asdf", false),
        (
            "&7  ClassiCube 1.2.4 + cef1.3.0 +cs3.6.0 + MM 1.2.5 + Ponies",
            true,
        ),
        ("> &7v2.1: &fSpiralP", true),
        ("okay", false),
    ];

    for (message, should_block) in messages {
        assert_eq!(
            super::handle_chat_message(message),
            should_block,
            "{message:?}"
        );
    }

    async_manager::run();
    async_manager::shutdown();
}
