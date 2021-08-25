use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};

use crate::model::{resource::ResourceSimple, tag::Tag};
use crate::util::serde_fn::option_str_to_vec;

#[derive(Debug, Deserialize)]
pub struct GetModArticleCoverQuery {
    pub last_id: i32,
    pub amount: i32,
    #[serde(default)]
    #[serde(deserialize_with = "option_str_to_vec")]
    pub tag_ids: Vec<i32>,
    pub game_article_id: Option<i32>,
    pub sort_by: ModArticleCoverSort,
}

#[derive(Debug, Deserialize, Display)]
pub enum ModArticleCoverSort {
    #[serde(rename = "id")]
    #[display(fmt = "id")]
    IdAsc,
    #[serde(rename = "id_desc")]
    #[display(fmt = "id DESC")]
    IdDesc,
    #[serde(rename = "updated_at")]
    #[display(fmt = "updated_at")]
    UpdatedAtAsc,
    #[serde(rename = "updated_at_desc")]
    #[display(fmt = "updated_at DESC")]
    UpdatedAtDesc,
    #[serde(rename = "view")]
    #[display(fmt = "view")]
    ViewAsc,
    #[serde(rename = "view_desc")]
    #[display(fmt = "view DESC")]
    ViewDesc,
    #[serde(rename = "downloaded")]
    #[display(fmt = "downloaded")]
    DownloadedASC,
    #[serde(rename = "downloaded_desc")]
    #[display(fmt = "downloaded DESC")]
    DownloadedDeSC,
    #[serde(rename = "subscription")]
    #[display(fmt = "subscription")]
    SubscriptionAsc,
    #[serde(rename = "subscription_desc")]
    #[display(fmt = "subscription DESC")]
    SubscriptionDesc,
}

#[derive(Debug, Deserialize)]
pub struct GetModArticlePath {
    pub id: i32,
}

pub type GetModArticleCoverOutput = Vec<GetModArticleCoverOutputItem>;

#[derive(Debug, Serialize)]
pub struct GetModArticleCoverOutputItem {
    pub id: i32,
    pub tags: Vec<Tag>,
    pub title: String,
    pub view: i32,
    pub subscription: i32,
    pub allowed_exp: i32,
    pub vertical_image: String,
    pub horizontal_image: String,
    pub game_article_id: i32,
    pub game_article_title: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct GetModArticleOutput {
    pub id: i32,
    pub tags: Vec<Tag>,
    pub app_id: i32,
    pub resources: Vec<ResourceSimple>,
    pub title: String,
    pub description: String,
    pub content: String,
    pub view: i32,
    pub subscription: i32,
    pub downloaded: i32,
    pub allowed_exp: i32,
    pub horizontal_image: String,
    pub content_images: Vec<String>,
    pub content_videos: Vec<String>,
    pub game_article_id: i32,
    pub game_article_title: String,
    pub updated_at: DateTime<Utc>,
}