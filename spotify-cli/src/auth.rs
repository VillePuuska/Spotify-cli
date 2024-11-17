use base64::{prelude::BASE64_STANDARD, Engine};
use rand::distributions::{Alphanumeric, DistString};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    StatusCode, Url,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    error, fs,
    io::{self, Read, Write},
    str::FromStr,
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

pub struct SpotifyAuth {
    client_id: String,
    client_secret: String,
    access_token: Option<String>,
    valid_until: Option<u64>,
    refresh_token: Option<String>,
    filepath: Option<String>,
}

#[derive(Deserialize, Debug, Serialize)]
struct TokenFile {
    access_token: Option<String>,
    valid_until: Option<u64>,
    refresh_token: Option<String>,
}

impl SpotifyAuth {
    /// Creates a new `SpotifyAuth` object.
    ///
    /// NOTE: the credentials will not be saved & synced to a file yet.
    /// To set a new file to save credentials to, set the filepath with
    /// `with_file` after initializing with this method.
    /// If you're actually looking to read credentials from a file,
    /// don't use this method; use `from_file` instead.
    pub fn new(client_id: &str, client_secret: &str) -> Result<SpotifyAuth, Box<dyn error::Error>> {
        Ok(SpotifyAuth {
            client_id: client_id.to_owned(),
            client_secret: client_secret.to_owned(),
            access_token: None,
            valid_until: None,
            refresh_token: None,
            filepath: None,
        })
    }

    /// Sets a file to save & sync credentials to.
    ///
    /// NOTE: overwrites any existing data.
    pub fn with_file(&mut self, filepath: &str) -> Result<(), Box<dyn error::Error>> {
        self.filepath = Some(filepath.to_owned());
        self.save()?;

        Ok(())
    }

    /// Stops saving & syncing credentials to a file.
    ///
    /// NOTE: this does not delete the file, if it already exists.
    /// The filepath is simply set to `None` in the struct.
    #[allow(dead_code)]
    pub fn remove_file(&mut self) {
        self.filepath = None;
    }

    /// Reads credentials from a file.
    ///
    /// NOTE: fails if file does not already exist. Use `with_file` if you're
    /// looking to read credentials from an existing file.
    pub fn from_file(
        client_id: &str,
        client_secret: &str,
        filepath: &str,
    ) -> Result<SpotifyAuth, Box<dyn error::Error>> {
        let mut auth = Self::new(client_id, client_secret)?;
        auth.filepath = Some(filepath.to_owned());
        auth.load()?;

        Ok(auth)
    }

    fn load(&mut self) -> Result<(), Box<dyn error::Error>> {
        let filepath = self
            .filepath
            .as_ref()
            .ok_or("Can't load when filepath is not set.")?;
        let mut token_file = fs::File::open(filepath.clone())
            .map_err(|_| format!("Failed to open file {}", filepath))?;
        let mut token_file_str = String::new();
        token_file.read_to_string(&mut token_file_str)?;
        let tokens: TokenFile = serde_json::from_str(&token_file_str)?;

        self.access_token = tokens.access_token;
        self.valid_until = tokens.valid_until;
        self.refresh_token = tokens.refresh_token;

        Ok(())
    }

    fn save(&self) -> Result<(), Box<dyn error::Error>> {
        if let Some(ref filepath) = self.filepath {
            let tokens = TokenFile {
                access_token: self.access_token.clone(),
                valid_until: self.valid_until,
                refresh_token: self.refresh_token.clone(),
            };
            let token_str = serde_json::to_string(&tokens)?;
            let mut token_file = fs::File::create(filepath)?;
            write!(token_file, "{token_str}")?;
        }

        Ok(())
    }

    /// Resets the tokens.
    ///
    /// NOTE: if the credentials are saved to a file, this method also
    /// resets the data in the file.
    pub async fn reset_auth(&mut self) -> Result<(), Box<dyn error::Error>> {
        self.access_token = None;
        self.valid_until = None;
        self.refresh_token = None;

        self.save()?;

        Ok(())
    }

    /// This method retrieves an access token for the authorized user.
    ///
    /// If there is not authorized user yet, starts with the authorization
    /// & authentication flow.
    ///
    /// If the token is about to expire within 2 minutes, then the token is
    /// first refreshed.
    pub async fn get_access_token(&mut self) -> Result<String, Box<dyn error::Error>> {
        match (&self.access_token, &self.valid_until, &self.refresh_token) {
            (Some(access_token), Some(valid_until), Some(_)) => {
                let curr_time = current_time_secs_from_epoch()?;
                if curr_time >= valid_until - 120 {
                    self.refresh_token().await?;
                    if let Some(access_token) = &self.access_token {
                        Ok(access_token.clone())
                    } else {
                        Err(
                            "Broken auth state: access token is missing after a refresh."
                                .to_string()
                                .into(),
                        )
                    }
                } else {
                    Ok(access_token.clone())
                }
            }
            (None, None, None) => {
                let (authorization_code, redirect_port) = self.authorize()?;
                let (access_token, refresh_token, valid_until) = self
                    .authenticate(&authorization_code, redirect_port)
                    .await?;
                self.access_token = Some(access_token.clone());
                self.valid_until = Some(valid_until);
                self.refresh_token = Some(refresh_token);

                self.save()?;

                Ok(access_token)
            }
            _ => Err(
                "Broken auth state: some of the token fields are missing but not all."
                    .to_string()
                    .into(),
            ),
        }
    }

