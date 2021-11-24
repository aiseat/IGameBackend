use actix_web::{get, web, HttpResponse};
use deadpool_postgres::{Client, Pool};
use futures::future::try_join;

use crate::db::Type as DBType;
use crate::error::ResponseError;
use crate::model::app::{AppType, GetAppOutput, GetAppPath};

// 获取app的信息
#[get("/app/{app_id}")]
pub async fn get_app(
    db_pool: web::Data<Pool>,
    path: web::Path<GetAppPath>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;

    let (s1, s2) = try_join(
        client.prepare_typed_cached(
            "SELECT id, name, type, depend_id, created_at
            FROM igame.app
            WHERE id = $1",
            &[DBType::INT4],
        ),
        client.prepare_typed_cached(
            "SELECT id, name
            FROM igame.article
            WHERE app_id = $1",
            &[DBType::INT4],
        ),
    )
    .await?;
    let r1 = client.query_one(&s1, &[&path.app_id]).await?;
    let r2 = client.query_one(&s2, &[&path.app_id]).await?;

    Ok(HttpResponse::Ok().json(GetAppOutput {
        app_id: r1.get("id"),
        app_name: r1.get("name"),
        app_type: AppType::from_int2(r1.get("type")),
        depend_id: r1.get("depend_id"),
        article_id: r2.get("id"),
        article_name: r2.get("name"),
        created_at: r1.get("created_at"),
    }))
}
