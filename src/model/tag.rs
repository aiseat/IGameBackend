use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Tag {
    pub id: i32,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct GetTagsQuery {
    pub r#type: TagType,
}

#[derive(Debug, Deserialize)]
pub enum TagType {
    #[serde(rename = "game")]
    Game,
    #[serde(rename = "mod")]
    Mod,
}

impl TagType {
    pub fn to_int2(&self) -> i16 {
        match self {
            Self::Game => 1,
            Self::Mod => 2,
        }
    }
}

pub type GetTagOutput = Vec<Tag>;
