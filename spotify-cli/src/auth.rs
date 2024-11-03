use base64::{prelude::BASE64_STANDARD, Engine};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    StatusCode, Url,
};
use serde::Deserialize;
use serde_json;
use std::{
    env, error,
    fmt::Display,
    fs,
    io::{self, Read},
    time::SystemTime,
};

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct AuthenticationResponse {
    access_token: String,
    token_type: String,
    scope: String,
    expires_in: u64,
    refresh_token: Option<String>,
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
    valid_until: Option<u64>,
    refresh_token: Option<String>,
}

#[derive(Deserialize, Debug)]
struct TokenFile {
    access_token: Option<String>,
    valid_until: Option<u64>,
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
            valid_until: None,
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

        let default_filepath = dirs::home_dir()
            .ok_or_else(|| "Can't get home directory?")?
            .join(".spotify_cli_token")
            .to_str()
            .unwrap()
            .to_string();
        let token_filepath = env::var("SPOTIFY_CLI_TOKEN_FILE").unwrap_or(default_filepath);
        let mut token_file = fs::File::open(token_filepath.clone())
            .map_err(|_| format!("Failed to open file {}", token_filepath))?;
        let mut token_file_str = String::new();
        token_file.read_to_string(&mut token_file_str)?;
        let tokens: TokenFile = serde_json::from_str(&token_file_str)?;

        Ok(SpotifyAuth {
            client_id: client_id,
            client_secret: client_secret,
            redirect_port: redirect_port,
            access_token: tokens.access_token,
            valid_until: tokens.valid_until,
            refresh_token: tokens.refresh_token,
        })
    }

    fn to_file() -> Result<(), Box<dyn error::Error>> {
        // TODO: save token to file
        unimplemented!()
    }

    pub async fn reset_auth(&mut self) -> Result<(), Box<dyn error::Error>> {
        self.access_token = None;
        self.valid_until = None;
        self.refresh_token = None;

        // TODO: reset tokens in file

        Ok(())
    }

    pub async fn get_access_token(&mut self) -> Result<String, Box<dyn error::Error>> {
        match (&self.access_token, &self.valid_until, &self.refresh_token) {
            (Some(access_token), Some(valid_until), Some(_)) => {
                let curr_time = current_time_secs_from_epoch()?;
                if curr_time >= valid_until - 120 {
                    self.refresh_token().await?;
                    if let Some(access_token) = &self.access_token {
                        Ok(access_token.clone())
                    } else {
                        Err(GenericError(
                            "Broken auth state: access token is missing after a refresh."
                                .to_string(),
                        )
                        .into())
                    }
                } else {
                    Ok(access_token.clone())
                }
            }
            (None, None, None) => {
                let authorization_code = self.authorize()?;
                let (access_token, refresh_token, valid_until) =
                    self.authenticate(&authorization_code).await?;
                self.access_token = Some(access_token.clone());
                self.valid_until = Some(valid_until);
                self.refresh_token = Some(refresh_token);
                Ok(access_token)
            }
            _ => Err(GenericError(
                "Broken auth state: some of the token fields are missing but not all.".to_string(),
            )
            .into()),
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
    ) -> Result<(String, String, u64), Box<dyn error::Error>> {
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

        let curr_time = current_time_secs_from_epoch()?;
        let client = reqwest::Client::new();
        let res = client.post(url).headers(headers).form(&form).send().await?;

        match res.status() {
            StatusCode::OK => {
                let auth_response: AuthenticationResponse = res.json().await?;

                #[cfg(debug_assertions)]
                println!("Authentication response:\n{:?}\n", auth_response);

                Ok((
                    auth_response.access_token,
                    auth_response.refresh_token.unwrap(),
                    curr_time + auth_response.expires_in,
                ))
            }
            _ => Err(GenericError(res.text().await?).into()),
        }
    }

    pub async fn refresh_token(&mut self) -> Result<(), Box<dyn error::Error>> {
        let url = Url::parse("https://accounts.spotify.com/api/token")?;

        let mut headers = HeaderMap::new();
        let encoded_id_and_secret =
            BASE64_STANDARD.encode(format!("{}:{}", self.client_id, self.client_secret));
        let authorization_header = format!("Basic {}", encoded_id_and_secret);
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&authorization_header)?,
        );

        if let Some(refresh_token) = &self.refresh_token {
            let form = [
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token.as_str()),
            ];

            #[cfg(debug_assertions)]
            println!("Refreshing token request url: {}", url.as_str());
            #[cfg(debug_assertions)]
            println!("Headers: {:?}", headers);
            #[cfg(debug_assertions)]
            println!("Form: {:?}\n", form);

            let curr_time = current_time_secs_from_epoch()?;
            let client = reqwest::Client::new();
            let res = client.post(url).headers(headers).form(&form).send().await?;

            match res.status() {
                StatusCode::OK => {
                    let auth_response: AuthenticationResponse = res.json().await?;

                    #[cfg(debug_assertions)]
                    println!("Refreshing token response:\n{:?}\n", auth_response);

                    self.access_token = Some(auth_response.access_token);
                    if let Some(refresh_token) = auth_response.refresh_token {
                        self.refresh_token = Some(refresh_token);
                    }
                    self.valid_until = Some(curr_time + auth_response.expires_in);

                    Ok(())
                }
                _ => Err(GenericError(res.text().await?).into()),
            }
        } else {
            Err(
                GenericError("Can't refresh token since refresh_token is missing.".to_string())
                    .into(),
            )
        }
    }

    #[cfg(debug_assertions)]
    pub fn print_auth_info(&self) {
        println!("client_id: {}", self.client_id);
        println!("client_secret: {}", self.client_secret);
        println!("redirect_port: {}", self.redirect_port);
        println!("access_token: {:?}", self.access_token);
        println!("valid_until: {:?}", self.valid_until);
        println!("refresh_token: {:?}", self.refresh_token);

        println!();
    }
}

fn current_time_secs_from_epoch() -> Result<u64, Box<dyn error::Error>> {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    Ok(secs)
}
