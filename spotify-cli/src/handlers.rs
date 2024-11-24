use super::auth::SpotifyAuth;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{env, error, fmt::Display, io, time::Duration};

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
    id: String,
}

impl Display for Artist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Deserialize, Debug)]
struct Song {
    album: Option<Album>,
    name: String,
    id: String,
    uri: String,
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
    total: u16,
}

#[derive(Deserialize, Debug)]
struct PlaylistTracks {
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
    pub async fn print_tracks(
        self,
        auth: &mut SpotifyAuth,
        highlight: Option<&str>,
        max_lines: Option<u16>,
    ) -> Result<(), Box<dyn error::Error>> {
        if let Some(0) = max_lines {
            return Ok(());
        }

        let tracks: Vec<Song> = self.get_tracks(auth).await?;

        let mut first_ind = 0;
        let mut last_ind = tracks.len() as i32;
        let mut highlight_ind = None;

        if let Some(name) = highlight {
            for (ind, track) in tracks.iter().enumerate() {
                if track.name == name {
                    highlight_ind = Some(ind);
                    if let Some(max_lines) = max_lines {
                        first_ind = ind as i32 - ((max_lines as i32 - 1) / 2);
                        last_ind = ind as i32 + (max_lines as i32 / 2);
                    }
                    break;
                }
            }
            if highlight_ind.is_none() {
                return Err("Could not find the song to highlight.".into());
            }
        } else if let Some(max_lines) = max_lines {
            last_ind = (max_lines - 1) as i32;
        }

        let n = tracks.len();
        for (ind, track) in tracks.iter().take(n - 1).enumerate() {
            if (ind as i32) < first_ind || (ind as i32) > last_ind {
                continue;
            }
            if highlight_ind.is_some() && ind == highlight_ind.unwrap() {
                println!("\x1b[93m#{ind} {}\x1b[0m", track);
            } else {
                println!("#{ind} {}", track);
            }
        }
        if let Some(last) = tracks.last() {
            if ((n - 1) as i32) < first_ind || ((n - 1) as i32) > last_ind {
            } else if highlight.is_some() && last.name == highlight.unwrap() {
                println!("\x1b[93m#{} {}\x1b[0m", n - 1, last);
            } else {
                println!("#{} {}", n - 1, last);
            }
        }

        Ok(())
    }

    pub async fn get_tracks(
        self,
        auth: &mut SpotifyAuth,
    ) -> Result<Vec<Song>, Box<dyn error::Error>> {
        let mut tracks: Vec<Song> = self
            .items
            .into_iter()
            .map(|track| track.track)
            .filter(|track| track.is_playable != Some(false))
            .collect();

        let mut next = self.next.clone();
        while let Some(url) = next {
            let headers = auth_header(auth).await?;
            let client = reqwest::Client::new();

            let res = client.get(url).headers(headers).send().await?;

            if res.error_for_status_ref().is_err() {
                let response_text = res.text().await?;
                let response_parsed: Value = serde_json::from_str(&response_text)?;
                return Err(response_parsed["error"]["message"].as_str().unwrap().into());
            }

            let response_text = res.text().await?;
            let playlist_tracks: PlaylistTracks =
                serde_json::from_str(&response_text).map_err(|_| response_text)?;

            let mut more_tracks: Vec<Song> = playlist_tracks
                .items
                .into_iter()
                .map(|track| track.track)
                .filter(|track| track.is_playable != Some(false))
                .collect();

            tracks.append(&mut more_tracks);

            next = playlist_tracks.next;
        }

        Ok(tracks)
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

#[derive(Deserialize, Debug)]
struct User {
    id: String,
}

#[derive(Deserialize, Debug)]
struct PlaylistCreateResponse {
    id: String,
}

#[derive(Deserialize, Debug)]
struct FindResponse {
    tracks: Option<TracksObject>,
    artists: Option<ArtistsObject>,
}

#[derive(Deserialize, Debug)]
struct TracksObject {
    items: Vec<Song>,
}

#[derive(Deserialize, Debug)]
struct ArtistsObject {
    items: Vec<Artist>,
}

#[derive(Deserialize, Debug)]
struct TrackOrArtist {
    name: String,
    id: String,
}

#[derive(Deserialize, Debug)]
struct RecommendationResponse {
    tracks: Vec<Song>,
}

#[derive(Deserialize, Debug, Default, Serialize)]
struct RecommendationParameters {
    limit: u8,
    artists: Vec<String>,
    seed_artists: Vec<String>,
    genres: Vec<String>,
    tracks: Vec<String>,
    seed_tracks: Vec<String>,
}

impl Display for RecommendationParameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Limit:   {:?}", self.limit)?;
        writeln!(f, "Artists: {:?}", self.artists)?;
        #[cfg(debug_assertions)]
        writeln!(f, "A ids:   {:?}", self.seed_artists)?;
        writeln!(f, "Genres:  {:?}", self.genres)?;
        writeln!(f, "Tracks:  {:?}", self.tracks)?;
        #[cfg(debug_assertions)]
        writeln!(f, "T ids:   {:?}", self.seed_tracks)?;

        Ok(())
    }
}

