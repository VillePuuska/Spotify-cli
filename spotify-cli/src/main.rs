mod auth;

use auth::SpotifyAuth;
use serde_json::Value;
use std::error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    // let res = reqwest::get("https://pokeapi.co/api/v2/pokemon/charizard/").await?;
    // let parsed_res: Value = serde_json::from_str(res.text().await?.as_str())?;
    // println!("PokéAPI result:");
    // println!("id {}", parsed_res["id"]);
    // println!("name {}", parsed_res["name"]);
    // println!("weight {}", parsed_res["weight"]);

    let mut a = SpotifyAuth::new()?;

    #[cfg(debug_assertions)]
    a.print_auth_info();

    println!("get_access_token result: {:?}", a.get_access_token());

    Ok(())
}
