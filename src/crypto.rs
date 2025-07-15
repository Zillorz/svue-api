use std::{
    string::FromUtf8Error,
    time::{SystemTime, UNIX_EPOCH},
};

use aes_gcm_siv::{
    aead::{Aead, OsRng},
    AeadCore, Aes128GcmSiv, KeyInit, Nonce,
};
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::api::ApiError;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error(transparent)]
    InvalidCipher(#[from] CipherError),
    #[error(transparent)]
    CryptError(#[from] aes_gcm_siv::aead::Error),
    #[error("The crypto key was invalid")]
    InvalidKey(#[from] crypto_common::InvalidLength),
    #[error("No key found :/")]
    Nokey,
}

#[derive(Error, Debug)]
pub enum CipherError {
    #[error("Invalid length")]
    Length,
    #[error("Invalid string")]
    Decoding(#[from] FromUtf8Error),
}

pub fn create_token(cipher_text: String) -> Result<Vec<u8>, CryptoError> {
    let key = BASE64_STANDARD.decode(
        std::env::var("ENKEY").map_err(|_| CryptoError::Nokey)?
    ).map_err(|_| CryptoError::Nokey)?;

    let cipher = Aes128GcmSiv::new_from_slice(&key)?;
    let nonce = Aes128GcmSiv::generate_nonce(&mut OsRng);

    let encrypted = cipher.encrypt(&nonce, cipher_text.as_bytes())?;

    let mut ret = nonce.to_vec();
    ret.extend(encrypted);

    Ok(ret)
}

pub fn try_decrypt_token(encrypted: &[u8]) -> Result<String, CryptoError> {
    let key = BASE64_STANDARD.decode(
        std::env::var("ENKEY").map_err(|_| CryptoError::Nokey)?
    ).map_err(|_| CryptoError::Nokey)?;

    if encrypted.len() <= 12 {
        return Err(CipherError::Length.into());
    }

    let nonce = Nonce::from_slice(&encrypted[..12]);
    let cipher = Aes128GcmSiv::new_from_slice(&key)?;
    let cookie = cipher.decrypt(nonce, &encrypted[12..])?;

    Ok(String::from_utf8(cookie).map_err(CipherError::Decoding)?)
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct AuthToken {
    pub username: String,
    pub password: String,
    pub cookie: Option<String>,

    // if at this point, kill
    #[serde(with = "string")]
    pub expiry: u128,

    pub district_url: String
}

mod string {
    use std::fmt::Display;
    use std::str::FromStr;

    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

fn get_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
}

impl AuthToken {
    pub fn is_empty(&self) -> bool {
        self.username.is_empty() || self.password.is_empty()
    }
}

impl<S: Send + Sync> FromRequestParts<S> for AuthToken {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _s: &S) -> Result<Self, Self::Rejection> {
        let authorization = parts
            .headers
            .get(AUTHORIZATION)
            .ok_or(ApiError::EmptyCredentials)?
            .to_str()
            .map_err(|_| ApiError::InvalidCredentials)?;

        let split = authorization.split_once(' ');
        match split {
            Some(("Bearer", contents)) => {
                let json = try_decrypt_token(
                    &BASE64_STANDARD
                        .decode(contents)
                        .map_err(|_| ApiError::InvalidCredentials)?,
                )?;

                let ret = serde_json::from_str(&json).map_err(|_| ApiError::InvalidCredentials)?;
                check_validity(ret)
            }
            Some(("Basic", contents)) => {
                let decoded = String::from_utf8(
                    BASE64_STANDARD
                        .decode(contents)
                        .map_err(|_| ApiError::InvalidCredentials)?,
                )
                .map_err(|_| ApiError::InvalidCredentials)?;

                let (username, password) = decoded
                    .split_once(':')
                    .ok_or(ApiError::InvalidCredentials)?;

                Ok(AuthToken {
                    username: username.to_string(),
                    password: password.to_string(),
                    cookie: None,
                    expiry: get_timestamp() + 1000 * 60 * 60 * 24,
                    // in the future, use this to support other districts
                    district_url: "md-mcps-psv.edupoint.com".to_string()
                })
            }
            _ => Err(ApiError::InvalidCredentials),
        }
    }
}

fn check_validity(
    token: AuthToken,
) -> Result<AuthToken, ApiError> {
    if get_timestamp() > token.expiry {
        Err(ApiError::ExpiredKey)?
    }

    Ok(token)
}
