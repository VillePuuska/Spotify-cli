use base64::{prelude::BASE64_STANDARD, Engine};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    StatusCode, Url,
};
use serde::Deserialize;
use std::{env, error, fmt::Display, io, time::Instant};

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct AuthenticationResponse {
    access_token: String,
    token_type: String,
    scope: String,
    expires_in: u64,
    refresh_token: String,
}

#[derive(Debug)]
pub struct GenericError(String);

impl Display for GenericError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GenericError: {}", self.0)
    }
}

impl error::Error for GenericError {}

pub struct SpotifyAuth {
    client_id: String,
    client_secret: String,
    redirect_port: u32,
    access_token: Option<String>,
    refresh_time: Option<Instant>,
    valid_for_secs: Option<u64>,
    refresh_token: Option<String>,
}

impl SpotifyAuth {
    pub fn new() -> Result<SpotifyAuth, Box<dyn error::Error>> {
        let client_id = env::var("SPOTIFY_CLI_CLIENT_ID")
            .map_err(|_| "The env variable SPOTIFY_CLI_CLIENT_ID must be set.")?;
        let client_secret = env::var("SPOTIFY_CLI_CLIENT_SECRET")
            .map_err(|_| "The env variable SPOTIFY_CLI_CLIENT_SECRET must be set.")?;
        let redirect_port = env::var("SPOTIFY_CLI_REDIRECT_PORT")
            .ok()
            .unwrap_or("5555".to_string())
            .parse::<u32>()
            .map_err(|_| "Failed to parse SPOTIFY_CLI_REDIRECT_PORT to a u32.")?;
        Ok(SpotifyAuth {
            client_id: client_id,
            client_secret: client_secret,
            redirect_port: redirect_port,
            access_token: None,
            refresh_time: None,
            valid_for_secs: None,
            refresh_token: None,
        })
    }

    pub fn from_file() -> Result<SpotifyAuth, Box<dyn error::Error>> {
        let client_id = env::var("SPOTIFY_CLI_CLIENT_ID")
            .map_err(|_| "The env variable SPOTIFY_CLI_CLIENT_ID must be set.")?;
        let client_secret = env::var("SPOTIFY_CLI_CLIENT_SECRET")
            .map_err(|_| "The env variable SPOTIFY_CLI_CLIENT_SECRET must be set.")?;
        let redirect_port = env::var("SPOTIFY_CLI_REDIRECT_PORT")
            .ok()
            .unwrap_or("5555".to_string())
            .parse::<u32>()
            .map_err(|_| "Failed to parse SPOTIFY_CLI_REDIRECT_PORT to a u32.")?;

        // TODO: get filepath and tokens from file

        Ok(SpotifyAuth {
            client_id: client_id,
            client_secret: client_secret,
            redirect_port: redirect_port,
            access_token: None,
            refresh_time: None,
            valid_for_secs: None,
            refresh_token: None,
        })
    }

    pub async fn reset_auth(&mut self) -> Result<(), Box<dyn error::Error>> {
        self.access_token = None;
        self.refresh_token = None;

        // TODO: reset tokens in file

        Ok(())
    }

    pub async fn get_access_token(&mut self) -> Result<String, Box<dyn error::Error>> {
        // TODO: check for need to refresh
        match &self.access_token {
            Some(token) => Ok(token.clone()),
            None => {
                let authorization_code = self.authorize()?;
                let (access_token, refresh_token, refresh_time, valid_for_secs) =
                    self.authenticate(&authorization_code).await?;
                self.access_token = Some(access_token.clone());
                self.refresh_time = Some(refresh_time);
                self.valid_for_secs = Some(valid_for_secs);
                self.refresh_token = Some(refresh_token);
                Ok(access_token)
            }
        }
    }

    fn authorize(&self) -> Result<String, Box<dyn error::Error>> {
        let url = Url::parse_with_params(
            "https://accounts.spotify.com/authorize",
            &[
                ("client_id", &self.client_id),
                ("response_type", &"code".to_string()),
                (
                    "redirect_uri",
                    &format!("https://localhost:{}", &self.redirect_port),
                ),
                ("scope", &"user-read-email".to_string()),
            ],
        )?;

        let mut user_provided_token = String::new();
        println!("Go to this url for the auth flow: {}", url.as_str());
        println!("Then, write the authorization code from the redirect url here:");
        io::stdin().read_line(&mut user_provided_token)?;
        user_provided_token = user_provided_token.trim().to_string();

        #[cfg(debug_assertions)]
        println!("\nUser provided token: {user_provided_token}\n");

        Ok(user_provided_token)
    }

    async fn authenticate(
        &self,
        authorization_code: &String,
    ) -> Result<(String, String, Instant, u64), Box<dyn error::Error>> {
        let url = Url::parse("https://accounts.spotify.com/api/token")?;

        let mut headers = HeaderMap::new();
        let encoded_id_and_secret =
            BASE64_STANDARD.encode(format!("{}:{}", self.client_id, self.client_secret));
        let authorization_header = format!("Basic {}", encoded_id_and_secret);
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&authorization_header)?,
        );

        let redirect_uri = format!("https://localhost:{}", &self.redirect_port);
        let form = [
            ("grant_type", "authorization_code"),
            ("code", authorization_code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
        ];

        #[cfg(debug_assertions)]
        println!("Authentication request url: {}", url.as_str());
        #[cfg(debug_assertions)]
        println!("Headers: {:?}", headers);
        #[cfg(debug_assertions)]
        println!("Form: {:?}\n", form);

        let curr_time = Instant::now();
        let client = reqwest::Client::new();
        let res = client.post(url).headers(headers).form(&form).send().await?;

        match res.status() {
            StatusCode::OK => {
                let auth_response: AuthenticationResponse = res.json().await?;

                #[cfg(debug_assertions)]
                println!("Authentication response:\n{:?}\n", auth_response);

                Ok((
                    auth_response.access_token,
                    auth_response.refresh_token,
                    curr_time,
                    auth_response.expires_in,
                ))
            }
            _ => Err(GenericError(res.text().await?).into()),
        }
    }

    #[allow(dead_code)]
    pub fn refresh_token(&mut self) -> Result<(), ()> {
        // TODO: refresh token
        Err(())
    }

    #[cfg(debug_assertions)]
    pub fn print_auth_info(&self) {
        println!("client_id: {}", self.client_id);
        println!("client_secret: {}", self.client_secret);
        println!("redirect_port: {}", self.redirect_port);
        println!("access_token: {:?}", self.access_token);
        println!("refresh_time: {:?}", self.refresh_time);
        println!("valid_for_secs: {:?}", self.valid_for_secs);
        println!("refresh_token: {:?}", self.refresh_token);

        println!();
    }
}
