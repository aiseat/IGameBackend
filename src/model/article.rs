use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ArticleSubscribeStatusQuery {
    pub id: i32,
    pub r#type: ArticleType,
}

#[derive(Debug, Deserialize)]
pub struct ArticleSubscribeInput {
    pub id: i32,
    pub r#type: ArticleType,
}

#[derive(Debug, Deserialize)]
pub struct ArticleUnsubscribeInput {
    pub id: i32,
    pub r#type: ArticleType,
}

#[derive(Debug, Deserialize)]
pub enum ArticleType {
    #[serde(rename = "game")]
    Game,
    #[serde(rename = "mod")]
    Mod,
}

impl ArticleType {
    pub fn to_int2(&self) -> i16 {
        match self {
            Self::Game => 1,
            Self::Mod => 2,
        }
    }
}