#[derive(Deserialize, Debug)]
struct GenresResponse {
    genres: Vec<String>,
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

async fn get_playlist_from_id(
    auth: &mut SpotifyAuth,
    id: &str,
) -> Result<PlaylistDescription, Box<dyn error::Error>> {
    let url = format!("https://api.spotify.com/v1/playlists/{id}");

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client
        .get(url)
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
    index: Option<u16>,
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
    // Could actually just implement this in the Display impl since `playlist_response` is not even
    // returned; it's just printed.
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

pub async fn playlist_current(
    auth: &mut SpotifyAuth,
    max_lines: Option<u16>,
) -> Result<(), Box<dyn error::Error>> {
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
                tracks
                    .print_tracks(auth, Some(&current_song), max_lines)
                    .await?;
            } else {
                println!("\nNot actually playing from a playlist currently.")
            }
        }
        None => println!("Not playing from a playlist currently."),
    }

    Ok(())
}

fn get_managed_playlist_id() -> Result<String, Box<dyn error::Error>> {
    env::var("SPOTIFY_CLI_MANAGED_PLAYLIST_ID")
        .map_err(|_| "The env variable SPOTIFY_CLI_MANAGED_PLAYLIST_ID is not set. If a managed playlist has not been created yet, run 'recommendation init'; if it has been created then set the env variable with the id of the playlist.".into())
}

pub async fn recommendation_show(
    auth: &mut SpotifyAuth,
    max_lines: Option<u16>,
) -> Result<(), Box<dyn error::Error>> {
    let managed_list = get_managed_playlist_id()?;

    let playlist_description = get_playlist_from_id(auth, &managed_list).await?;

    println!("{}", playlist_description.name);

    if let Some(desc) = playlist_description.description {
        if !desc.is_empty() {
            println!(" - {}", desc);
        }
    }

    if let Some(tracks) = playlist_description.tracks {
        println!();
        tracks.print_tracks(auth, None, max_lines).await?;
    } else {
        println!("\nNo songs in the list.");
    }

    Ok(())
}

pub async fn recommendation_play(
    auth: &mut SpotifyAuth,
    index: Option<u16>,
) -> Result<(), Box<dyn error::Error>> {
    let managed_list = get_managed_playlist_id()?;

    playback_play(
        auth,
        Some(&format!("spotify:playlist:{managed_list}")),
        index,
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(500u64)).await;
    playback_show(auth, false).await
}

pub async fn recommendation_save(
    auth: &mut SpotifyAuth,
    name: String,
    description: Option<String>,
) -> Result<(), Box<dyn error::Error>> {
    let managed_list = get_managed_playlist_id()?;
    let playlist_description = get_playlist_from_id(auth, &managed_list).await?;

    if playlist_description.tracks.is_none() {
        return Err("No tracks in the current managed playlist.".into());
    }

    let tracks = playlist_description
        .tracks
        .unwrap()
        .get_tracks(auth)
        .await?;

    let playlist_create_response = create_playlist(
        auth,
        &name,
        &description.unwrap_or(
            "Playlist created by a CLI tool to save a list of recommendations.".to_string(),
        ),
        false,
    )
    .await?;

    replace_playlist_items(auth, &playlist_create_response.id, &tracks).await
}

