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
struct TrackItem {
    track: Song,
}

pub async fn playback_show(
    auth: &mut SpotifyAuth,
    show_playlist: bool,
) -> Result<(), Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/player".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client.get(url).headers(headers.clone()).send().await?;

    let response: PlayerResponse = serde_json::from_str(res.text().await?.as_str())?;

    println!("Current song: {}", response.song);
    if !response.is_playing {
        println!("(paused)");
    }
    println!("Running on:   {}", response.device);

    if show_playlist && response.context.is_some() {
        let ctx = response.context.unwrap();
        let playlist_res = client.get(ctx.href).headers(headers).send().await?;
        let playlist_response: PlaylistDescription =
            serde_json::from_str(playlist_res.text().await?.as_str())?;

        println!("Playing from: {} ({})", playlist_response.name, ctx.r#type);

        if let Some(desc) = playlist_response.description {
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

    let response = res.text().await?;

    #[cfg(debug_assertions)]
    let response_str = response.as_str();
    #[cfg(debug_assertions)]
    println!("{response_str}");

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
        map.insert("offset".to_string(), serde_json::Value::Object(tmp.into()));
    }
    if map.is_empty() {
        res_builder = res_builder.header("content-length", 0);
    } else {
        res_builder = res_builder.json(&map);
    }
    let res = res_builder.send().await?;

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

#[derive(Deserialize, Debug)]
struct PlaylistListResponse {
    #[allow(dead_code)]
    next: Option<String>,
    items: Vec<Playlist>,
}

impl Display for PlaylistListResponse {
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
        let n = self.items.len();
        for (ind, track) in self.items.iter().take(n - 1).enumerate() {
            if track.track.name == highlight {
                println!("\x1b[93m#{ind} {}\x1b[0m", track.track);
            } else {
                println!("#{ind} {}", track.track);
            }
        }
        if let Some(last) = self.items.last() {
            if last.track.name == highlight {
                println!("\x1b[93m#{} {}\x1b[0m", n - 1, last.track);
            } else {
                println!("#{} {}", n - 1, last.track);
            }
        }
    }
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

    let response: PlaylistListResponse = serde_json::from_str(res.text().await?.as_str())?;

    println!("{response}");

    Ok(())
}

pub async fn get_current_playlist_uri(
    auth: &mut SpotifyAuth,
) -> Result<String, Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/player".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client.get(url).headers(headers.clone()).send().await?;

    let response: PlayerResponse = serde_json::from_str(res.text().await?.as_str())?;

    match response.context {
        Some(ctx) => Ok(ctx.uri),
        None => Err("Not playing from a playlist.".to_string().into()),
    }
}

pub async fn playlist_current(auth: &mut SpotifyAuth) -> Result<(), Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/player".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client.get(url).headers(headers.clone()).send().await?;

    let response: PlayerResponse = serde_json::from_str(res.text().await?.as_str())?;

    let current_song = response.song.name;

    match response.context {
        Some(ctx) => {
            let playlist_res = client.get(ctx.href).headers(headers).send().await?;

            let playlist_response: PlaylistDescription =
                serde_json::from_str(playlist_res.text().await?.as_str())?;

            println!("{}", playlist_response.name);

            if let Some(desc) = playlist_response.description {
                if !desc.is_empty() {
                    println!(" - {}", desc);
                }
            }

            // TODO: pagination. `tracks.items` will "only" have the first 100 tracks;
            // the rest need to be fetched using `tracks.next` URIs until it's None.
            if let Some(tracks) = playlist_response.tracks {
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
