use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ResetPasswordInput {
    pub email: String,
    pub verify_code: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct UserLoginInput {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UserRegisterInput {
    pub email: String,
    pub password: String,
    pub nick_name: String,
    pub verify_code: String,
}

#[derive(Debug, Deserialize)]
pub struct NewTokenInput {
    pub refresh_token: String,
}
