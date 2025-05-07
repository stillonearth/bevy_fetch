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
    Str(String),
    Image(String),
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
            FetchRequest::Str(link) => link.clone(),
            FetchRequest::Image(link) => link.clone(),
        };
        let frag_fetch_string = match er {
            FetchRequest::Str(_) => true,
            FetchRequest::Image(_) => false,
        };

        // TODO: DEDUP
        #[cfg(not(target_arch = "wasm32"))]
        tasks.spawn_tokio(move |ctx| async move {
            if frag_fetch_string {
                let response = fetch_string(link.clone()).await;
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
                let response = fetch_string(link.clone()).await;
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

async fn fetch_string(link: String) -> Result<String> {
    let client = Client::new();
    let response = client.get(link).send().await?;
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
