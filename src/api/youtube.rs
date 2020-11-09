use crate::{async_manager, error::*};
use serde::Deserialize;

pub const API_URL: &str = "http://youtube.spiralp.uk.to:43210";

#[derive(Debug, Deserialize)]
struct ApiError {
    code: u64,
    message: String,
}

#[derive(Debug, Deserialize)]
pub struct VideoResponse {
    pub title: String,
    pub duration_seconds: u64,
}

pub async fn video(id: &str) -> Result<VideoResponse> {
    let id = id.to_string();

    let result = async_manager::spawn(async move {
        let client = reqwest::Client::new();
        let bytes = client
            .get(&format!("{}/video/{}", API_URL, id))
            .send()
            .await?
            .bytes()
            .await?;

        if let Ok(error) = serde_json::from_slice::<ApiError>(&bytes) {
            bail!("{}", error.message);
        } else {
            Ok::<_, Error>(serde_json::from_slice::<VideoResponse>(&bytes)?)
        }
    })
    .await??;

    Ok(result)
}

pub async fn playlist(id: &str) -> Result<Vec<String>> {
    let id = id.to_string();

    let result = async_manager::spawn(async move {
        let client = reqwest::Client::new();
        let bytes = client
            .get(&format!("{}/playlist/{}", API_URL, id))
            .send()
            .await?
            .bytes()
            .await?;

        if let Ok(error) = serde_json::from_slice::<ApiError>(&bytes) {
            bail!("{}", error.message);
        } else {
            Ok::<_, Error>(serde_json::from_slice::<Vec<String>>(&bytes)?)
        }
    })
    .await??;

    Ok(result)
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub id: String,
    pub title: String,
    pub duration_seconds: u64,
}

pub async fn search(query: &str) -> Result<SearchResponse> {
    let query = query.to_string();

    let result = async_manager::spawn(async move {
        let client = reqwest::Client::new();
        let bytes = client
            .get(&format!("{}/search", API_URL))
            .query(&[("q", &query)])
            .send()
            .await?
            .bytes()
            .await?;

        if let Ok(error) = serde_json::from_slice::<ApiError>(&bytes) {
            bail!("{}", error.message);
        } else {
            Ok::<_, Error>(serde_json::from_slice::<SearchResponse>(&bytes)?)
        }
    })
    .await??;

    Ok(result)
}

#[cfg(test)]
#[no_mangle]
extern "C" fn Gfx_DeleteTexture() {}

#[cfg(test)]
#[no_mangle]
extern "C" fn Gfx_CreateTexture() {}

#[cfg(test)]
#[no_mangle]
extern "C" fn Entity_SetModel() {}

#[cfg(test)]
#[no_mangle]
extern "C" fn Options_Get() {}

#[cfg(test)]
#[no_mangle]
extern "C" fn Options_Set() {}

#[cfg(test)]
#[no_mangle]
extern "C" fn Chat_Send() {}

#[cfg(test)]
#[no_mangle]
extern "C" fn ScheduledTask_Add() {}

#[cfg(test)]
#[no_mangle]
static mut Entities: () = ();

#[cfg(test)]
#[no_mangle]
static mut Camera: () = ();

#[ignore]
#[test]
fn test_youtube_search() {
    async_manager::initialize();

    async_manager::block_on_local(async {
        println!("{:#?}", search("nyan").await);
    });
}

#[ignore]
#[test]
fn test_youtube_video() {
    async_manager::initialize();

    async_manager::block_on_local(async {
        println!("{:#?}", video("QH2-TGUlwu4").await);
    });
}
