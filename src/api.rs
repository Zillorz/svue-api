use std::fmt::Debug;
use std::num::ParseIntError;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use base58::ToBase58;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::api::gradebook::GradebookError;
use crate::crypto::{AuthToken, CryptoError};
use crate::get_edu_version;

pub(crate) mod documents;
pub(crate) mod gradebook;
pub(crate) mod school_info;
pub(crate) mod student_info;

pub fn base64_mangle<T: std::error::Error>(inp: T) -> String {
    inp.to_string().as_bytes().to_base58()
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
    StudentVue(#[from] RtError),
    #[error("Username or password is empty")]
    EmptyCredentials,
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
    #[error("Try again soon, updating backend (code: ood)")]
    Version,

    #[error("Resource not found")]
    NotFound,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let code = match &self {
            ApiError::StudentVue(_) | ApiError::EmptyCredentials => StatusCode::BAD_REQUEST,
            ApiError::InvalidCredentials | ApiError::ExpiredKey => StatusCode::UNAUTHORIZED,
            ApiError::Crypto(crypto) => match crypto {
                CryptoError::InvalidCipher(_) | CryptoError::CryptError(_) => {
                    StatusCode::BAD_REQUEST
                }
                CryptoError::InvalidKey(_) | CryptoError::Nokey => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            },
            ApiError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (code, self.to_string()).into_response()
    }
}

// SLOW AUTH IS DEAD (rip)
// This is probably a bad idea
lazy_static::lazy_static! {
    pub static ref CLIENT: Client = Client::new();
}

#[derive(Serialize, Deserialize)]
struct LoginResp {
    access_token: String,
    refresh_token: String,
}

pub async fn login(token: &mut AuthToken) -> Result<(), ApiError> {
    let auth = STANDARD.encode(format!("{}:{}", token.username, token.password));

    let resp = CLIENT
        .post(format!(
            "https://{}/api/v1/mobile/PXPWebServices/AttemptLogin",
            token.district_url
        ))
        .header("Content-Type", "application/json")
        .header("edupointkeyversion", get_edu_version().await?)
        .header("Authorization", format!("Basic {}", auth))
        .body(serde_json::to_string(&ApiReq {
            arguments: ApiReqInner {
                request: "{\"userID\":\"\",\"password\":\"\",\"userType\":\"Student\"}".to_string(),
            },
        })?)
        .send()
        .await?
        .json::<LoginResp>()
        .await?;

    token.access_token = Some(resp.access_token);
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct ApiResp {
    data: Value,
}

#[derive(Serialize, Deserialize)]
struct ApiReq {
    arguments: ApiReqInner,
}

#[derive(Serialize, Deserialize)]
struct ApiReqInner {
    request: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DefaultReq {
    #[serde(rename = "childIntID")]
    child_int_id: i32,
    langauge_code: String,
}

impl Default for DefaultReq {
    fn default() -> Self {
        DefaultReq {
            child_int_id: 0,
            langauge_code: "ENG".to_string(),
        }
    }
}

pub async fn api_req<I: Serialize, T: DeserializeOwned>(
    path: &str,
    body: I,
    token: &mut AuthToken,
) -> Result<T, ApiError> {
    if token.access_token.is_none() {
        login(token).await?;
    }

    let Some(at) = &token.access_token else {
        panic!()
    };

    let res = CLIENT
        .post(format!(
            "https://{}/api/v1/mobile/PXPWebServices/{}",
            token.district_url, path
        ))
        .header("Authorization", format!("Bearer {at}"))
        .header("edupointkeyversion", get_edu_version().await?)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&ApiReq {
            arguments: ApiReqInner {
                request: serde_json::to_string(&body)?,
            },
        })?)
        .send()
        .await?;

    let mut cookies = String::new();
    for header in res.headers().get_all("Set-Cookie") {
        cookies += header
            .to_str()
            .unwrap_or_default()
            .split_whitespace()
            .next()
            .unwrap_or_default();
        cookies += " ";
    }

    if !cookies.is_empty() {
        token.cookie = Some(cookies);
    }

    let res: ApiResp = res.json().await?;
    Ok(serde_json::from_value(res.data)?)
}

#[derive(Serialize, Deserialize, Debug, Error)]
#[serde(rename = "RT_ERROR")]
#[error("{error_message}")]
pub struct RtError {
    #[serde(rename = "@ERROR_MESSAGE")]
    pub error_message: String,
}
