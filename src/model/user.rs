use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::role::{Role, RoleID};

#[derive(Debug, Serialize)]
pub struct User {
    pub user_id: i32,
    pub email: String,
    pub nick_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub exp: i32,
    pub coin: i32,
    pub roles: Vec<Role>,
    pub avatar_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct PostUserInput {
    pub email: String,
    pub nick_name: String,
    pub password: String,
    pub role: RoleID,
}

#[derive(Debug, Serialize)]
pub struct PostUserOutput {
    pub user_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct PostUserResetPasswordInput {
    pub email: String,
    pub verify_code: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct PostUserResetPasswordOutput {
    pub user_id: i32,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct PostUserLoginInput {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct PostUserLoginOutput {
    pub user_id: i32,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct PostUserRegisterInput {
    pub email: String,
    pub password: String,
    pub nick_name: String,
    pub verify_code: String,
}

#[derive(Debug, Serialize)]
pub struct PostUserRegisterOutput {
    pub user_id: i32,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct PostNewTokenInput {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct PostNewTokenOutput {
    pub user_id: i32,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct GetUserPath {
    pub user_id: i32,
}

#[derive(Debug, Serialize)]
pub struct GetUserOutput {
    pub user_id: i32,
    pub email: String,
    pub nick_name: String,
    pub exp: i32,
    pub coin: i32,
    pub roles: Vec<Role>,
    pub avatar_url: String,
    pub login_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct GetMyselfOutput {
    pub user_id: i32,
    pub email: String,
    pub nick_name: String,
    pub exp: i32,
    pub coin: i32,
    pub roles: Vec<Role>,
    pub avatar_url: String,
    pub login_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PostUserDailyBonusOutput {
    pub daily_bonus_id: i32,
    pub count: i32,
    pub added_coin: i32,
    pub added_exp: i32,
    pub total_coin: i32,
    pub total_exp: i32,
}
