use jsonwebtoken::{
    decode, encode, errors::ErrorKind, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::GLOBAL_CONFIG;
use crate::error::ResponseError;

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub user_id: i32,
    iat: u64,
    exp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    pub user_id: i32,
    pub password: String,
    iat: u64,
    exp: u64,
}

impl AccessTokenClaims {
    pub fn new(user_id: i32) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            user_id,
            iat: now,
            exp: now + GLOBAL_CONFIG.jwt.access_token_expire,
        }
    }
}

impl RefreshTokenClaims {
    pub fn new(user_id: i32, password: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            user_id,
            password: password.to_string(),
            iat: now,
            exp: now + GLOBAL_CONFIG.jwt.refresh_token_expire,
        }
    }
}

pub fn parse_access_token(jwt: &str) -> Result<AccessTokenClaims, ResponseError> {
    let token = decode::<AccessTokenClaims>(
        &jwt,
        &DecodingKey::from_secret(GLOBAL_CONFIG.jwt.token_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| match e.kind() {
        &ErrorKind::ExpiredSignature => {
            ResponseError::new_expire_token_error("access_token已过期", Some("用户访问凭证已过期"))
        }
        _ => e.into(),
    })?;
    Ok(token.claims)
}

pub fn parse_refresh_token(jwt: &str) -> Result<RefreshTokenClaims, ResponseError> {
    let token = decode::<RefreshTokenClaims>(
        &jwt,
        &DecodingKey::from_secret(GLOBAL_CONFIG.jwt.token_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| match e.kind() {
        &ErrorKind::ExpiredSignature => {
            ResponseError::new_expire_token_error("refresh_token已过期", Some("用户刷新凭证已过期"))
        }
        _ => e.into(),
    })?;
    Ok(token.claims)
}

pub fn generate_access_token(user_id: i32) -> Result<String, jsonwebtoken::errors::Error> {
    let token = encode(
        &Header::default(),
        &AccessTokenClaims::new(user_id),
        &EncodingKey::from_secret(GLOBAL_CONFIG.jwt.token_secret.as_bytes()),
    )?;
    Ok(token)
}

pub fn generate_refresh_token(
    user_id: i32,
    password: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let token = encode(
        &Header::default(),
        &RefreshTokenClaims::new(user_id, password),
        &EncodingKey::from_secret(GLOBAL_CONFIG.jwt.token_secret.as_bytes()),
    )?;
    Ok(token)
}
