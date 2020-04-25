use super::{encoding, wait_for_message, SHOULD_BLOCK};
use crate::{
    chat::{Chat, ENTITIES, TAB_LIST},
    error::*,
};
use async_std::future::timeout;
use classicube_helpers::{CellGetSet, OptionWithInner};
use log::{debug, info, warn};
use std::time::Duration;

pub async fn listen_loop() {
    loop {
        match step().await {
            Ok(_) => {}

            Err(e) => {
                warn!("whisper listen_loop: {}", e);
            }
        }
    }
}

async fn step() -> Result<()> {
    let message = wait_for_message().await;

    // incoming whisper
    if &message.as_bytes()[0..1] == b"&"
        && &message.as_bytes()[2..6] == b"[>] "
        && message.ends_with(": &f?CEF?")
    {
        SHOULD_BLOCK.set(true);

        info!("incoming_whisper {:?}", message);

        // "&9[>] "
        let colon_pos = message.find(": &f").chain_err(|| "couldn't find colon")?;
        let nick_name = &message[6..colon_pos];
        info!("from {:?}", nick_name);

        // find real nick and also make sure they're real
        let maybe_real_name = TAB_LIST
            .with_inner(|tab_list| {
                tab_list
                    .find_entry_by_nick_name(&nick_name)
                    .and_then(|entry| {
                        let id = entry.get_id();

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
            debug!("sending to {:?}", real_name);

            let message = encoding::create_message();
            let encoded = encoding::encode(&message)?;
            Chat::send(format!("@{}+ !CEF!{}", real_name, encoded));

            // my outgoing whisper
            timeout(Duration::from_secs(5), async {
                loop {
                    let message = wait_for_message().await;

                    if &message.as_bytes()[0..1] == b"&"
                        && &message.as_bytes()[2..6] == b"[<] "
                        && message.contains(": &f!CEF!")
                    {
                        SHOULD_BLOCK.set(true);

                        // also block > continuation messages
                        let timeout_result = timeout(Duration::from_secs(1), async {
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

                        if timeout_result.is_err() {
                            debug!("stopping because of timeout");
                        }

                        break;
                    }
                }
            })
            .await
            .chain_err(|| "never found my outgoing whisper reply")?;
        }
    }

    Ok(())
}
