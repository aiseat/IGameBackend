use actix_web::{get, web, HttpResponse};
use deadpool_postgres::{Client, Pool};

use crate::db::Type as DBType;
use crate::error::ResponseError;
use crate::model::tag::{GetTagOutput, GetTagsQuery, Tag};

// 获取tags
#[get("/tags")]
pub async fn get_tags(
    db_pool: web::Data<Pool>,
    query: web::Query<GetTagsQuery>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let s1 = client
        .prepare_typed_cached(
            "SELECT id, value FROM igame.tag WHERE type = $1",
            &[DBType::INT2],
        )
        .await?;

    let r1s = client.query(&s1, &[&query.r#type.to_int2()]).await?;

    let mut output: GetTagOutput = Vec::new();
    for r1 in r1s {
        output.push(Tag {
            id: r1.get("id"),
            value: r1.get("value"),
        });
    }

    Ok(HttpResponse::Ok().json(output))
}
