use std::fmt::Debug;
use std::num::ParseIntError;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use quick_xml::DeError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::api::gradebook::GradebookError;
use crate::crypto::{AuthToken, CryptoError};
use crate::get_edu_version;

#[cfg(feature = "attendance")]
pub(crate) mod attendance;

#[cfg(feature = "enhanced")]
pub(crate) mod cache;

#[cfg(feature = "schedule")]
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
    Parsing(#[from] DeError),
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
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let code = match &self {
            ApiError::StudentVue(_) | ApiError::EmptyCredentials => StatusCode::BAD_REQUEST,
            ApiError::InvalidCredentials => StatusCode::UNAUTHORIZED,
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

// SLOW AUTH IS DEAD
// async fn rel_auth(req: &ProcessWebServiceRequest) -> Result<String, ApiError> {
//     slow_auth(req).await
// }

// This is probably a bad idea
lazy_static::lazy_static! {
    pub static ref CLIENT: Client = Client::new();
}

pub async fn api_request(
    req: ProcessWebServiceRequest,
    token: &mut AuthToken,
) -> Result<String, ApiError> {
    let res = CLIENT
        .post(format!("https://{}/Service/PXPCommunication.asmx", token.district_url))
        .header(
            "Cookie",
            format!(
                "{}AppSupportsSession=1; edupointkey=1; edupointkeyversion={}",
                token.cookie.as_ref().unwrap_or(&String::new()),
                get_edu_version().await?
            ),
        )
        .header("Content-Type", "text/xml")
        .body(SoapEnvelope::new_request(req).as_string())
        .send()
        .await?;

    if res.status() == StatusCode::METHOD_NOT_ALLOWED {
        Err(ApiError::Maintainance)?
    }

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

    let mut res = res.text().await?;

    // bad workaround :/
    res = res.replace("soap:", "").to_string();

    let resp: Result<SoapEnvelope<SoapBodyResponse>, DeError> =
        quick_xml::de::from_str(res.as_str());
    let resp = resp?;

    if res.contains("ERROR_MESSAGE=") {
        let err: RtError = quick_xml::de::from_str(
            &resp
                .soap_body
                .process_web_service_request_response
                .process_web_service_request_result,
        )?;

        if err.error_message.contains(".dll") {
            return Err(ApiError::Unknown);
        }

        return Err(err.into());
    }

    Ok(resp
        .soap_body
        .process_web_service_request_response
        .process_web_service_request_result)
}

impl SoapEnvelope<SoapBodyRequest> {
    pub fn new_request(body: ProcessWebServiceRequest) -> Self {
        SoapEnvelope {
            xmlns_soap: "http://schemas.xmlsoap.org/soap/envelope/".to_string(),
            xmlns_xsi: "http://www.w3.org/2001/XMLSchema-instance".to_string(),
            xmlns_xsd: "http://www.w3.org/2001/XMLSchema".to_string(),
            soap_body: SoapBodyRequest {
                process_web_service_request: body,
            },
        }
    }

    pub fn as_string(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
        {}
        "#,
            quick_xml::se::to_string(&self).expect("Valid XML Schema")
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Error)]
#[serde(rename = "RT_ERROR")]
#[error("{error_message}")]
pub struct RtError {
    #[serde(rename = "@ERROR_MESSAGE")]
    pub error_message: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename(serialize = "soap:Envelope", deserialize = "Envelope"))]
pub struct SoapEnvelope<T: Serialize + Debug> {
    #[serde(rename = "@xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "@xmlns:xsd")]
    pub xmlns_xsd: String,
    #[serde(rename = "@xmlns:soap")]
    pub xmlns_soap: String,
    #[serde(rename(serialize = "soap:Body", deserialize = "Body"))]
    pub soap_body: T,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SoapBodyRequest {
    #[serde(rename = "ProcessWebServiceRequest")]
    pub process_web_service_request: ProcessWebServiceRequest,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SoapBodyResponse {
    #[serde(rename = "ProcessWebServiceRequestResponse")]
    pub process_web_service_request_response: ProcessWebServiceRequestResponse,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessWebServiceRequest {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    #[serde(rename = "userID")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub user_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub password: String,
    #[serde(rename = "skipLoginLog")]
    pub skip_login_log: String,
    pub parent: String,
    #[serde(rename = "webServiceHandleName")]
    pub web_service_handle_name: String,
    #[serde(rename = "methodName")]
    pub method_name: String,
    #[serde(rename = "paramStr")]
    pub param_str: String,
}

impl ProcessWebServiceRequest {
    fn up_default(
        user_id: String,
        password: String,
        method_name: String,
        params: String,
    ) -> ProcessWebServiceRequest {
        ProcessWebServiceRequest {
            xmlns: "http://edupoint.com/webservices/".to_string(),
            user_id,
            password,
            skip_login_log: "1".to_string(),
            parent: "0".to_string(),
            web_service_handle_name: "PXPWebServices".to_string(),
            method_name,
            param_str: format!("<Parms><ChildIntID>0</ChildIntID>{params}</Parms>"),
        }
    }

    pub fn ck_default(
        method_name: String,
        params: String,
        token: &AuthToken,
    ) -> ProcessWebServiceRequest {
        ProcessWebServiceRequest::up_default(
            token.username.clone(),
            token.password.clone(),
            method_name,
            params,
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessWebServiceRequestResponse {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    #[serde(rename = "ProcessWebServiceRequestResult")]
    pub process_web_service_request_result: String,
}