pub async fn recommendation_generate(auth: &mut SpotifyAuth) -> Result<(), Box<dyn error::Error>> {
    let managed_list = get_managed_playlist_id()?;

    let mut genres: Option<Vec<String>> = None;

    let mut recommendation_parameters = RecommendationParameters {
        limit: 20,
        ..Default::default()
    };

    let mut user_response: String = String::new();
    while !user_response.starts_with("q") {
        println!("\n***********************************\n");
        println!("Current parameters:\n{recommendation_parameters}\n");
        println!("What would you like to edit? (Enter the number of the option)");
        println!("1 - Change the limit/number of recommendations.");
        println!("2 - Add an artist.");
        println!("3 - Add a genre.");
        println!("4 - Add a track/song.");
        println!("7 - Clear artists.");
        println!("8 - Clear genres.");
        println!("9 - Clear tracks/songs.");
        println!("g - Generate recommendations.");
        println!("q - Quit without generating recommendations.");
        println!();

        user_response = String::new();
        io::stdin().read_line(&mut user_response)?;
        user_response = user_response.trim().to_lowercase();

        match user_response.as_str() {
            // TODO: implement all optional tuning knobs somehow
            "1" => {
                println!("New limit? (1-100)");
                let mut new_limit = String::new();
                io::stdin().read_line(&mut new_limit)?;
                let parsed_limit: Result<u8, _> = new_limit.trim().parse();
                match parsed_limit {
                    Ok(limit) => {
                        if limit == 0 || limit > 100 {
                            println!("Limit needs to be between 1-100.");
                        } else {
                            recommendation_parameters.limit = limit
                        }
                    }
                    Err(e) => println!("{e}"),
                }
            }
            "2" => {
                println!("Artist name?");
                let mut new_artist = String::new();
                io::stdin().read_line(&mut new_artist)?;
                new_artist = new_artist.trim().to_lowercase();
                println!();

                match find(auth, None, Some(&new_artist)).await {
                    Ok(artist) => {
                        recommendation_parameters.artists.push(artist.name);
                        recommendation_parameters.seed_artists.push(artist.id);
                    }
                    Err(e) => println!("{}", e),
                }
            }
            "3" => {
                if genres.is_none() {
                    genres = Some(get_available_genres(auth).await?);
                }
                println!(
                    "Available genres:\n{}\n",
                    genres.as_ref().unwrap().join(", ")
                );

                println!("Genre name?");
                let mut new_genre = String::new();
                io::stdin().read_line(&mut new_genre)?;
                new_genre = new_genre.trim().to_lowercase();

                if !genres.as_ref().unwrap().contains(&new_genre) {
                    println!("Illegal genre.");
                    continue;
                }

                recommendation_parameters.genres.push(new_genre);
            }
            "4" => {
                println!("Song name?");
                let mut new_track = String::new();
                io::stdin().read_line(&mut new_track)?;
                new_track = new_track.trim().to_lowercase();

                println!("\nDo you want to specify an artist? (Empty response if not)");
                let mut by_artist = String::new();
                io::stdin().read_line(&mut by_artist)?;
                by_artist = by_artist.trim().to_lowercase();
                println!();

                let artist: Option<&str> = if !by_artist.is_empty() {
                    Some(&by_artist)
                } else {
                    None
                };

                match find(auth, Some(&new_track), artist).await {
                    Ok(track) => {
                        recommendation_parameters.tracks.push(track.name);
                        recommendation_parameters.seed_tracks.push(track.id);
                    }
                    Err(e) => println!("{}", e),
                }
            }
            "7" => {
                recommendation_parameters.artists = Vec::new();
                recommendation_parameters.seed_artists = Vec::new();
            }
            "8" => {
                recommendation_parameters.genres = Vec::new();
            }
            "9" => {
                recommendation_parameters.tracks = Vec::new();
                recommendation_parameters.seed_tracks = Vec::new();
            }
            "g" => {
                let seeds = recommendation_parameters.seed_artists.len()
                    + recommendation_parameters.genres.len()
                    + recommendation_parameters.seed_tracks.len();
                if seeds == 0 {
                    println!("You need to specify at least one artist or genre or track.");
                    continue;
                }
                if seeds == 0 || seeds > 5 {
                    println!("Too many artists & genres & tracks ({seeds}) specified.");
                    println!("Can specify at most 5 in total.");
                    continue;
                }
                let songs = get_recommendations(auth, &recommendation_parameters).await?;

                println!("Got the following recommendations:");
                for song in songs.iter() {
                    println!("{song}");
                }

                println!("\nAccept this list or keep trying? (y to accept, N to keep trying)");
                let mut user_response = String::new();
                io::stdin().read_line(&mut user_response)?;
                user_response = user_response.trim().to_lowercase();

                if user_response.starts_with("y") {
                    replace_playlist_items(auth, &managed_list, &songs).await?;

                    println!("Added recommendations to the managed playlist.");
                    println!("Start playing the list? (Y/n)");
                    let mut user_response = String::new();
                    io::stdin().read_line(&mut user_response)?;
                    user_response = user_response.trim().to_lowercase();

                    if user_response.is_empty() || user_response.starts_with("y") {
                        recommendation_play(auth, None).await?;
                    }

                    break;
                } else {
                    println!("Ok, going again.");
                }
            }
            "q" => {
                println!("Ok, quitting without generating recommendations.");
                break;
            }
            _ => println!("Unrecognized command: {user_response}"),
        }
    }

    Ok(())
}

