use super::auth::SpotifyAuth;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    StatusCode,
};
use serde::Deserialize;
use serde_json::Value;
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
    is_playable: Option<bool>,
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
    context: Option<Context>,
}

#[derive(Deserialize, Debug)]
struct Context {
    r#type: String,
    href: String,
    uri: String,
}

#[derive(Deserialize, Debug)]
struct PlaylistDescription {
    name: String,
    description: Option<String>,
    tracks: Option<PlaylistTracks>,
}

#[derive(Deserialize, Debug)]
struct PlaylistResponse {
    #[allow(dead_code)]
    next: Option<String>,
    items: Vec<Playlist>,
}

impl Display for PlaylistResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.items.len();
        for playlist in self.items.iter().take(n - 1) {
            writeln!(f, "{playlist}\n")?;
        }
        if let Some(last) = self.items.last() {
            write!(f, "{}", last)
        } else {
            Ok(())
        }
    }
}

#[derive(Deserialize, Debug)]
struct Playlist {
    description: Option<String>,
    #[allow(dead_code)]
    href: String,
    uri: String,
    name: String,
    tracks: TracksLink,
    public: Option<bool>,
}

impl Display for Playlist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;

        if let Some(true) = &self.public {
            write!(f, " (public)")?;
        } else if let Some(false) = &self.public {
            write!(f, " (private)")?;
        }

        if let Some(desc) = &self.description {
            write!(f, ": {}", desc)?;
        }

        write!(f, "({} tracks) uri: {}", self.tracks.total, self.uri)
    }
}

#[derive(Deserialize, Debug)]
struct TracksLink {
    #[allow(dead_code)]
    href: String,
    total: u16,
}

#[derive(Deserialize, Debug)]
struct PlaylistTracks {
    #[allow(dead_code)]
    next: Option<String>,
    items: Vec<TrackItem>,
}

impl Display for PlaylistTracks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.items.len();
        for (ind, track) in self.items.iter().take(n - 1).enumerate() {
            writeln!(f, "#{ind} {}", track.track)?;
        }
        if let Some(last) = self.items.last() {
            write!(f, "#{} {}", n - 1, last.track)
        } else {
            Ok(())
        }
    }
}

impl PlaylistTracks {
    pub fn print_tracks(&self, highlight: &str) {
        let tracks: Vec<&TrackItem> = self
            .items
            .iter()
            .filter(|track| track.track.is_playable != Some(false))
            .collect();
        let n = tracks.len();
        for (ind, track) in tracks.iter().take(n - 1).enumerate() {
            if track.track.name == highlight {
                println!("\x1b[93m#{ind} {}\x1b[0m", track.track);
            } else {
                println!("#{ind} {}", track.track);
            }
        }
        if let Some(last) = tracks.last() {
            if last.track.name == highlight {
                println!("\x1b[93m#{} {}\x1b[0m", n - 1, last.track);
            } else {
                println!("#{} {}", n - 1, last.track);
            }
        }
    }
}

#[derive(Deserialize, Debug)]
struct TrackItem {
    track: Song,
}

#[derive(Deserialize, Debug)]
struct PlayerQueueResponse {
    #[serde(rename(deserialize = "currently_playing"))]
    current: Option<Song>,
    #[serde(rename(deserialize = "queue"))]
    queued: Vec<Song>,
}

async fn get_player(auth: &mut SpotifyAuth) -> Result<PlayerResponse, Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/player".to_string();

    let headers = auth_header(auth).await?;
    let client = reqwest::Client::new();

    let res = client.get(url).headers(headers.clone()).send().await?;

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    if res.status() == StatusCode::NO_CONTENT {
        return Err("No active devices.".into());
    }

    let response_text = res.text().await?;
    let player_response: PlayerResponse =
        serde_json::from_str(&response_text).map_err(|_| response_text)?;

    Ok(player_response)
}

async fn get_playlist_from_href(
    auth: &mut SpotifyAuth,
    href: &str,
) -> Result<PlaylistDescription, Box<dyn error::Error>> {
    // TODO: pagination. `tracks.items` will "only" have the first 100 tracks;
    // the rest need to be fetched using `tracks.next` URIs until it's None.
    // Need to add a param to specify if all tracks are actually needed/wanted.

    let headers = auth_header(auth).await?;
    let client = reqwest::Client::new();

    let res = client
        .get(href)
        .headers(headers)
        .query(&[("market", "from_token")])
        .send()
        .await?;

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    let response_text = res.text().await?;
    let playlist_description: PlaylistDescription =
        serde_json::from_str(&response_text).map_err(|_| response_text)?;

    Ok(playlist_description)
}

