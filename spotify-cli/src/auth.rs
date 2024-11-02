use std::{env, error};

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

    pub fn get_access_token(&self) -> Result<String, ()> {
        match &self.access_token {
            Some(token) => Ok(token.clone()),
            None => {
                // TODO: Auth/refresh flow
                Err(())
            }
        }
    }

    fn authorize(&mut self) -> Result<(), ()> {
        // TODO: authorization flow
        Err(())
    }

    fn refresh(&mut self) -> Result<(), ()> {
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
