use classicube_helpers::async_manager;
use serde::Deserialize;
use tracing::debug;

use crate::error::{bail, Error, Result};

const API_URL: &str = "https://youtube-api.spiralp.xyz";
const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Deserialize)]
struct ApiError {
    #[allow(dead_code)]
    code: u64,
    message: String,
}

#[derive(Debug, Deserialize)]
pub struct VideoResponse {
    pub title: String,
    pub duration_seconds: u64,
}

fn make_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
        .unwrap()
}

pub async fn video(id: &str) -> Result<VideoResponse> {
    let id = id.to_string();

    let result = async_manager::spawn(async move {
        let client = make_client();
        let bytes = client
            .get(&format!("{API_URL}/video/{id}"))
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

#[tracing::instrument]
pub async fn playlist(id: &str) -> Result<Vec<String>> {
    let id = id.to_string();

    let result = async_manager::spawn(async move {
        let client = make_client();
        let bytes = client
            .get(&format!("{API_URL}/playlist/{id}"))
            .send()
            .await?
            .bytes()
            .await?;

        if let Ok(error) = serde_json::from_slice::<ApiError>(&bytes) {
            bail!("ApiError: {}", error.message);
        } else {
            Ok::<_, Error>(serde_json::from_slice::<Vec<String>>(&bytes)?)
        }
    })
    .await??;

    debug!("{:?}", result);

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
        let client = make_client();
        let bytes = client
            .get(&format!("{API_URL}/search"))
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

#[test]
#[ignore]
fn test_youtube_search() {
    crate::logger::initialize(true, None, false);
    async_manager::initialize();

    async_manager::spawn_local_on_main_thread(async {
        println!("{:#?}", search("nyan").await.unwrap());
    });

    async_manager::run();
    async_manager::shutdown();
}

#[test]
#[ignore]
fn test_youtube_video() {
    crate::logger::initialize(true, None, false);
    async_manager::initialize();

    async_manager::spawn_local_on_main_thread(async {
        println!("{:#?}", video("whBoLspQSqQ").await.unwrap());
    });

    async_manager::run();
    async_manager::shutdown();
}
