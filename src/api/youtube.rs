use crate::{async_manager, error::*};
use serde::Deserialize;
use tracing::*;

const API_URL: &str = "http://youtube.spiralp.uk.to:43210";
const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

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

#[tracing::instrument]
pub async fn playlist(id: &str) -> Result<Vec<String>> {
    let id = id.to_string();

    let result = async_manager::spawn(async move {
        let client = make_client();
        let bytes = client
            .get(&format!("{}/playlist/{}", API_URL, id))
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
static mut Entities: () = ();

#[cfg(test)]
#[no_mangle]
static mut Camera: () = ();

macro_rules! test_noop {
    ($name:tt) => {
        #[cfg(test)]
        #[no_mangle]
        pub extern "C" fn $name() {}
    };
}

test_noop!(Entity_SetModel);
test_noop!(Options_Get);
test_noop!(Options_Set);
test_noop!(Chat_Send);
test_noop!(ScheduledTask_Add);
test_noop!(Chat_AddOf);
test_noop!(Chat_Add);
test_noop!(Gfx_CreateTexture);
test_noop!(Gfx_DeleteTexture);

#[ignore]
#[test]
fn test_youtube_search() {
    crate::logger::initialize(true, false, false);
    crate::async_manager::initialize();

    async_manager::block_on_local(async {
        println!("{:#?}", search("nyan").await.unwrap());
    });
}

#[ignore]
#[test]
fn test_youtube_video() {
    crate::logger::initialize(true, false, false);
    crate::async_manager::initialize();

    async_manager::block_on_local(async {
        println!("{:#?}", video("QH2-TGUlwu4").await.unwrap());
    });
}
