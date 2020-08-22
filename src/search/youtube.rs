use crate::{async_manager, error::*};
use invidious::api::search;
use log::debug;

pub async fn search(input: &str) -> Result<String> {
    let input = input.to_string();

    async_manager::spawn(async move {
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

#[test]
fn test_search_youtube() {
    async_manager::initialize();

    async_manager::block_on_local(async {
        println!("{:#?}", search("nyan").await);
    });
}
