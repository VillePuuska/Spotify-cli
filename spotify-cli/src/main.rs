mod auth;

use auth::SpotifyAuth;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
use std::{env, error, time::SystemTime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    // let mut a = SpotifyAuth::new()?;

    let default_filepath = dirs::home_dir()
        .ok_or_else(|| "Can't get home directory?")?
        .join(".spotify_cli_token")
        .to_str()
        .unwrap()
        .to_string();
    let token_filepath = env::var("SPOTIFY_CLI_TOKEN_FILE").unwrap_or(default_filepath);
    let mut a = SpotifyAuth::from_file(&token_filepath)?;

    #[cfg(debug_assertions)]
    println!(
        "Starting time as secs since epoch:\n{}\n\n",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
    );
    #[cfg(debug_assertions)]
    a.print_auth_info();

    let access_token = a.get_access_token().await?;
    println!("get_access_token result: {:?}\n", access_token);

    println!("\n\nGot an access token. Getting user info:\n\n");

    let authorization_value = format!("Bearer {}", access_token);
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&authorization_value)?,
    );
    let url = "https://api.spotify.com/v1/me".to_string();

    let client = reqwest::Client::new();
    let res = client.get(url).headers(headers).send().await?;
    let parsed_res: Value = serde_json::from_str(res.text().await?.as_str())?;

    println!("{parsed_res}\n");

    a.refresh_token().await?;

    #[cfg(debug_assertions)]
    println!("\n\nAuth struct state after refreshing token:");
    #[cfg(debug_assertions)]
    a.print_auth_info();

    let access_token = a.get_access_token().await?;
    println!(
        "\n\n Second time get_access_token result: {:?}",
        access_token
    );

    #[cfg(debug_assertions)]
    println!("\n\nAuth struct state after everything:");
    #[cfg(debug_assertions)]
    a.print_auth_info();

    Ok(())
}
