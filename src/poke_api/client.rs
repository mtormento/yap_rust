use reqwest::{Client, StatusCode};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct PokemonInfo {
    pub name: String,
    pub description: String,
    pub habitat: String,
    pub is_legendary: bool,
}

pub struct PokeApiClient {
    http_client: Client,
    base_url: String,
}

impl PokeApiClient {
    pub fn new(base_url: String) -> Self {
        Self {
            http_client: Client::new(),
            base_url,
        }
    }

    pub async fn get_info(&self, name: &str) -> Result<PokemonInfo, PokeApiClientError> {
        let url = format!("{}/pokemon-species/{}", self.base_url, name);
        let response = self.http_client.get(url).send().await?;
        match response.status() {
            StatusCode::OK => {
                let json = response.text().await?;
                self.build_pokemon_info(&json)
            }
            StatusCode::NOT_FOUND => Err(PokeApiClientError::NotFound),
            _ => Err(PokeApiClientError::InternalError),
        }
    }

    fn build_pokemon_info(&self, json: &str) -> Result<PokemonInfo, PokeApiClientError> {
        let parsed = serde_json::from_str::<Value>(json)?;
        let name = parsed["name"].as_str();
        let description = parsed["flavor_text_entries"]
            .as_array()
            .unwrap()
            .into_iter()
            .find(|desc| desc["language"]["name"].as_str().unwrap().eq("en"))
            .unwrap()["flavor_text"]
            .as_str();
        let habitat = parsed["habitat"]["name"].as_str();
        let is_legendary = parsed["is_legendary"].as_bool();
        if name
            .and(description)
            .and(habitat)
            .and(is_legendary)
            .is_some()
        {
            Ok(PokemonInfo {
                name: String::from(name.unwrap()),
                description: String::from(description.unwrap()).replace('\n', " "),
                habitat: String::from(habitat.unwrap()),
                is_legendary: is_legendary.unwrap(),
            })
        } else {
            Err(PokeApiClientError::InternalError)
        }
    }
}

#[derive(Debug)]
pub enum PokeApiClientError {
    NotFound,
    InternalError,
    BadRequest { message: String },
}

impl From<serde_json::Error> for PokeApiClientError {
    fn from(error: serde_json::Error) -> Self {
        PokeApiClientError::InternalError
    }
}

impl From<reqwest::Error> for PokeApiClientError {
    fn from(error: reqwest::Error) -> Self {
        PokeApiClientError::InternalError
    }
}
