use super::{encoding, wait_for_message, SHOULD_BLOCK};
use crate::{
    async_manager,
    chat::{
        helpers::{
            is_cef_reply_whisper, is_cef_request_whisper, is_incoming_whisper, is_outgoing_whisper,
        },
        is_continuation_message, Chat,
    },
    error::*,
};
use classicube_helpers::CellGetSet;
use std::time::Duration;
use tracing::debug;

pub async fn query_whisper(real_name: &str) -> Result<bool> {
    debug!("query_whisper asking {}", real_name);

    // my outgoing info request whisper
    async_manager::timeout(Duration::from_secs(3), async {
        Chat::send(format!("@{} ?CEF?", real_name));
        // SpiralP2 -> SpiralP
        // &7[<] &uSpiralP2: &f?CEF?
        // &9[>] &uSpiralP: &f?CEF?

        loop {
            let message = wait_for_message().await;

            if is_outgoing_whisper(&message) && is_cef_request_whisper(&message) {
                SHOULD_BLOCK.set(true);
                break;
            }
        }
    })
    .await
    .chain_err(|| "never found my outgoing whisper")?;

    // incoming info reply whisper
    let full_message_encoded = async_manager::timeout(Duration::from_secs(5), async {
        loop {
            let message = wait_for_message().await;
            if is_incoming_whisper(&message) && is_cef_reply_whisper(&message) {
                SHOULD_BLOCK.set(true);
                debug!("got whisper response {:?}", message);

                let mut parts: Vec<String> = Vec::new();

                let first_parts = message.splitn(2, "!CEF!").collect::<Vec<_>>();
                let first_encoded = first_parts[1].to_string();
                parts.push(first_encoded);

                let timeout_result = async_manager::timeout(Duration::from_secs(1), async {
                    loop {
                        let message = wait_for_message().await;
                        if let Some(continuation) = is_continuation_message(&message) {
                            SHOULD_BLOCK.set(true);

                            parts.push(continuation.to_string());
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
