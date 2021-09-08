use reqwest::{Client, StatusCode};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
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

    pub async fn get_pokemon_info(&self, name: &str) -> Result<PokemonInfo, PokeApiClientError> {
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

#[derive(Debug, PartialEq)]
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

#[cfg(test)]
mod tests {
    use claim::{assert_err, assert_ok};
    use serde_json::json;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::{method, path}};
    use fake::{Fake, Faker};

    use crate::poke_api::client::{PokeApiClient, PokeApiClientError};

    #[tokio::test]
    async fn get_pokemon_info_fires_a_request_to_base_url() {
        // Arrange
        let mock_server = MockServer::start().await;
        let poke_api_client = PokeApiClient::new(mock_server.uri());
        
        //let json_body = "{\"flavor_text_entries\":[{\"flavor_text\":\"It was created by\\na scientist after\\nyears of horrific\\fgene splicing and\\nDNA engineering\\nexperiments.\",\"language\":{\"name\":\"en\",\"url\":\"https:\\/\\/pokeapi.co\\/api\\/v2\\/language\\/9\\/\"},\"version\":{\"name\":\"red\",\"url\":\"https:\\/\\/pokeapi.co\\/api\\/v2\\/version\\/1\\/\"}}],\"habitat\":{\"name\":\"rare\",\"url\":\"https:\\/\\/pokeapi.co\\/api\\/v2\\/pokemon-habitat\\/5\\/\"},\"is_legendary\":true,\"name\":\"mewtwo\"}";
        let json_body = json!({"flavor_text_entries":[{"flavor_text":"It was created by a scientist after years of horrific gene splicing and DNA engineering experiments.","language":{"name":"en","url":"https://pokeapi.co/api/v2/language/9/"},"version":{"name":"red","url":"https://pokeapi.co/api/v2/version/1/"}}],"habitat":{"name":"rare","url":"https://pokeapi.co/api/v2/pokemon-habitat/5/"},"is_legendary":true,"name":"mewtwo"});
        
        let pokemon = Faker.fake::<String>();
        Mock::given(path(format!("/pokemon-species/{}", &pokemon)))
            .and(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let info = poke_api_client
            .get_pokemon_info(&pokemon)
            .await;
        
        assert_ok!(&info);
        let info = info.unwrap();
        assert_eq!(info.name, "mewtwo");
        assert_eq!(info.habitat, "rare");
        assert_eq!(info.is_legendary, true);
        assert_eq!(info.description, "It was created by a scientist after years of horrific gene splicing and DNA engineering experiments.");
    }

    #[tokio::test]
    async fn get_pokemon_info_fails_if_the_server_returns_404() {
        // Arrange
        let mock_server = MockServer::start().await;
        let poke_api_client = PokeApiClient::new(mock_server.uri());
        
        let pokemon = Faker.fake::<String>();
        Mock::given(path(format!("/pokemon-species/{}", &pokemon)))
            .and(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&mock_server)
            .await;
            
        // Act
        let info = poke_api_client
            .get_pokemon_info(&pokemon)
            .await;
        
        assert_err!(&info);
        let error = info.unwrap_err();
        assert_eq!(error, PokeApiClientError::NotFound);
    }
}