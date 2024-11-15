mod auth;
mod handlers;

use auth::SpotifyAuth;
use clap::{Args, Parser, Subcommand};
use handlers::*;
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

    /// Play next track
    Next,

    /// Play previous track
    Previous,

    /// Restart current track
    Restart,
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
        Command::Queue(QueueCommand::Show) => {
            queue_show(&mut auth).await?;
        }
        Command::Auth(AuthCommand::Refresh) => auth.refresh_token().await?,
        Command::Auth(AuthCommand::Reset) => auth.reset_auth().await?,
        #[allow(unreachable_patterns)]
        _ => unimplemented!(),
    }

    Ok(())
}
