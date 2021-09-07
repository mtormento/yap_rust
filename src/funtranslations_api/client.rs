use reqwest::Client;

pub struct FunTranslationsApiClient {
    pub http_client: Client,
    pub base_url: String,
}

impl FunTranslationsApiClient {
    pub fn new(base_url: String) -> Self {
        Self {
            http_client: Client::new(),
            base_url,
        }
    }
}
