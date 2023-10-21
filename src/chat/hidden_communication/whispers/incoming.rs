use super::{encoding, wait_for_message, SHOULD_BLOCK};
use crate::{
    chat::{
        helpers::{
            is_cef_reply_whisper, is_cef_request_whisper, is_incoming_whisper, is_outgoing_whisper,
        },
        is_continuation_message, Chat, ENTITIES, TAB_LIST,
    },
    error::{Result, ResultExt},
};
use classicube_helpers::async_manager;
use classicube_helpers::{shared::FutureShared, WithInner};
use std::time::Duration;
use tracing::{debug, info, warn};

pub async fn listen_loop() {
    loop {
        let message = wait_for_message().await;

        // incoming info request whisper
        if is_incoming_whisper(&message) && is_cef_request_whisper(&message) {
            SHOULD_BLOCK.set(true);

            info!("incoming_whisper {:?}", message);

            async_manager::spawn_local_on_main_thread(async move {
                match handle_request(message).await {
                    Ok(()) => {}

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
    let colon_pos = message.find(": ").chain_err(|| "couldn't find colon")?;
    let nick_name = message.get(6..colon_pos).chain_err(|| "char boundary")?;
    info!("from {:?}", nick_name);

    // find real nick
    let maybe_real_name = TAB_LIST
        .with_inner(|tab_list| {
            tab_list
                .find_entry_by_nick_name(nick_name)
                .and_then(|entry| {
                    let entry = entry.upgrade()?;
                    let id = entry.get_id();

                    // make sure they're real
                    ENTITIES
                        .with_inner(|entities| {
                            if entities.get(id).is_some() {
                                Some(entry.get_real_name())
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
        let mut mutex = SENDING.with(Clone::clone);
        let mutex = mutex.lock().await;

        send_reply(real_name).await?;

        // don't trigger spam mute
        async_manager::sleep(Duration::from_secs(2)).await;

        drop(mutex);
    }

    Ok(())
}

async fn send_reply(real_name: String) -> Result<()> {
    debug!("sending to {:?}", real_name);

    let message = encoding::create_message();

    if message.entities.is_empty() {
        // don't send anything if nothing to send, asker will time out and ask someone else
        debug!("no entities to send, not responding");
        return Ok(());
    }

    let encoded = encoding::encode(&message)?;
    debug!("sending encoded message length {}", encoded.len());

    // my outgoing info reply whisper
    async_manager::timeout(Duration::from_secs(5), async {
        Chat::send(format!("@{real_name} !CEF!{encoded}"));

        loop {
            let message = wait_for_message().await;

            if is_outgoing_whisper(&message) && is_cef_reply_whisper(&message) {
                SHOULD_BLOCK.set(true);

                // also block > continuation messages
                let timeout_result = async_manager::timeout(Duration::from_secs(1), async {
                    loop {
                        let message = wait_for_message().await;
                        if let Some(_continuation) = is_continuation_message(&message) {
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
