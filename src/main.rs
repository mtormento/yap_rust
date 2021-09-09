mod funtranslations_api;
mod poke_api;

use actix_web::{
    get, http,
    http::header,
    web::{self, Data},
    App, HttpResponse, HttpResponseBuilder, HttpServer, ResponseError,
};
use funtranslations_api::client::{FunTranslationsApiClient, FunTranslationsApiClientError};
use poke_api::client::{PokeApiClient, PokeApiClientError};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    time::Duration,
};

#[derive(Deserialize)]
struct PathParams {
    name: String,
}

#[derive(Debug, Serialize)]
struct PokeError {
    #[serde(skip_serializing)]
    status_code: u16,
    code: String,
    message: String,
}

impl Display for PokeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl ResponseError for PokeError {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::from_u16(self.status_code).unwrap()
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code())
            .insert_header(header::ContentType(mime::APPLICATION_JSON))
            .body(serde_json::to_string(&self).unwrap())
    }
}

impl From<PokeApiClientError> for PokeError {
    fn from(error: PokeApiClientError) -> Self {
        match error {
            PokeApiClientError::BadRequest { message } => PokeError {
                status_code: http::StatusCode::BAD_REQUEST.as_u16(),
                code: String::from("PE_BAD_REQUEST"),
                message: message,
            },
            PokeApiClientError::InternalError => PokeError {
                status_code: http::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                code: String::from("PE_INTERNAL"),
                message: String::from("internal error"),
            },
            PokeApiClientError::NotFound => PokeError {
                status_code: http::StatusCode::NOT_FOUND.as_u16(),
                code: String::from("PE_NOT_FOUND"),
                message: String::from("pokemon not found"),
            },
        }
    }
}

impl From<FunTranslationsApiClientError> for PokeError {
    fn from(error: FunTranslationsApiClientError) -> Self {
        match error {
            FunTranslationsApiClientError::BadRequest { message } => PokeError {
                status_code: http::StatusCode::BAD_REQUEST.as_u16(),
                code: String::from("PE_BAD_REQUEST"),
                message: message,
            },
            FunTranslationsApiClientError::NotFound => PokeError {
                status_code: http::StatusCode::NOT_FOUND.as_u16(),
                code: String::from("PE_NOT_FOUND"),
                message: String::from("not found"),
            },
            FunTranslationsApiClientError::InternalError => PokeError {
                status_code: http::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                code: String::from("PE_INTERNAL"),
                message: String::from("internal error"),
            },
        }
    }
}

#[get("/pokemon/{name}")]
async fn get_pokemon_info(
    info: web::Path<PathParams>,
    poke_api_client: web::Data<PokeApiClient>,
) -> Result<HttpResponse, PokeError> {
    let pokemon_info = poke_api_client.get_pokemon_info(&info.name).await?;
    Ok(HttpResponse::Ok().json(pokemon_info))
}

#[get("/pokemon/translated/{name}")]
async fn get_pokemon_info_translated(
    info: web::Path<PathParams>,
    poke_api_client: web::Data<PokeApiClient>,
    funtranslations_api_client: web::Data<FunTranslationsApiClient>,
) -> Result<HttpResponse, PokeError> {
    let mut pokemon_info = poke_api_client.get_pokemon_info(&info.name).await?;
    let mut dialect = "shakespeare";
    if pokemon_info.habitat == "cave" || pokemon_info.is_legendary {
        dialect = "yoda";
    }
    let translation = funtranslations_api_client
        .translate(dialect, &pokemon_info.description)
        .await?;
    pokemon_info.description = String::from(translation.translated);
    Ok(HttpResponse::Ok().json(pokemon_info))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let poke_api_client = Data::new(PokeApiClient::new(
        String::from("https://pokeapi.co/api/v2"),
        Duration::from_secs(10),
    ));
    let funtranslations_api_client = Data::new(FunTranslationsApiClient::new(
        String::from("https://api.funtranslations.com"),
        Duration::from_secs(10),
    ));

    HttpServer::new(move || {
        App::new()
            .service(get_pokemon_info)
            .service(get_pokemon_info_translated)
            .app_data(poke_api_client.clone())
            .app_data(funtranslations_api_client.clone())
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
