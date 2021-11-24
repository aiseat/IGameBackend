use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct GetNoticesOutputItem {
    pub notice_id: i32,
    pub title: String,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

pub type GetNoticesOutput = Vec<GetNoticesOutputItem>;

#[derive(Debug, Deserialize)]
pub struct GetNoticePath {
    pub notice_id: i32,
}

#[derive(Debug, Serialize)]
pub struct GetNoticeOutput {
    pub notice_id: i32,
    pub title: String,
    pub content: String,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PostNoticeInput {
    pub title: String,
    pub content: String,
    pub user_ids: Option<Vec<i32>>,
    pub send_new_user: bool,
}
