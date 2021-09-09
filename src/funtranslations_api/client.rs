use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize, Debug)]
pub struct Translation {
    pub dialect: String,
    pub original: String,
    pub translated: String,
}

pub struct FunTranslationsApiClient {
    http_client: Client,
    base_url: String,
}

impl FunTranslationsApiClient {
    pub fn new(base_url: String, timeout: Duration) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            http_client,
            base_url,
        }
    }

    pub async fn translate(
        &self,
        dialect: &str,
        text: &str,
    ) -> Result<Translation, FunTranslationsApiClientError> {
        let url = format!("{}/translate/{}.json", self.base_url, dialect);
        let response = self
            .http_client
            .get(url)
            .query(&[("text", text)])
            .send()
            .await?;
        match response.status() {
            StatusCode::OK => {
                let json = response.text().await?;
                self.build_translation(&json)
            }
            StatusCode::NOT_FOUND => Err(FunTranslationsApiClientError::NotFound),
            _ => Err(FunTranslationsApiClientError::InternalError),
        }
    }

    fn build_translation(&self, json: &str) -> Result<Translation, FunTranslationsApiClientError> {
        let parsed = serde_json::from_str::<Value>(json)?;
        let total = parsed["success"]["total"].as_u64();
        if let Some(total) = total {
            if total > 0 {
                let contents = &parsed["contents"];
                let translated = contents["translated"].as_str();
                let text = contents["text"].as_str();
                let translation = contents["translation"].as_str();
                if let (Some(translated), Some(text), Some(translation)) =
                    (translated, text, translation)
                {
                    Ok(Translation {
                        dialect: String::from(translation),
                        original: String::from(text),
                        translated: String::from(translated),
                    })
                } else {
                    Err(FunTranslationsApiClientError::InternalError)
                }
            } else {
                Err(FunTranslationsApiClientError::InternalError)
            }
        } else {
            Err(FunTranslationsApiClientError::InternalError)
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum FunTranslationsApiClientError {
    InternalError,
    NotFound,
    BadRequest { message: String },
}

impl From<serde_json::Error> for FunTranslationsApiClientError {
    fn from(error: serde_json::Error) -> Self {
        FunTranslationsApiClientError::InternalError
    }
}

impl From<reqwest::Error> for FunTranslationsApiClientError {
    fn from(error: reqwest::Error) -> Self {
        if let Some(StatusCode::NOT_FOUND) = error.status() {
            FunTranslationsApiClientError::NotFound
        } else {
            FunTranslationsApiClientError::InternalError
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use claim::{assert_err, assert_ok};
    use fake::{Fake, Faker};
    use serde_json::json;
    use wiremock::{
        matchers::{any, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use crate::funtranslations_api::client::{
        FunTranslationsApiClient, FunTranslationsApiClientError,
    };

    #[tokio::test]
    async fn translate_fires_a_request_to_base_url() {
        // Arrange
        let mock_server = MockServer::start().await;
        let funtranslations_api_client =
            FunTranslationsApiClient::new(mock_server.uri(), Duration::from_millis(200));

        //let json_body = "{\"flavor_text_entries\":[{\"flavor_text\":\"It was created by\\na scientist after\\nyears of horrific\\fgene splicing and\\nDNA engineering\\nexperiments.\",\"language\":{\"name\":\"en\",\"url\":\"https:\\/\\/pokeapi.co\\/api\\/v2\\/language\\/9\\/\"},\"version\":{\"name\":\"red\",\"url\":\"https:\\/\\/pokeapi.co\\/api\\/v2\\/version\\/1\\/\"}}],\"habitat\":{\"name\":\"rare\",\"url\":\"https:\\/\\/pokeapi.co\\/api\\/v2\\/pokemon-habitat\\/5\\/\"},\"is_legendary\":true,\"name\":\"mewtwo\"}";
        let json_body = json!({
            "success": {
              "total": 1
            },
            "contents": {
              "translated": "Lost a planet,  master obiwan has.",
              "text": "Master Obiwan has lost a planet.",
              "translation": "yoda"
            }
          }
        );

        let dialect = Faker.fake::<String>();
        let text = "Master Obiwan has lost a planet.";
        Mock::given(path(format!("/translate/{}.json", &dialect)))
            .and(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let translation = funtranslations_api_client.translate(&dialect, &text).await;

        assert_ok!(&translation);
        let translation = translation.unwrap();
        assert_eq!(translation.dialect, "yoda");
        assert_eq!(translation.original, "Master Obiwan has lost a planet.");
        assert_eq!(translation.translated, "Lost a planet,  master obiwan has.");
    }

    #[tokio::test]
    async fn translate_fails_if_the_server_returns_404() {
        // Arrange
        let mock_server = MockServer::start().await;
        let funtranslations_api_client =
            FunTranslationsApiClient::new(mock_server.uri(), Duration::from_millis(200));

        let dialect = Faker.fake::<String>();
        let text = Faker.fake::<String>();
        Mock::given(path(format!("/translate/{}.json", &dialect)))
            .and(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let translation = funtranslations_api_client.translate(&dialect, &text).await;

        assert_err!(&translation);
        let error = translation.unwrap_err();
        assert_eq!(error, FunTranslationsApiClientError::NotFound);
    }

    #[tokio::test]
    async fn translate_fails_if_the_server_returns_500() {
        // Arrange
        let mock_server = MockServer::start().await;
        let funtranslations_api_client =
            FunTranslationsApiClient::new(mock_server.uri(), Duration::from_millis(200));

        let dialect = Faker.fake::<String>();
        let text = Faker.fake::<String>();
        Mock::given(path(format!("/translate/{}.json", &dialect)))
            .and(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let translation = funtranslations_api_client.translate(&dialect, &text).await;

        assert_err!(&translation);
        let error = translation.unwrap_err();
        assert_eq!(error, FunTranslationsApiClientError::InternalError);
    }

    #[tokio::test]
    async fn get_pokemon_info_fails_if_the_server_take_too_much_time() {
        // Arrange
        let mock_server = MockServer::start().await;
        let funtranslations_api_client =
            FunTranslationsApiClient::new(mock_server.uri(), Duration::from_millis(200));

        let json_body = json!({
            "success": {
              "total": 1
            },
            "contents": {
              "translated": "Lost a planet,  master obiwan has.",
              "text": "Master Obiwan has lost a planet.",
              "translation": "yoda"
            }
          }
        );
        let response = ResponseTemplate::new(200)
            .set_body_json(json_body)
            .set_delay(Duration::from_secs(180));
        let text = Faker.fake::<String>();
        let dialect = Faker.fake::<String>();

        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let info = funtranslations_api_client.translate(&dialect, &text).await;

        assert_err!(&info);
    }
}
