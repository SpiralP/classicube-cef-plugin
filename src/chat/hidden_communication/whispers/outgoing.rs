use super::{encoding, wait_for_message, SHOULD_BLOCK};
use crate::{
    async_manager::AsyncManager,
    chat::{
        helpers::{is_incoming_whisper, is_outgoing_whisper},
        Chat,
    },
    error::*,
};
use classicube_helpers::CellGetSet;
use log::debug;
use std::time::Duration;

pub async fn query_whisper(real_name: &str) -> Result<bool> {
    debug!("query_whisper asking {}", real_name);

    Chat::send(format!("@{}+ ?CEF?", real_name));
    // SpiralP2 -> SpiralP
    // &7[<] &uSpiralP2: &f?CEF?
    // &9[>] &uSpiralP: &f?CEF?

    // my outgoing whisper
    AsyncManager::timeout(Duration::from_secs(3), async {
        loop {
            let message = wait_for_message().await;

            if is_outgoing_whisper(&message) && message.ends_with(": &f?CEF?") {
                SHOULD_BLOCK.set(true);
                break;
            }
        }
    })
    .await
    .chain_err(|| "never found my outgoing whisper")?;

    // incoming whisper from them
    let full_message_encoded = AsyncManager::timeout(Duration::from_secs(5), async {
        loop {
            let message = wait_for_message().await;
            if is_incoming_whisper(&message) && message.contains(": &f!CEF!") {
                SHOULD_BLOCK.set(true);
                debug!("got whisper response {:?}", message);

                let mut parts: Vec<String> = Vec::new();

                let first_parts = message.splitn(2, ": &f!CEF!").collect::<Vec<_>>();
                let first_encoded = first_parts[1].to_string();
                parts.push(first_encoded);

                let timeout_result = AsyncManager::timeout(Duration::from_secs(1), async {
                    loop {
                        let message = wait_for_message().await;
                        if message.starts_with("> &f") {
                            // a continuation "> &f"
                            SHOULD_BLOCK.set(true);

                            parts.push(message[4..].to_string());
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

                let encoded: String = parts.join("");

                break Ok::<String, Error>(encoded);
            }
        }
    })
    .await
    .chain_err(|| "never found response to my whisper")??;

    debug!("got encoded message length {}", full_message_encoded.len());
    let message = encoding::decode(full_message_encoded)?;
    debug!("decoded {:#?}", message);
    Ok(encoding::received_message(message).await?)
}