pub async fn playback_show(
    auth: &mut SpotifyAuth,
    show_playlist: bool,
) -> Result<(), Box<dyn error::Error>> {
    let player_response = get_player(auth).await?;

    println!("Current song: {}", player_response.song);
    if !player_response.is_playing {
        println!("(paused)");
    }
    println!("Running on:   {}", player_response.device);

    if show_playlist && player_response.context.is_some() {
        let ctx = player_response.context.unwrap();

        let playlist_description = get_playlist_from_href(auth, &ctx.href).await?;

        println!(
            "Playing from: {} ({})",
            playlist_description.name, ctx.r#type
        );

        if let Some(desc) = playlist_description.description {
            if !desc.is_empty() {
                println!(" - {}", desc);
            }
        }
    };

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

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    #[cfg(debug_assertions)]
    let response = res.text().await?;
    #[cfg(debug_assertions)]
    println!("{response}");

    Ok(())
}

pub async fn playback_play(
    auth: &mut SpotifyAuth,
    uri: Option<&str>,
    index: Option<u8>,
) -> Result<(), Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/player/play".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let mut res_builder = client.put(url).headers(headers);
    let mut map = serde_json::Map::new();
    if let Some(uri) = uri {
        map.insert(
            "context_uri".to_string(),
            serde_json::Value::String(uri.to_owned()),
        );
    }
    if let Some(offset) = index {
        let mut tmp = serde_json::Map::new();
        tmp.insert(
            "position".to_string(),
            serde_json::Value::Number(offset.into()),
        );
        map.insert("offset".to_string(), serde_json::Value::Object(tmp));

        if uri.is_none() {
            let player_response = get_player(auth).await?;
            match player_response.context {
                Some(ctx) => {
                    if ctx.r#type != "playlist" {
                        return Err("Not playing from a playlist; can't jump to an index.".into());
                    }
                    map.insert(
                        "context_uri".to_string(),
                        serde_json::Value::String(ctx.uri.to_owned()),
                    );
                }
                None => return Err("Not playing from a playlist; can't jump to an index.".into()),
            }
        }
    }

    if map.is_empty() {
        res_builder = res_builder.header("content-length", 0);
    } else {
        res_builder = res_builder.json(&map);
    }
    let res = res_builder.send().await?;

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    #[cfg(debug_assertions)]
    let response = res.text().await?;
    #[cfg(debug_assertions)]
    println!("{response}");

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

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    #[cfg(debug_assertions)]
    let response = res.text().await?;
    #[cfg(debug_assertions)]
    println!("{response}");

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

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    #[cfg(debug_assertions)]
    let response = res.text().await?;
    #[cfg(debug_assertions)]
    println!("{response}");

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

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    #[cfg(debug_assertions)]
    let response = res.text().await?;
    #[cfg(debug_assertions)]
    println!("{response}");

    Ok(())
}

pub async fn queue_show(
    auth: &mut SpotifyAuth,
    number: usize,
) -> Result<(), Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/player/queue".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client.get(url).headers(headers).send().await?;

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    let response_text = res.text().await?;

    let player_queue_response: PlayerQueueResponse =
        serde_json::from_str(&response_text).map_err(|_| response_text)?;

    if player_queue_response.current.is_none() {
        return Err("Not playing anything currently.".into());
    }

    let current = player_queue_response.current.unwrap();
    println!("Currently playing: {}", current);
    if number > 1 {
        let digits = number.to_string().len();
        for (ind, song) in player_queue_response
            .queued
            .iter()
            .take(number - 1)
            .enumerate()
        {
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

pub async fn playlist_list(auth: &mut SpotifyAuth) -> Result<(), Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/playlists".to_string();

    let headers = auth_header(auth).await?;

    // TODO: pagination. Do I _actually_ care? When would I ever have >50 playlists created&liked?
    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .headers(headers)
        .query(&[("limit", 50)])
        .send()
        .await?;

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    let response_text = res.text().await?;
    let playlist_response: PlaylistResponse =
        serde_json::from_str(&response_text).map_err(|_| response_text)?;

    println!("{playlist_response}");

    Ok(())
}

pub async fn playlist_current(auth: &mut SpotifyAuth) -> Result<(), Box<dyn error::Error>> {
    let player_response = get_player(auth).await?;

    let current_song = player_response.song.name;

    match player_response.context {
        Some(ctx) => {
            let playlist_description = get_playlist_from_href(auth, &ctx.href).await?;

            println!("{}", playlist_description.name);

            if let Some(desc) = playlist_description.description {
                if !desc.is_empty() {
                    println!(" - {}", desc);
                }
            }

            if let Some(tracks) = playlist_description.tracks {
                println!();
                tracks.print_tracks(&current_song);
            } else {
                println!("\nNot actually playing from a playlist currently.")
            }
            // TODO: maybe add a param to print all vs only some number of tracks _around_
            // the current track?
        }
        None => println!("Not playing from a playlist currently."),
    }

    Ok(())
}
