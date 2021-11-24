use actix_web::{body::Body, get, post, web, HttpRequest, HttpResponse};
use deadpool_postgres::{Client, Pool};

use crate::db::Type as DBType;
use crate::error::{is_db_dup_unique_error, is_db_zero_line_error, ResponseError};
use crate::model::app_subscribe::{AppSubscribePath, GetAppSubscribeStatusOutput};
use crate::util::req_parse::get_user_id;

// 获取app订阅状态
#[get("/app/{app_id}/subscribe_status")]
pub async fn get_app_subscribe_status(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    path: web::Path<AppSubscribePath>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;
    let app_id = path.app_id;

    let s1 = client
        .prepare_typed_cached(
            "SELECT created_at FROM igame.user_app_sub WHERE user_id = $1 AND app_id = $2",
            &[DBType::INT4, DBType::INT4],
        )
        .await?;
    match client.query_one(&s1, &[&user_id, &app_id]).await {
        Ok(r1) => Ok(HttpResponse::Ok().json(GetAppSubscribeStatusOutput {
            subscribe: true,
            created_at: Some(r1.get("created_at")),
        })),
        Err(e) => {
            if is_db_zero_line_error(&e) {
                return Ok(HttpResponse::Ok().json(GetAppSubscribeStatusOutput {
                    subscribe: false,
                    created_at: None,
                }));
            }
            Err(ResponseError::from(e))
        }
    }
}

// 订阅app
#[post("/app/{app_id}/subscribe")]
pub async fn post_app_subscribe(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    path: web::Path<AppSubscribePath>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;
    let app_id = path.app_id;

    let s1 = client
        .prepare_typed_cached(
            "INSERT INTO igame.user_app_sub(user_id, app_id) VALUES($1, $2)",
            &[DBType::INT4, DBType::INT4],
        )
        .await?;
    let result = client.execute(&s1, &[&user_id, &app_id]).await;
    if let Err(e) = result {
        // 如果不是已订阅错误，那么返回该错误
        if !is_db_dup_unique_error(&e) {
            return Err(ResponseError::from(e));
        }
        // 如果是已订阅错误，那么无视
    }

    Ok(HttpResponse::Ok().body(Body::Empty))
}

// 取消订阅文章
#[post("/app/{app_id}/unsubscribe")]
pub async fn post_app_unsubscribe(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    path: web::Path<AppSubscribePath>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;
    let app_id = path.app_id;

    let s1 = client
        .prepare_typed_cached(
            "DELETE FROM igame.user_app_sub WHERE user_id = $1 AND app_id = $2",
            &[DBType::INT4, DBType::INT4],
        )
        .await?;
    client.execute(&s1, &[&user_id, &app_id]).await?;

    Ok(HttpResponse::Ok().body(Body::Empty))
}
