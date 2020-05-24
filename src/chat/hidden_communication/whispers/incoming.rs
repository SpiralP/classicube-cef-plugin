use super::{encoding, wait_for_message, SHOULD_BLOCK};
use crate::{
    async_manager::AsyncManager,
    chat::{
        helpers::{is_incoming_whisper, is_outgoing_whisper},
        Chat, ENTITIES, TAB_LIST,
    },
    error::*,
};
use classicube_helpers::{shared::FutureShared, CellGetSet, OptionWithInner};
use log::{debug, info, warn};
use std::time::Duration;

pub async fn listen_loop() {
    loop {
        let message = wait_for_message().await;

        // incoming whisper
        if is_incoming_whisper(&message) && message.ends_with(": &f?CEF?") {
            SHOULD_BLOCK.set(true);

            info!("incoming_whisper {:?}", message);

            AsyncManager::spawn_local_on_main_thread(async move {
                match handle_request(message).await {
                    Ok(_) => {}

                    Err(e) => {
                        warn!("whisper handle_request: {}", e);
                    }
                }
            });
        }
    }
}

// so that we only ever send 1 message every few seconds
// and not get muted for spamming
thread_local!(
    static SENDING: FutureShared<()> = FutureShared::new(());
);

async fn handle_request(message: String) -> Result<()> {
    // "&9[>] "
    let colon_pos = message.find(": &f").chain_err(|| "couldn't find colon")?;
    let nick_name = &message[6..colon_pos];
    info!("from {:?}", nick_name);

    // find real nick
    let maybe_real_name = TAB_LIST
        .with_inner(|tab_list| {
            tab_list
                .find_entry_by_nick_name(&nick_name)
                .and_then(|entry| {
                    let id = entry.get_id();

                    // make sure they're real
                    ENTITIES
                        .with_inner(|entities| {
                            if entities.get(id).is_some() {
                                Some(entry.get_real_name()?)
                            } else {
                                None
                            }
                        })
                        .chain_err(|| "ENTITIES")
                        .ok()?
                })
        })
        .chain_err(|| "TAB_LIST")?;

    if let Some(real_name) = maybe_real_name {
        let mut mutex = SENDING.with(|m| m.clone());
        let mutex = mutex.lock().await;

        send_reply(real_name).await?;

        // don't trigger spam mute
        AsyncManager::sleep(Duration::from_secs(2)).await;

        drop(mutex);
    }

    Ok(())
}

async fn send_reply(real_name: String) -> Result<()> {
    debug!("sending to {:?}", real_name);

    let message = encoding::create_message().await;

    if message.entities.is_empty() {
        // don't send anything if nothing to send, asker will time out and ask someone else
        debug!("no entities to send, not responding");
        return Ok(());
    }

    let encoded = encoding::encode(&message)?;
    Chat::send(format!("@{}+ !CEF!{}", real_name, encoded));

    // my outgoing whisper
    AsyncManager::timeout(Duration::from_secs(5), async {
        loop {
            let message = wait_for_message().await;

            if is_outgoing_whisper(&message) && message.contains(": &f!CEF!") {
                SHOULD_BLOCK.set(true);

                // also block > continuation messages
                let timeout_result = AsyncManager::timeout(Duration::from_secs(1), async {
                    loop {
                        let message = wait_for_message().await;
                        if message.starts_with("> &f") {
                            // a continuation "> &f"
                            SHOULD_BLOCK.set(true);
                        } else {
                            debug!("stopping because of other message {:?}", message);
                            break;
                        }
                    }
                })
                .await;

                if timeout_result.is_none() {
                    debug!("stopping because of timeout");
                }

                break;
            }
        }
    })
    .await
    .chain_err(|| "never found my outgoing whisper reply")?;

    Ok(())
}
