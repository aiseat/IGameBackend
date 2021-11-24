use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AppSubscribePath {
    pub app_id: i32,
}

#[derive(Debug, Serialize)]
pub struct GetAppSubscribeStatusOutput {
    pub subscribe: bool,
    pub created_at: Option<DateTime<Utc>>,
}
