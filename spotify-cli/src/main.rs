mod auth;

use auth::SpotifyAuth;
use clap::{Args, Parser, Subcommand};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
use std::{env, error};

#[derive(Debug, Parser)]
#[clap(
    name = "spotify-cli",
    version,
    author,
    help_template = "
{before-help}{name} {version}
{author-with-newline}
{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
"
)]
/// Simple CLI tool for managing Spotify playback and playlists
struct App {
    #[clap(flatten)]
    options: Options,

    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Debug, Args)]
struct Options {
    /// Filepath for storing auth tokens; if omitted ~/.spotify_cli_token is used
    #[clap(long, short, global = true)]
    token_path: Option<String>,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    /// Control/see active playback
    #[command(subcommand)]
    Playback(PlaybackCommand),

    /// Control/see current queue or create new queue from recommendations
    #[command(subcommand)]
    Queue(QueueCommand),

    /// Control authentication tokens
    #[command(subcommand)]
    Auth(AuthCommand),
}

#[derive(Clone, Debug, Subcommand)]
enum PlaybackCommand {
    /// Pause playback
    Pause,

    /// Start/resume playback
    Play,

    /// Show current playback
    Show,
}

#[derive(Clone, Debug, Subcommand)]
enum QueueCommand {
    /// Show current queue
    Show,
}

#[derive(Clone, Debug, Subcommand)]
enum AuthCommand {
    /// Refresh current token
    Refresh,

    /// Reset token, i.e. re-authorize & authenticate
    Reset,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let args = App::parse();

    let token_path = match args.options.token_path {
        Some(token_path) => token_path,
        None => {
            let default_filepath = dirs::home_dir()
                .ok_or_else(|| "Can't get home directory?")?
                .join(".spotify_cli_token")
                .to_str()
                .unwrap()
                .to_string();
            env::var("SPOTIFY_CLI_TOKEN_FILE").unwrap_or(default_filepath)
        }
    };

    // TODO: check if file exists; if not create auth struct with new file instead
    let mut auth = SpotifyAuth::from_file(&token_path)?;

    match args.command {
        Command::Playback(PlaybackCommand::Show) => {
            playback_show(&mut auth).await?;
        }
        Command::Auth(AuthCommand::Refresh) => auth.refresh_token().await?,
        Command::Auth(AuthCommand::Reset) => auth.reset_auth().await?,
        _ => unimplemented!(),
    }

    Ok(())
}

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

async fn playback_show(auth: &mut SpotifyAuth) -> Result<(), Box<dyn error::Error>> {
    let url = "https://api.spotify.com/v1/me/player".to_string();

    let headers = auth_header(auth).await?;

    let client = reqwest::Client::new();
    let res = client.get(url).headers(headers).send().await?;
    let parsed_res: Value = serde_json::from_str(res.text().await?.as_str())?;

    println!("Playing on: {}", parsed_res["device"]["name"]);

    // TODO: song struct, impl display, maybe don't just unwrap everything
    let artists: Vec<String> = parsed_res["item"]["artists"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["name"].clone().to_string())
        .collect();
    println!(
        "Artists:    {}",
        artists
            .iter()
            .fold("".to_string(), |acc, x| acc + ", " + x)
            .strip_prefix(", ")
            .unwrap()
    );

    println!("Song:       {}", parsed_res["item"]["name"]);

    Ok(())
}

// Manual testing of auth

// use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
// use serde_json::Value;
// use std::{env, error, time::SystemTime};

// async fn main() -> Result<(), Box<dyn error::Error>> {
//     let default_filepath = dirs::home_dir()
//         .ok_or_else(|| "Can't get home directory?")?
//         .join(".spotify_cli_token")
//         .to_str()
//         .unwrap()
//         .to_string();
//     let token_filepath = env::var("SPOTIFY_CLI_TOKEN_FILE").unwrap_or(default_filepath);

//     let mut a = SpotifyAuth::from_file(&token_filepath)?;

//     // let mut a = SpotifyAuth::new()?;
//     // a.with_file(&token_filepath)?;

//     #[cfg(debug_assertions)]
//     println!(
//         "Starting time as secs since epoch:\n{}\n\n",
//         SystemTime::now()
//             .duration_since(SystemTime::UNIX_EPOCH)?
//             .as_secs()
//     );
//     #[cfg(debug_assertions)]
//     a.print_auth_info();

//     // a.reset_auth().await?;

//     // #[cfg(debug_assertions)]
//     // println!("\nAuth after reset:");
//     // #[cfg(debug_assertions)]
//     // a.print_auth_info();

//     let access_token = a.get_access_token().await?;
//     println!("\nget_access_token result: {:?}\n", access_token);

//     println!("\n\nGot an access token. Getting user info:\n\n");

//     let authorization_value = format!("Bearer {}", access_token);
//     let mut headers = HeaderMap::new();
//     headers.insert(
//         HeaderName::from_static("authorization"),
//         HeaderValue::from_str(&authorization_value)?,
//     );
//     let url = "https://api.spotify.com/v1/me".to_string();

//     let client = reqwest::Client::new();
//     let res = client.get(url).headers(headers).send().await?;
//     let parsed_res: Value = serde_json::from_str(res.text().await?.as_str())?;

//     println!("{parsed_res}\n");

//     a.refresh_token().await?;

//     #[cfg(debug_assertions)]
//     println!("\n\nAuth struct state after refreshing token:");
//     #[cfg(debug_assertions)]
//     a.print_auth_info();

//     let access_token = a.get_access_token().await?;
//     println!(
//         "\n\n Second time get_access_token result: {:?}",
//         access_token
//     );

//     #[cfg(debug_assertions)]
//     println!("\n\nAuth struct state after everything:");
//     #[cfg(debug_assertions)]
//     a.print_auth_info();

//     Ok(())
// }
