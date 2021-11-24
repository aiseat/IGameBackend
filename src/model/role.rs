use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Display)]
pub enum Permission {
    #[display(fmt = "get_user")]
    GetUser,
    #[display(fmt = "create_user")]
    CreateUser,
    #[display(fmt = "send_email")]
    SendEmail,
    #[display(fmt = "create_notice")]
    CreateNotice,
    #[display(fmt = "free_download")]
    FreeDownload,
    #[display(fmt = "free_install")]
    FreeInstall,
    #[display(fmt = "ignore_exp")]
    IgnoreExp,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum RoleID {
    #[serde(rename = "admin")]
    Admin = 1,
    #[serde(rename = "user")]
    User = 2,
    #[serde(rename = "vip")]
    Vip = 3,
}

impl RoleID {
    pub fn to_i32(&self) -> i32 {
        *self as i32
    }
}

#[derive(Debug, Serialize)]
pub struct Role {
    pub role_id: i32,
    pub name: String,
    pub expire_at: Option<DateTime<Utc>>,
}
