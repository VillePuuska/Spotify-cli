mod auth;
mod handlers;

use auth::SpotifyAuth;
use clap::{Args, Parser, Subcommand};
use handlers::*;
use std::{env, error, fs};

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
    #[command(visible_alias = "stop")]
    Pause,

    /// Start/resume playback
    #[command(visible_alias = "start")]
    Play,

    /// Show current playback
    #[command(visible_alias = "current")]
    Show,

    /// Play next track
    #[command(visible_alias = "forward")]
    Next,

    /// Play previous track
    #[command(visible_alias = "back")]
    Previous,

    /// Restart current track
    #[command(visible_alias = "rewind")]
    Restart,
}

#[derive(Clone, Debug, Subcommand)]
enum QueueCommand {
    /// Show current queue
    #[command(visible_alias = "current")]
    Show {
        /// Number of songs in the queue to show (including the current song).
        #[arg(default_value = "5")]
        number: usize,
    },
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

    let client_id = env::var("SPOTIFY_CLI_CLIENT_ID")
        .map_err(|_| "The env variable SPOTIFY_CLI_CLIENT_ID must be set.")?;
    let client_secret = env::var("SPOTIFY_CLI_CLIENT_SECRET")
        .map_err(|_| "The env variable SPOTIFY_CLI_CLIENT_SECRET must be set.")?;

    // TODO: Should I just do this in SpotifyAuth::from_file instead?
    // Or should I maybe add a user prompt _here_ to verify they want to create a new file
    // to make sure they (read: I) didn't e.g. typo the token filepath? That was the original
    // reason to not do this in SpotifyAuth::from_file.
    let mut auth = match fs::exists(&token_path) {
        Ok(true) => SpotifyAuth::from_file(&client_id, &client_secret, &token_path)?,
        _ => {
            let mut tmp = SpotifyAuth::new(&client_id, &client_secret)?;
            tmp.with_file(&token_path)?;
            tmp
        }
    };

    match args.command {
        Command::Playback(PlaybackCommand::Pause) => {
            playback_pause(&mut auth).await?;
        }
        Command::Playback(PlaybackCommand::Play) => {
            playback_play(&mut auth).await?;
        }
        Command::Playback(PlaybackCommand::Show) => {
            playback_show(&mut auth).await?;
        }
        Command::Playback(PlaybackCommand::Next) => {
            playback_next(&mut auth).await?;
        }
        Command::Playback(PlaybackCommand::Previous) => {
            playback_previous(&mut auth).await?;
        }
        Command::Playback(PlaybackCommand::Restart) => {
            playback_restart(&mut auth).await?;
        }
        Command::Queue(QueueCommand::Show { number }) => {
            queue_show(&mut auth, number).await?;
        }
        Command::Auth(AuthCommand::Refresh) => auth.refresh_token().await?,
        Command::Auth(AuthCommand::Reset) => auth.reset_auth().await?,
        #[allow(unreachable_patterns)]
        _ => unimplemented!(),
    }

    Ok(())
}
