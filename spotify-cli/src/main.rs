use serde_json::Value;
use std::error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let res = reqwest::get("https://pokeapi.co/api/v2/pokemon/charizard/").await?;
    let parsed_res: Value = serde_json::from_str(res.text().await?.as_str())?;
    println!("PokéAPI result:");
    println!("id {}", parsed_res["id"]);
    println!("name {}", parsed_res["name"]);
    println!("weight {}", parsed_res["weight"]);
    Ok(())
}