    fn authorize(&self) -> Result<(String, u16), Box<dyn error::Error>> {
        let state = generate_random_state();

        let redirect_port = get_free_port()?;
        let url = Url::parse_with_params(
            "https://accounts.spotify.com/authorize",
            &[
                ("client_id", &self.client_id),
                ("response_type", &"code".to_string()),
                (
                    "redirect_uri",
                    &format!("http://localhost:{}", redirect_port),
                ),
                ("state", &state),
                (
                    "scope",
                    &"user-read-playback-state user-read-currently-playing user-modify-playback-state playlist-read-private"
                        .to_string(),
                ),
            ],
        )?;

        println!("Go to this url for the auth flow: {}", url.as_str());

        let redirected_to = match tiny_http::Server::http(format!("127.0.0.1:{redirect_port}")) {
            Ok(server) => {
                let request = server.recv()?;
                let request_url = request.url().to_string();
                request.respond(tiny_http::Response::from_string(
                    "Succesfully received the redirected url. You can now close this tab."
                        .to_string(),
                ))?;
                format!("http://localhost:{redirect_port}{request_url}")
            }
            Err(e) => {
                println!("Failed to start a server to listen to the redirect:\n{e}\n");
                println!("Instead, write the entire url you were redirected to here:");
                let mut user_provided_url = String::new();
                io::stdin().read_line(&mut user_provided_url)?;
                user_provided_url.trim().to_string()
            }
        };

        #[cfg(debug_assertions)]
        println!("\nRedirected to: {redirected_to}");

        let redirected_url = Url::from_str(&redirected_to)?;

        let query_params: HashMap<String, String> =
            redirected_url.query_pairs().into_owned().collect();

        #[cfg(debug_assertions)]
        println!("\nQuery params in the redirected url: {query_params:?}");

        let token = query_params
            .get("code")
            .ok_or("The query param code is missing from redirect url.")?
            .clone();
        let redirect_state = query_params
            .get("state")
            .ok_or("The query param state is missing from redirect url.")?;

        #[cfg(debug_assertions)]
        println!("\nGenerated state: {state}");
        #[cfg(debug_assertions)]
        println!("User provided state: {redirect_state}\n");
        #[cfg(debug_assertions)]
        println!("\nToken: {token}\n");

        if &state != redirect_state {
            Err("Invalid state! Something fishy might be going on."
                .to_string()
                .into())
        } else {
            Ok((token, redirect_port))
        }
    }

    async fn authenticate(
        &self,
        authorization_code: &str,
        redirect_port: u16,
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

        let redirect_uri = format!("http://localhost:{}", redirect_port);
        let form = [
            ("grant_type", "authorization_code"),
            ("code", authorization_code),
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
            _ => Err(res.text().await?.into()),
        }
    }

    /// Refreshed the access token. Returns an error if there is no
    /// authorized & authenticated user yet.
    ///
    /// NOTE: this is mainly intended to allow manual refreshing of a token
    /// if the current token is not yet about to expire but is misbehaving
    /// for some reason, OR if you need to get a token that will not expire
    /// within a longer duration than the 2 minutes set as the refresh limit
    /// in the method `get_access_token`.
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

                    self.save()?;

                    Ok(())
                }
                _ => Err(res.text().await?.into()),
            }
        } else {
            Err("Can't refresh token since refresh_token is missing."
                .to_string()
                .into())
        }
    }

    #[cfg(debug_assertions)]
    #[allow(dead_code)]
    pub fn print_auth_info(&self) {
        println!("client_id: {}", self.client_id);
        println!("client_secret: {}", self.client_secret);
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

fn generate_random_state() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 16)
}

fn get_free_port() -> Result<u16, Box<dyn error::Error>> {
    // Allowed redirect URIs need to be specified in Spotify's app dashboard.
    // Thus we can't use actually random ports. To allow multiple port choices,
    // we need to list http://localhost:5555, http://localhost:5556, ... in
    // the app dashboard.
    // TODO: get a list of ports from an env var or something? Hardcoding is nasty.
    let possible_ports = [5555, 5556, 5557, 5558, 5559];
    for port in possible_ports {
        if portpicker::is_free(port) {
            return Ok(port);
        }
    }
    Err("All ports unavailable.".to_string().into())
}
