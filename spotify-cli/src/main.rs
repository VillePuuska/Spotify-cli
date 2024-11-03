mod auth;

use auth::SpotifyAuth;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
use std::error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let mut a = SpotifyAuth::new()?;

    #[cfg(debug_assertions)]
    a.print_auth_info();

    let access_token = a.get_access_token().await?;
    println!("get_access_token result: {:?}", access_token);

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

    println!("{parsed_res}");

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
