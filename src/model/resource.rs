use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::ProviderGroup;

// 简略资源信息
#[derive(Debug, Deserialize)]
pub struct GetBriefResourcesPath {
    pub app_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct GetBriefResourcesQuery {
    pub last_index: i32,
    pub limit: i32,
}

#[derive(Debug, Serialize)]
pub struct GetBriefResourcesOutputItem {
    pub resource_id: i32,
    pub name: String,
    pub version: String,
    pub allowed_exp: i32,
}

pub type GetBriefResourcesOutput = Vec<GetBriefResourcesOutputItem>;

// 完整资源信息
#[derive(Debug, Deserialize)]
pub struct GetResourcePath {
    pub resource_id: i32,
}

#[derive(Debug, Serialize)]
pub struct GetResourceOutput {
    pub resource_id: i32,
    pub app_id: i32,
    pub name: String,
    pub description: String,
    pub version: String,
    pub allowed_exp: i32,
    pub downloaded: i32,
    pub normal_download_cost: i32,
    pub fast_download_cost: i32,
    pub install_cost: i32,
    pub can_normal_download: bool,
    pub can_fast_download: bool,
    pub updated_at: DateTime<Utc>,
}

// 资源URL
#[derive(Debug, Deserialize)]
pub struct GetResourceUrlPath {
    pub resource_id: i32,
    pub provider_group: ProviderGroup,
}

#[derive(Debug, Serialize)]
pub struct GetResourceUrlOutput {
    pub download_url: String,
    pub trade_id: Option<i32>,
    pub remain_coin: Option<i32>,
    pub downloaded: i32,
}
