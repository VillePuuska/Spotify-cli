use reqwest::Url;
use std::{env, error, fmt::Display, io};

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
            refresh_token: None,
        })
    }

    pub fn get_access_token(&mut self) -> Result<String, Box<dyn error::Error>> {
        match &self.access_token {
            Some(token) => Ok(token.clone()),
            None => {
                let authorization_code = self.authorize()?;
                let (access_token, refresh_token) = self.authenticate(&authorization_code)?;
                self.access_token = Some(access_token.clone());
                self.refresh_token = refresh_token;
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
            ],
        )?;
        println!("{}", url.as_str());

        let mut user_provided_token = String::new();
        println!("Go to this url for the auth flow: {}", url.as_str());
        println!("Then, write the authorization code from the redirect url here:");
        io::stdin().read_line(&mut user_provided_token)?;

        #[cfg(debug_assertions)]
        println!("Got: {user_provided_token}");

        Ok(user_provided_token)
    }

    fn authenticate(
        &mut self,
        authorization_code: &String,
    ) -> Result<(String, Option<String>), Box<dyn error::Error>> {
        // TODO: implement authentication flow
        Ok(("placeholder token".to_string(), None))
    }

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
        println!("refresh_token: {:?}", self.refresh_token);
    }
}
