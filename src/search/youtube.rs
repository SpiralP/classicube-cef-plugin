use crate::{async_manager::AsyncManager, error::*};
use invidious::api::search;
use log::debug;

pub async fn search(input: &str) -> Result<String> {
    let input = input.to_string();

    AsyncManager::spawn(async move {
        debug!("searching {:?}", input);
        let schema = search::request(search::Parameters {
            q: Some(input),
            ..Default::default()
        })
        .await?;

        let first = schema.get(0).chain_err(|| "no results")?;
        if let search::SchemaType::Video(video) = &first {
            let id = video.video_id.to_string();
            Ok(id)
        } else {
            Err("other schema type found, not video".into())
        }
    })
    .await?
}
