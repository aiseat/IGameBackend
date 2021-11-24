use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum AppType {
    #[serde(rename = "game")]
    Game,
    #[serde(rename = "mod")]
    Mod,
    #[serde(rename = "other")]
    Other,
}

impl AppType {
    pub fn to_int2(&self) -> i16 {
        match self {
            Self::Game => 1,
            Self::Mod => 2,
            Self::Other => 3,
        }
    }

    pub fn from_int2(v: i16) -> Self {
        match v {
            1 => Self::Game,
            2 => Self::Mod,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GetAppPath {
    pub app_id: i32,
}

#[derive(Debug, Serialize)]
pub struct GetAppOutput {
    pub app_id: i32,
    pub app_name: String,
    pub app_type: AppType,
    pub depend_id: Option<i32>,
    pub article_id: i32,
    pub article_name: String,
    pub created_at: DateTime<Utc>,
}
