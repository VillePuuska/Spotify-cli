use super::auth::SpotifyAuth;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use std::error;

async fn auth_header(auth: &mut SpotifyAuth) -> Result<HeaderMap, Box<dyn error::Error>> {
    let access_token = auth.get_access_token().await?;
    let authorization_value = format!("Bearer {}", access_token);
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&authorization_value)?,
    );
    Ok(headers)
}

#[derive(Deserialize, Debug)]
struct Album {
    name: String,
    artists: Vec<Artist>,
}

#[derive(Deserialize, Debug)]
struct Artist {
    name: String,
}

#[derive(Deserialize, Debug)]
struct Song {
    album: Option<Album>,
    name: String,
    artists: Vec<Artist>,
}

#[derive(Deserialize, Debug)]
struct Device {
    name: String,
    r#type: String,
}

#[derive(Deserialize, Debug)]
struct PlayerResponse {
    device: Device,
    #[serde(rename(deserialize = "item"))]
    song: Song,
    is_playing: bool,
}

pub async fn playback_show(auth: &mut SpotifyAuth) -> Result<(), Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/player".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client.get(url).headers(headers).send().await?;

    let response: PlayerResponse = serde_json::from_str(res.text().await?.as_str())?;

    println!("{:#?}", response.device);
    println!("{:#?}", response.song);
    println!("Playing? {}", response.is_playing);

    Ok(())
}

pub async fn playback_pause(auth: &mut SpotifyAuth) -> Result<(), Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/player/pause".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client
        .put(url)
        .headers(headers)
        .header("content-length", 0)
        .send()
        .await?;

    let response = res.text().await?;

    #[cfg(debug_assertions)]
    let response_str = response.as_str();
    #[cfg(debug_assertions)]
    println!("{response_str}");

    Ok(())
}

pub async fn playback_play(auth: &mut SpotifyAuth) -> Result<(), Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/player/play".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client
        .put(url)
        .headers(headers)
        .header("content-length", 0)
        .send()
        .await?;

    let response = res.text().await?;

    #[cfg(debug_assertions)]
    let response_str = response.as_str();
    #[cfg(debug_assertions)]
    println!("{response_str}");

    Ok(())
}

pub async fn playback_next(auth: &mut SpotifyAuth) -> Result<(), Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/player/next".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client
        .post(url)
        .headers(headers)
        .header("content-length", 0)
        .send()
        .await?;

    let response = res.text().await?;

    #[cfg(debug_assertions)]
    let response_str = response.as_str();
    #[cfg(debug_assertions)]
    println!("{response_str}");

    Ok(())
}

pub async fn playback_previous(auth: &mut SpotifyAuth) -> Result<(), Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/player/previous".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client
        .post(url)
        .headers(headers)
        .header("content-length", 0)
        .send()
        .await?;

    let response = res.text().await?;

    #[cfg(debug_assertions)]
    let response_str = response.as_str();
    #[cfg(debug_assertions)]
    println!("{response_str}");

    Ok(())
}

pub async fn playback_restart(auth: &mut SpotifyAuth) -> Result<(), Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/player/seek".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client
        .put(url)
        .query(&[("position_ms", 0)])
        .headers(headers)
        .header("content-length", 0)
        .send()
        .await?;

    let response = res.text().await?;

    #[cfg(debug_assertions)]
    let response_str = response.as_str();
    #[cfg(debug_assertions)]
    println!("{response_str}");

    Ok(())
}

#[derive(Deserialize, Debug)]
struct PlayerQueueResponse {
    #[serde(rename(deserialize = "currently_playing"))]
    current: Song,
    #[serde(rename(deserialize = "queue"))]
    queued: Vec<Song>,
}

pub async fn queue_show(
    auth: &mut SpotifyAuth,
    number: usize,
) -> Result<(), Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/player/queue".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client.get(url).headers(headers).send().await?;

    let response: PlayerQueueResponse = serde_json::from_str(res.text().await?.as_str())?;

    println!("{:#?}", response.current);
    if number > 1 {
        println!(
            "{:#?}",
            response
                .queued
                .iter()
                .take(number - 1)
                .collect::<Vec<&Song>>()
        );
    }

    Ok(())
}