async fn get_available_genres(
    auth: &mut SpotifyAuth,
) -> Result<Vec<String>, Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/recommendations/available-genre-seeds".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client.get(url).headers(headers).send().await?;

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    let response_text = res.text().await?;
    let genres_response: GenresResponse =
        serde_json::from_str(&response_text).map_err(|_| response_text)?;

    Ok(genres_response.genres)
}

async fn replace_playlist_items(
    auth: &mut SpotifyAuth,
    playlist_id: &str,
    tracks: &[Song],
) -> Result<(), Box<dyn error::Error>> {
    let url = format!("https://api.spotify.com/v1/playlists/{playlist_id}/tracks");

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let uris: Vec<String> = tracks.iter().map(|song| song.uri.to_owned()).collect();
    let res = client
        .put(url)
        .headers(headers)
        .header("content-length", 0)
        .query(&[("uris", uris.join(","))])
        .send()
        .await?;

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    Ok(())
}

async fn get_recommendations(
    auth: &mut SpotifyAuth,
    params: &RecommendationParameters,
) -> Result<Vec<Song>, Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/recommendations".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let mut request_builder = client
        .get(url)
        .headers(headers)
        .query(&[("limit", params.limit)])
        .query(&[("market", "from_token")]);
    if !params.seed_artists.is_empty() {
        request_builder = request_builder.query(&[("seed_artists", params.seed_artists.join(","))])
    }
    if !params.genres.is_empty() {
        request_builder = request_builder.query(&[("seed_genres", params.genres.join(","))])
    }
    if !params.seed_tracks.is_empty() {
        request_builder = request_builder.query(&[("seed_tracks", params.seed_tracks.join(","))])
    }
    let res = request_builder.send().await?;

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    let response_text = res.text().await?;
    let recommendation_response: RecommendationResponse =
        serde_json::from_str(&response_text).map_err(|_| response_text)?;

    Ok(recommendation_response.tracks)
}

async fn find(
    auth: &mut SpotifyAuth,
    track: Option<&str>,
    artist: Option<&str>,
) -> Result<TrackOrArtist, Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/search".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let mut request_builder = client.get(url).headers(headers).query(&[("limit", 5)]);

    if let Some(track) = track {
        if let Some(artist) = artist {
            request_builder =
                request_builder.query(&[("q", format!("track:{track} artist:{artist}"))]);
        } else {
            request_builder = request_builder.query(&[("q", format!("track:{track}"))]);
        }
        request_builder = request_builder.query(&[("type", "track".to_string())]);
    } else if let Some(artist) = artist {
        request_builder = request_builder.query(&[
            ("q", format!("artist:{artist}")),
            ("type", "artist".to_string()),
        ]);
    } else {
        return Err(
            "You have to specify an artist or track. What are we going to search for otherwise?"
                .into(),
        );
    }
    let res = request_builder.send().await?;

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    let response_text = res.text().await?;
    let find_response: FindResponse =
        serde_json::from_str(&response_text).map_err(|_| response_text)?;

    if track.is_some() {
        match find_response.tracks {
            Some(t) => {
                if t.items.is_empty() {
                    return Err("Didn't find any tracks. Did you typo the song name?".into());
                }
                let ind = choose_element(&t.items)?;
                let found_track = t.items.get(ind as usize).ok_or("Index out of bounds!")?;
                Ok(TrackOrArtist {
                    name: found_track.name.clone(),
                    id: found_track.id.clone(),
                })
            }
            None => Err("Didn't find any tracks. Did you typo the song name?".into()),
        }
    } else {
        match find_response.artists {
            Some(a) => {
                if a.items.is_empty() {
                    return Err("Didn't find any artists. Did you typo the artists name?".into());
                }
                let ind = choose_element(&a.items)?;
                let found_artist = a.items.get(ind as usize).ok_or("Index out of bounds!")?;
                Ok(TrackOrArtist {
                    name: found_artist.name.clone(),
                    id: found_artist.id.clone(),
                })
            }
            None => Err("Didn't find any artists. Did you typo the artists name?".into()),
        }
    }
}

