use anyhow::{anyhow, Result};
use bevy::prelude::*;
use bevy_wasm_tasks::*;
use image::{self, DynamicImage};
use reqwest::Client;
use std::io::Cursor;

#[derive(Default)]
pub struct FetchPlugin;

#[derive(Event)]
pub enum FetchRequest {
    GetString(String),
    PostString(String, String),
    GetImage(String),
}

#[derive(Event)]
pub enum FetchResponse {
    Str(String),
    Image(DynamicImage),
}

impl Plugin for FetchPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FetchRequest>()
            .add_event::<FetchResponse>()
            .add_systems(Update, handle_fetch_request);
    }
}

fn handle_fetch_request(mut er_fetch_request: EventReader<FetchRequest>, tasks: Tasks) {
    for er in er_fetch_request.read() {
        let link = match er {
            FetchRequest::GetString(link) => link.clone(),
            FetchRequest::GetImage(link) => link.clone(),
            FetchRequest::PostString(link, _) => link.clone(),
        };
        let frag_fetch_string = match er {
            FetchRequest::GetString(_) => true,
            FetchRequest::GetImage(_) => false,
            FetchRequest::PostString(_, _) => true,
        };
        let get_or_post = match er {
            FetchRequest::GetString(_) => true,
            FetchRequest::GetImage(_) => true,
            FetchRequest::PostString(_, _) => false,
        };
        let payload = match er {
            FetchRequest::GetString(_) => "".to_string(),
            FetchRequest::GetImage(_) => "".to_string(),
            FetchRequest::PostString(_, payload) => payload.clone(),
        };

        // TODO: DEDUP
        #[cfg(not(target_arch = "wasm32"))]
        tasks.spawn_tokio(move |ctx| async move {
            if frag_fetch_string {
                let response = match get_or_post {
                    true => get_string(link.clone()).await,
                    false => post_string(link.clone(), payload.clone()).await,
                };
                if response.is_ok() {
                    let text = response.unwrap();
                    ctx.run_on_main_thread(move |ctx| {
                        let world: &mut World = ctx.world;
                        world.send_event(FetchResponse::Str(text.clone()));
                    })
                    .await;
                } else {
                    panic!("error: {:?}", response.err());
                }
            } else {
                let response = fetch_image(link.clone()).await;
                if response.is_ok() {
                    let image = response.unwrap();
                    ctx.run_on_main_thread(move |ctx| {
                        let world: &mut World = ctx.world;
                        world.send_event(FetchResponse::Image(image));
                    })
                    .await;
                } else {
                    panic!("error: {:?}", response.err());
                }
            }
        });

        #[cfg(target_arch = "wasm32")]
        tasks.spawn_wasm(move |ctx| async move {
            if frag_fetch_string {
                let response = match get_or_post {
                    true => get_string(link.clone()).await,
                    false => post_string(link.clone(), payload.clone()).await,
                };
                if response.is_ok() {
                    let text = response.unwrap();
                    ctx.run_on_main_thread(move |ctx| {
                        let world: &mut World = ctx.world;
                        world.send_event(FetchResponse::Str(text.clone()));
                    })
                    .await;
                } else {
                    panic!("error: {:?}", response.err());
                }
            } else {
                let response = fetch_image(link.clone()).await;
                if response.is_ok() {
                    let image = response.unwrap();
                    ctx.run_on_main_thread(move |ctx| {
                        let world: &mut World = ctx.world;
                        world.send_event(FetchResponse::Image(image));
                    })
                    .await;
                } else {
                    panic!("error: {:?}", response.err());
                }
            }
        });
    }
}

async fn get_string(link: String) -> Result<String> {
    let client = Client::new();
    let response = client.get(link).send().await?;
    let response_text = response.text().await?;

    Ok(response_text)
}

async fn post_string(url: String, payload: String) -> Result<String> {
    let client = Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(payload)
        .send()
        .await?;
    let response_text = response.text().await?;

    Ok(response_text)
}

async fn fetch_image(url: String) -> Result<DynamicImage> {
    let response = reqwest::get(url).await?;

    if response.status().is_success() {
        let image_bytes = response.bytes().await?;
        let img: DynamicImage = image::load(Cursor::new(image_bytes), image::ImageFormat::Jpeg)?;
        Ok(img)
    } else {
        Err(anyhow!(format!(
            "Failed to download image: {}",
            response.status()
        )))
    }
}
