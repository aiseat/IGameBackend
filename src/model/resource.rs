use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};

use crate::resource_provider::ClientGroup;

#[derive(Debug, Serialize)]
pub struct ResourceSimple {
    pub id: i32,
    pub name: String,
    pub version: Vec<i32>,
    pub allowed_exp: i32,
    pub downloaded: i32,
}

#[derive(Debug, Deserialize)]
pub struct GetResourcePath {
    pub id: i32,
}

#[derive(Debug, Serialize)]
pub struct GetResourceOutput {
    pub id: i32,
    pub app_id: i32,
    pub name: String,
    pub version: Vec<i32>,
    pub allowed_exp: i32,
    pub downloaded: i32,
    pub full_costs: Vec<i32>,
    pub update_costs: Vec<i32>,
    pub supported_systems: Vec<i16>,
    pub change_log: String,
    pub updated_at: DateTime<Utc>,
    pub depends: Vec<DependResource>,
}

#[derive(Debug, Serialize)]
pub struct DependResource {
    pub id: i32,
    pub name: String,
    pub app_type: i16,
    pub article_id: i32,
}

#[derive(Debug, Serialize)]
pub struct DependResourceWithoutArticleId {
    pub id: i32,
    pub name: String,
    pub app_type: i16,
}

#[derive(Debug, Deserialize)]
pub struct GetResourceUrlPath {
    pub id: i32,
}

#[derive(Debug, Deserialize)]
pub struct GetResourceUrlQuery {
    pub r#type: TradeResourceType,
    pub group: ClientGroup,
}

#[derive(Debug, Deserialize, Display)]
pub enum TradeResourceType {
    #[serde(rename = "full")]
    #[display(fmt = "full")]
    Full,
    #[serde(rename = "update")]
    #[display(fmt = "update")]
    Update,
}

impl TradeResourceType {
    pub fn to_index(&self) -> usize {
        match self {
            Self::Full => 0,
            Self::Update => 1,
        }
    }
}
