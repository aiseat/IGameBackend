use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type GetNotificationOutput = Vec<GetNotificationOutputItem>;

#[derive(Debug, Serialize)]
pub struct GetNotificationOutputItem {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SetNotificationInput {
    pub title: String,
    pub content: String,
    pub global: bool,
    pub user_ids: Vec<i32>,
}
