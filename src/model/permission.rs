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
    #[display(fmt = "create_notification")]
    CreateNotification,
    #[display(fmt = "free_download")]
    FreeDownload,
    #[display(fmt = "ignore_exp")]
    IgnoreExp,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum RoleID {
    #[serde(rename = "admin")]
    Admin = 1,
    #[serde(rename = "user")]
    User,
}

impl RoleID {
    pub fn to_i32(&self) -> i32 {
        *self as i32
    }
}
