use reqwest::{Client, StatusCode};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
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
    pub fn new(base_url: String) -> Self {
        Self {
            http_client: Client::new(),
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

#[derive(Debug)]
pub enum FunTranslationsApiClientError {
    InternalError,
    BadRequest { message: String },
}

impl From<serde_json::Error> for FunTranslationsApiClientError {
    fn from(error: serde_json::Error) -> Self {
        FunTranslationsApiClientError::InternalError
    }
}

impl From<reqwest::Error> for FunTranslationsApiClientError {
    fn from(error: reqwest::Error) -> Self {
        FunTranslationsApiClientError::InternalError
    }
}