fn choose_element<T: Display>(elems: &[T]) -> Result<u8, Box<dyn error::Error>> {
    println!("Which one of these is the one you wanted?");
    println!("Give the number/index of the one you want, or X if none of them.\n");
    for (ind, e) in elems.iter().enumerate() {
        println!("#{ind}: {e}");
    }

    let mut user_response = String::new();
    io::stdin().read_line(&mut user_response)?;
    user_response = user_response.trim().to_lowercase();

    if !(user_response.is_empty() || user_response.starts_with("x")) {
        let ind: u8 = user_response.parse()?;

        Ok(ind)
    } else {
        Err("None selected.".into())
    }
}

async fn create_playlist(
    auth: &mut SpotifyAuth,
    name: &str,
    description: &str,
    public: bool,
) -> Result<PlaylistCreateResponse, Box<dyn error::Error>> {
    let user = get_user(auth).await?;

    #[cfg(debug_assertions)]
    println!("Creating playlist for user with id: {}", user.id);

    let url = format!("https://api.spotify.com/v1/users/{}/playlists", user.id);

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let mut res_builder = client.post(url).headers(headers);
    let mut map = serde_json::Map::new();
    map.insert("name".to_string(), serde_json::Value::from(name));
    map.insert("public".to_string(), serde_json::Value::from(public));
    map.insert(
        "description".to_string(),
        serde_json::Value::from(description),
    );
    res_builder = res_builder.json(&map);
    let res = res_builder.send().await?;

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    let response_text = res.text().await?;
    let playlist_create_response: PlaylistCreateResponse =
        serde_json::from_str(&response_text).map_err(|_| response_text)?;

    Ok(playlist_create_response)
}

pub async fn recommendation_init(auth: &mut SpotifyAuth) -> Result<(), Box<dyn error::Error>> {
    if let Ok(id) = get_managed_playlist_id() {
        println!("The env variable for a managed playlist is already set to: {id}");
        println!("Do you want to create a new managed playlist anyway? (Y/n)");

        let mut user_response = String::new();
        io::stdin().read_line(&mut user_response)?;
        user_response = user_response.trim().to_lowercase();

        if !(user_response.is_empty() || user_response.starts_with("y")) {
            println!("Ok, NOT creating a new playlist. Exiting.");
            return Ok(());
        }
    }

    let name = "CLI managed playlist";
    let description = "This playlist is created and managed by a CLI tool to hold generated recommendations. Do not touch!";
    let playlist_create_response = create_playlist(auth, name, description, false).await?;

    println!("Managed playlist created.");
    println!("The API does not allow setting the playlist as fully private; you might want to do this from the app now.");
    println!();
    println!("You now need to set the following environment variable:");
    println!(
        "export SPOTIFY_CLI_MANAGED_PLAYLIST_ID={}",
        playlist_create_response.id
    );

    Ok(())
}

async fn get_user(auth: &mut SpotifyAuth) -> Result<User, Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client.get(url).headers(headers).send().await?;

    if res.error_for_status_ref().is_err() {
        let response_text = res.text().await?;
        let response_parsed: Value = serde_json::from_str(&response_text)?;
        return Err(response_parsed["error"]["message"].as_str().unwrap().into());
    }

    let response_text = res.text().await?;
    let user_response: User = serde_json::from_str(&response_text).map_err(|_| response_text)?;

    Ok(user_response)
}
