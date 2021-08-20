use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::permission::Role;

#[derive(Debug, Serialize)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub nick_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_at: Option<DateTime<Utc>>,
    pub avatar_url: String,
    pub exp: i32,
    pub coin: i32,
}

#[derive(Debug, Deserialize)]
pub struct GetUserPath {
    pub id: i32,
}

#[derive(Debug, Deserialize)]
pub struct UserCreateInput {
    pub email: String,
    pub nick_name: String,
    pub password: String,
    pub role: Role,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: 0,
            email: String::new(),
            nick_name: String::new(),
            password: None,
            created_at: None,
            login_at: None,
            avatar_url: String::new(),
            exp: 0,
            coin: 0,
        }
    }
}
