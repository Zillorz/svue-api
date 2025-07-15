use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::Query;
use axum::http::{header, HeaderMap, HeaderName, HeaderValue};
use axum::routing::get;
use axum::{Json, Router as AxumRouter};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;

use crate::api::documents::Document;
use crate::api::school_info::SchoolInfo;
use crate::api::student_info::StudentInfo;
use crate::api::{documents, gradebook, school_info, student_info, ApiError};
use crate::crypto::AuthToken;

#[cfg(feature = "schedule")]
use crate::api::schedule;

#[cfg(feature = "attendance")]
use crate::api::attendance;

mod api;
mod crypto;

#[cfg(feature = "enhanced")]
mod db;

#[cfg(feature = "enhanced")]
mod deserializer;

#[cfg(feature = "access")]
mod access;

#[cfg(feature = "enhanced")]
mod advanced;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;


async fn get_version_key() -> Option<String> {
    #[cfg(feature = "access")]
    {
        return access::generate_version_key(&std::env::var("VERSION_NUMBER").ok()?).ok();
    }

    #[allow(unreachable_code)]

    // ask the api for it
    reqwest::get("https://axum-svue.fly.dev/akey").await.ok()?.text().await.ok()
}

pub async fn get_edu_version() -> Result<String, ApiError> {
    get_version_key().await.ok_or(ApiError::AccessKey)
}

type Resp<T> = Result<(HeaderMap, Json<T>), ApiError>;

async fn get_data<T: Serialize>(
    mut token: AuthToken,
    fetch: impl for<'a> AsyncFnOnce(&'a mut AuthToken) -> Result<T, ApiError>,
) -> Resp<T> {
    if token.is_empty() {
        Err(ApiError::EmptyCredentials)?
    }
    let old = token.clone();

    let data = fetch(&mut token).await?;

    let mut hm = HeaderMap::new();
    if old != token {
        let enc = serde_json::to_string(&token).map_err(|_| ApiError::Unknown)?;
        let tok = BASE64_STANDARD.encode(crypto::create_token(enc)?);

        hm.insert(
            HeaderName::from_static("set-token"),
            HeaderValue::from_str(&tok).unwrap(),
        );
    }

    Ok((hm, Json(data)))
}

#[derive(Deserialize)]
struct GradeReq {
    report_period: Option<i32>,
}

async fn grades(token: AuthToken, req: Query<GradeReq>) -> Resp<gradebook::Response> {
    get_data(token, async |t: &mut AuthToken| {
        return gradebook::get_grade_book(t, req.report_period).await;
    })
    .await
}

#[cfg(feature = "attendance")]
async fn attendance(token: AuthToken) -> Resp<attendance::Response> {
    get_data(token, attendance::get_attendance).await
}

async fn documents(token: AuthToken) -> Resp<Vec<Document>> {
    get_data(token, documents::list_documents).await
}

#[derive(Deserialize)]
struct DocReq {
    gu: String,
}

// needs old format :/
async fn document(
    mut token: AuthToken,
    Query(dr): Query<DocReq>,
) -> Result<(HeaderMap, Body), ApiError> {
    if token.is_empty() {
        Err(ApiError::EmptyCredentials)?
    }
    let old = token.clone();

    let document = documents::get_document(&mut token, dr.gu).await?;
    let mut headers = HeaderMap::new();

    if old != token {
        let enc = serde_json::to_string(&token).map_err(|_| ApiError::Unknown)?;
        let tok = BASE64_STANDARD.encode(crypto::create_token(enc)?);
        headers.insert(
            HeaderName::from_static("Set-Token"),
            HeaderValue::from_str(&tok).unwrap(),
        );
    }

    if document.file_name.to_lowercase().ends_with(".pdf") {
        let dep = format!("inline; filename=\"{}\"", document.file_name);
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/pdf"),
        );
        headers.insert(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&dep).unwrap(),
        );
    } else {
        let dep = format!("attachment; filename=\"{}\"", document.file_name);
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
        headers.insert(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&dep).unwrap(),
        );
    };

    Ok((headers, Body::from(document.file_data)))
}

async fn student_info(token: AuthToken) -> Resp<StudentInfo> {
    get_data(token, student_info::student_info).await
}


// old format for this too
async fn student_photo(mut token: AuthToken) -> Result<(HeaderMap, Body), ApiError> {
    if token.is_empty() {
        Err(ApiError::EmptyCredentials)?
    }
    let old = token.clone();

    let bytes = student_info::photo(&mut token).await?;
    let mut headers = HeaderMap::new();

    if old != token {
        let enc = serde_json::to_string(&token).map_err(|_| ApiError::Unknown)?;
        let tok = BASE64_STANDARD.encode(crypto::create_token(enc)?);
        headers.insert(
            HeaderName::from_static("Set-Token"),
            HeaderValue::from_str(&tok).unwrap(),
        );
    }

    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"image.png\""),
    );

    Ok((headers, Body::from(bytes)))
}


async fn school_info(token: AuthToken) -> Resp<SchoolInfo> {
    get_data(token, school_info::school_info).await
}

#[cfg(feature = "schedule")]
#[derive(Deserialize)]
struct ScheduleReq {
    term_index: Option<i32>,
}

#[cfg(feature = "schedule")]
async fn schedule(
    token: AuthToken,
    Query(req): Query<ScheduleReq>,
) -> Resp<schedule::Schedule> {
    get_data(token, async |t: &mut AuthToken| {
        return schedule::schedule(t, req.term_index).await;
    })
    .await
}

#[tokio::main]
pub async fn main() {
    let mut router = AxumRouter::new()
        .route("/grades", get(grades))
        .route("/documents", get(documents))
        .route("/document", get(document))
        .route("/student", get(student_info))
        .route("/photo", get(student_photo))
        .route("/school", get(school_info)); 
        
    #[cfg(feature = "schedule")]
    {
        router = router.route("/schedule", get(schedule)); 
    }

    #[cfg(feature = "attendance")]
    {
        router = router.route("/attendance", get(attendance));
    }

    #[cfg(feature = "enhanced")]
    {
        router = router.merge(advanced::ext());
    }

    router = router.layer(CorsLayer::very_permissive().expose_headers([HeaderName::from_static("set-token")]))
        .layer(CompressionLayer::new().br(true));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:2727").await.unwrap();
    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}
