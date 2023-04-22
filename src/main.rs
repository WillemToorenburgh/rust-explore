use core::panic;
use std::str::FromStr;

use reqwest::StatusCode;
use serde::Deserialize;
use tracing::{info, Level};
use tracing_subscriber::{filter::Targets, layer::SubscriberExt, util::SubscriberInitExt};

use axum::{
    body::BoxBody,
    http::header,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

#[tokio::main]
async fn main() {
    let filter = Targets::from_str(std::env::var("RUST_LOG").as_deref().unwrap_or("info"))
        .expect("RUST_LOG should be a valid tracing filter (text key-values of <target>=<level>)");
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .json()
        .finish()
        .with(filter)
        .init();

    let app = Router::new()
        .route("/", get(root_get))
        .route("/panic", get(|| async { panic!("YEET") }));

    let server_address = "0.0.0.0:8080".parse().unwrap();
    info!("Listening on {server_address}");
    axum::Server::bind(&server_address)
        .serve(app.into_make_service())
        .await
        .unwrap();

    let art = get_cat_ascii_art().await.unwrap();
    println!("{art}");
}

async fn root_get() -> Response {
    match get_cat_ascii_art().await {
        Ok(art) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            art,
        )
            .into_response(),

        Err(e) => {
            println!("Encountered error: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Encountered error").into_response()
        }
    }
}

async fn get_cat_ascii_art() -> color_eyre::Result<String> {
    #[derive(Deserialize)]
    struct CatImage {
        url: String,
    }

    let api_url = "https://api.thecatapi.com/v1/images/search";
    let client = reqwest::Client::default();

    let image = client
        .get(api_url)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<CatImage>>()
        .await?
        .pop()
        .ok_or_else(|| color_eyre::eyre::eyre!("The cat API did not return an image"))?;

    let image_bytes = client
        .get(image.url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec();

    let image = image::load_from_memory(&image_bytes)?;
    let ascii_art = artem::convert(
        image,
        artem::options::OptionBuilder::new()
            .target(artem::options::TargetType::HtmlFile(true, true))
            .build(),
    );

    Ok(ascii_art)
}
