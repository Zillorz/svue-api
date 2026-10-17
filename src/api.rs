use std::fmt::Debug;
use std::num::ParseIntError;

use axum::http::header::AUTHORIZATION;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use reqwest::{Client, Response};
use serde::de::{DeserializeOwned, IgnoredAny};
use serde::Deserialize;
use serde_json::{json, Value};
use thiserror::Error;

use crate::api::gradebook::GradebookError;
use crate::crypto::{AuthToken, CryptoError};
use crate::get_edu_version;

pub(crate) mod attendance;

#[cfg(feature = "enhanced")]
pub(crate) mod cache;

pub(crate) mod schedule;

pub(crate) mod documents;
pub(crate) mod gradebook;
pub(crate) mod school_info;

#[cfg(feature = "enhanced")]
mod scraper;

pub(crate) mod student_info;

pub fn base64_mangle<T: std::error::Error>(inp: T) -> String {
    BASE64_STANDARD.encode(inp.to_string().as_bytes())
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Cannot parse response: {}", base64_mangle(.0))]
    Parsing(#[from] serde_json::Error),
    #[error("Unable to load Gradebook, message: {0}")]
    Gradebook(#[from] GradebookError),
    #[error("Unable to load Attendance, message: invalid term idx")]
    Attendance(#[from] ParseIntError),
    #[error("Unable to reach StudentVue")]
    Network(#[from] reqwest::Error),
    #[error("{0}")]
    StudentVue(#[from] SvError),
    #[error("Username or password is empty")]
    EmptyCredentials,

    // We don't scrape anymore
    #[cfg(feature = "enhanced")]
    #[error("Failed to login to StudentVue, code: {0}")]
    Scraping(#[from] scraper::ScrapingError),

    #[error("Unknown error (code: x_dll)")]
    Unknown,
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error("StudentVue is currently undergoing maintenance")]
    Maintainance,
    #[error("Invalid root req")]
    InvalidRoot,
    #[error("Invalid credentials provided")]
    InvalidCredentials,
    #[error("Unable to create access key")]
    AccessKey,
    #[error("This key has expired")]
    ExpiredKey,
    #[error("Security failed - do you have a user agent?")]
    NoSecureResponse,
    #[error("Not found")]
    NotFound,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let code = match &self {
            ApiError::StudentVue(_) | ApiError::EmptyCredentials => StatusCode::BAD_REQUEST,
            ApiError::InvalidCredentials | ApiError::ExpiredKey => StatusCode::UNAUTHORIZED,
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::Crypto(crypto) => match crypto {
                CryptoError::InvalidCipher(_) | CryptoError::CryptError(_) => {
                    StatusCode::BAD_REQUEST
                }
                CryptoError::InvalidKey(_) | CryptoError::Nokey => StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (code, self.to_string()).into_response()
    }
}

// This is probably a bad idea
lazy_static::lazy_static! {
    pub static ref CLIENT: Client = Client::new();
}

#[derive(Deserialize, Debug, Error)]
#[error("{message}")]
pub struct SvError {
    pub code: String,
    pub message: String,
}

#[derive(Deserialize)]
struct Envelope<T> {
    error: Option<SvError>,
    data: Option<T>,
}

#[derive(Deserialize)]
struct Tokens {
    access_token: String,
    refresh_token: String,
}

async fn post(
    token: &AuthToken,
    method: &str,
    authorization: String,
    body: String,
) -> Result<Response, ApiError> {
    Ok(CLIENT
        .post(format!(
            "https://{}/api/v1/mobile/PXPWebServices/{method}",
            token.district_url
        ))
        .header("Content-Type", "application/json")
        .header("User-Agent", "ksoap")
        .header("edupointkeyversion", get_edu_version().await?)
        .header(AUTHORIZATION, authorization)
        .body(body)
        .send()
        .await?)
}

async fn login(token: &mut AuthToken) -> Result<(), ApiError> {
    let (username, password) = token.credentials.as_ref().ok_or(ApiError::ExpiredKey)?;
    let basic = BASE64_STANDARD.encode(format!("{username}:{password}"));
    let body = json!({"arguments": {"request": json!({
        "userID": null, "password": null, "userType": "Student"
    }).to_string()}});

    let text = post(token, "AttemptLogin", format!("Basic {basic}"), body.to_string())
        .await?
        .error_for_status()?
        .text()
        .await?;

    if let Ok(tokens) = serde_json::from_str::<Tokens>(&text) {
        token.access_token = tokens.access_token;
        token.refresh_token = tokens.refresh_token;
        return Ok(());
    }

    match serde_json::from_str::<Envelope<IgnoredAny>>(&text)?.error {
        Some(err) if err.code == "401" || err.message.contains("password") => {
            Err(ApiError::InvalidCredentials)
        }
        Some(err) => Err(err.into()),
        None => Err(ApiError::Unknown),
    }
}

async fn refresh(token: &mut AuthToken) -> Result<(), ApiError> {
    let res = post(
        token,
        "RefreshToken",
        format!("Bearer {}", token.refresh_token),
        "{}".to_string(),
    )
    .await?;

    let tokens: Tokens = serde_json::from_str(&res.text().await?).map_err(|_| ApiError::ExpiredKey)?;
    token.access_token = tokens.access_token;
    token.refresh_token = tokens.refresh_token;
    Ok(())
}

pub async fn api_request<T: DeserializeOwned>(
    method: &str,
    params: Value,
    token: &mut AuthToken,
) -> Result<T, ApiError> {
    if token.access_token.is_empty() {
        login(token).await?;
    }

    let body = json!({"arguments": {"request": params.to_string()}}).to_string();

    let mut res = post(token, method, format!("Bearer {}", token.access_token), body.clone()).await?;
    if res.status() == StatusCode::UNAUTHORIZED {
        refresh(token).await?;
        res = post(token, method, format!("Bearer {}", token.access_token), body).await?;
    }

    if res.status() == StatusCode::UNAUTHORIZED {
        Err(ApiError::ExpiredKey)?
    }

    let envelope: Envelope<T> = serde_json::from_str(&res.error_for_status()?.text().await?)?;
    match envelope {
        Envelope { error: Some(err), .. } => Err(err.into()),
        Envelope { data: Some(data), .. } => Ok(data),
        _ => Err(ApiError::Unknown),
    }
}
