use super::auth::SpotifyAuth;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use std::{error, fmt::Display};

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
    // artists: Vec<Artist>,
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

impl Display for Song {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let artists_str = if !self.artists.is_empty() {
            let tmp = self
                .artists
                .iter()
                .fold("".to_string(), |acc, x| acc + ", " + &x.name);
            tmp.strip_prefix(", ").unwrap().to_string()
        } else {
            "unknown artist".to_string()
        };

        match &self.album {
            Some(album) => write!(
                f,
                "{} - {} [from the album: {}]",
                self.name, artists_str, album.name
            ),
            None => write!(f, "{} - {}", self.name, artists_str),
        }
    }
}

#[derive(Deserialize, Debug)]
struct Device {
    name: String,
    r#type: String,
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.r#type)
    }
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

    println!("Current song: {}", response.song);
    if !response.is_playing {
        println!("(paused)");
    }
    println!("Device:       {}", response.device);

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

    println!("Currently playing: {}", response.current);
    if number > 1 {
        let digits = number.to_string().len();
        for (ind, song) in response.queued.iter().take(number - 1).enumerate() {
            let start = format!(
                "#{}{} in queue:       ",
                ind + 1,
                " ".repeat(digits - (ind + 1).to_string().len())
            );
            let (start, _) = start.split_at(19);
            println!("{}{}", start, song);
        }
    }

    Ok(())
}
