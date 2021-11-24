use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};

use crate::model::app::AppType;
use crate::model::tag::Tag;
use crate::util::serde_fn::option_str_to_vec;

// 封面
#[derive(Debug, Deserialize)]
pub struct GetArticleCoverQuery {
    pub last_index: i32,
    pub limit: i32,
    pub app_type: AppType,
    pub depend_app_id: Option<i32>,
    pub sort_by: ArticleCoverSort,
    #[serde(default)]
    #[serde(deserialize_with = "option_str_to_vec")]
    pub tag_ids: Vec<i32>,
}

#[derive(Debug, Deserialize, Display)]
pub enum ArticleCoverSort {
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

#[derive(Debug, Serialize)]
pub struct GetArticleCoverOutputItem {
    pub article_id: i32,
    pub title: String,
    pub tags: Vec<Tag>,
    pub view: i32,
    pub downloaded: i32,
    pub subscription: i32,
    pub allowed_exp: i32,
    pub vertical_image: String,
    pub horizontal_image: String,
    pub updated_at: DateTime<Utc>,
}

pub type GetArticleCoverOutput = Vec<GetArticleCoverOutputItem>;

// 主体
#[derive(Debug, Deserialize)]
pub struct GetArticlePath {
    pub article_id: i32,
}

#[derive(Debug, Serialize)]
pub struct GetArticleOutput {
    pub article_id: i32,
    pub app_id: i32,
    pub title: String,
    pub description: String,
    pub content: String,
    pub tags: Vec<Tag>,
    pub view: i32,
    pub downloaded: i32,
    pub subscription: i32,
    pub allowed_exp: i32,
    pub vertical_image: String,
    pub horizontal_image: String,
    pub content_images: Vec<String>,
    pub content_video_thumbs: Vec<String>,
    pub content_videos: Vec<String>,
    pub updated_at: DateTime<Utc>,
    pub depend_id: Option<i32>,
}
