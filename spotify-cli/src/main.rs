mod auth;
mod handlers;

use auth::SpotifyAuth;
use clap::{Args, Parser, Subcommand};
use handlers::*;
use std::{env, error, fs, io, time::Duration};

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
    /// Show current playback
    Show,

    /// Pause playback
    #[command(visible_alias = "stop")]
    Pause,

    /// Start/resume playback
    #[command(visible_alias = "start")]
    Play,

    /// Play next track
    #[command(visible_alias = "forward")]
    Next,

    /// Play previous track
    #[command(visible_alias = "back")]
    Previous,

    /// Restart current track
    #[command(visible_alias = "rewind")]
    Restart,

    /// Show the current playlist's tracks
    Current {
        /// Max number of songs to print around the current track
        max_lines: Option<u16>,
    },

    /// Jump to song in current playlist
    Jump { offset: u16 },

    /// Show current queue
    Queue {
        /// Number of songs in the queue to show (including the current song).
        #[arg(default_value = "5")]
        number: usize,
    },

    /// Control/see playlists (see subcommands)
    #[command(subcommand)]
    Playlist(PlaylistCommand),

    /// Control authentication tokens (see subcommands)
    #[command(subcommand)]
    Auth(AuthCommand),

    /// Recommendations commands (see subcommands)
    #[command(subcommand, visible_alias = "rec")]
    Recommendation(RecommendationCommand),
}

#[derive(Clone, Debug, Subcommand)]
enum AuthCommand {
    /// Refresh current token
    Refresh,

    /// Reset token, i.e. re-authorize & authenticate
    Reset,
}

#[derive(Clone, Debug, Subcommand)]
enum PlaylistCommand {
    /// Show/list all my playlists
    List,

    /// Start playing a playlist
    Play { uri: String, index: Option<u16> },
}

#[derive(Clone, Debug, Subcommand)]
enum RecommendationCommand {
    /// Show latest recommendation list
    Show {
        /// Max number of songs to print from the start of the list
        max_lines: Option<u16>,
    },

    /// Start playing the latest recommendation list
    Play { index: Option<u16> },

    /// Save the latest list of recommendations to a playlist
    Save {
        name: String,
        description: Option<String>,
    },

    /// Generate a new list of recommendations
    Generate,

    /// Creates a new playlist to be managed by this tool and prints the corresponding env variable
    Init,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let args = App::parse();

    let token_path = match args.options.token_path {
        Some(token_path) => token_path,
        None => {
            let default_filepath = dirs::home_dir()
                .ok_or("Can't get home directory?")?
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

    let mut auth = match fs::exists(&token_path)? {
        true => SpotifyAuth::from_file(&client_id, &client_secret, &token_path)?,
        false => {
            println!("There are no tokens saved in {token_path}.");
            println!("Save new tokens there? Y/n");

            let mut user_response = String::new();
            io::stdin().read_line(&mut user_response)?;
            user_response = user_response.trim().to_lowercase();

            if !(user_response.is_empty() || user_response.starts_with("y")) {
                println!("Ok, NOT generating and saving new tokens. Exiting.");
                return Ok(());
            }

            let mut tmp = SpotifyAuth::new(&client_id, &client_secret)?;
            tmp.with_file(&token_path)?;
            tmp
        }
    };

    match args.command {
        Command::Show => playback_show(&mut auth, true).await?,
        Command::Pause => playback_pause(&mut auth).await?,
        Command::Play => playback_play(&mut auth, None, None).await?,
        Command::Next => {
            playback_next(&mut auth).await?;
            // The API keeps returning the previously played song
            // without a bit of a sleep here. Not happy about this
            // but what can I do...
            tokio::time::sleep(Duration::from_millis(500u64)).await;
            playback_show(&mut auth, false).await?;
        }
        Command::Previous => {
            playback_previous(&mut auth).await?;
            tokio::time::sleep(Duration::from_millis(500u64)).await;
            playback_show(&mut auth, false).await?;
        }
        Command::Restart => playback_restart(&mut auth).await?,
        Command::Current { max_lines } => playlist_current(&mut auth, max_lines).await?,
        Command::Jump { offset } => {
            playback_play(&mut auth, None, Some(offset)).await?;
            tokio::time::sleep(Duration::from_millis(500u64)).await;
            playback_show(&mut auth, false).await?;
        }
        Command::Queue { number } => queue_show(&mut auth, number).await?,
        Command::Playlist(PlaylistCommand::List) => playlist_list(&mut auth).await?,
        Command::Playlist(PlaylistCommand::Play { uri, index }) => {
            playback_play(&mut auth, Some(&uri), index).await?;
            tokio::time::sleep(Duration::from_millis(500u64)).await;
            playback_show(&mut auth, false).await?;
        }
        Command::Auth(AuthCommand::Refresh) => auth.refresh_token().await?,
        Command::Auth(AuthCommand::Reset) => auth.reset_auth().await?,
        Command::Recommendation(RecommendationCommand::Show { max_lines }) => {
            recommendation_show(&mut auth, max_lines).await?
        }
        Command::Recommendation(RecommendationCommand::Play { index }) => {
            recommendation_play(&mut auth, index).await?
        }
        Command::Recommendation(RecommendationCommand::Save { name, description }) => {
            recommendation_save(&mut auth, name, description).await?
        }
        Command::Recommendation(RecommendationCommand::Generate) => {
            recommendation_generate(&mut auth).await?
        }
        Command::Recommendation(RecommendationCommand::Init) => {
            recommendation_init(&mut auth).await?
        }
        #[allow(unreachable_patterns)]
        _ => unimplemented!(),
    }

    Ok(())
}
