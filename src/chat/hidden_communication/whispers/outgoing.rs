use super::{wait_for_message, SHOULD_BLOCK};
use crate::{chat::Chat, error::*};
use async_std::future::timeout;
use classicube_helpers::CellGetSet;
use log::debug;
use std::time::Duration;

pub async fn query_whisper(real_name: String) -> Result<()> {
    debug!("query_whisper asking {}", real_name);

    Chat::send(format!("@{}+ ?CEF?", real_name));
    // SpiralP2 -> SpiralP
    // &7[<] &uSpiralP2: &f?CEF?
    // &9[>] &uSpiralP: &f?CEF?

    // my outgoing whisper
    timeout(Duration::from_secs(5), async {
        loop {
            let message = wait_for_message().await;
            if message.starts_with("&7[<] ") && message.ends_with(": &f?CEF?") {
                SHOULD_BLOCK.set(true);
                break;
            }
        }
    })
    .await
    .chain_err(|| "never found my outgoing whisper")?;

    // incoming whisper from them
    timeout(Duration::from_secs(5), async {
        loop {
            let message = wait_for_message().await;
            if message.starts_with("&9[>] ") && message.contains(": &f!CEF! ") {
                SHOULD_BLOCK.set(true);
                debug!("got whisper response {:?}", message);

                break;
            }
        }
    })
    .await
    .chain_err(|| "never found response to my whisper")?;

    Ok(())
}
