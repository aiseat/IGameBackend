use actix_web::{body::Body, get, post, web, HttpRequest, HttpResponse};
use deadpool_postgres::{Client, Pool};
use serde_json::json;

use crate::db::Type as DBType;
use crate::error::{is_db_dup_unique_error, ResponseError};
use crate::model::article::{
    ArticleSubscribeInput, ArticleSubscribeStatusQuery, ArticleUnsubscribeInput,
};
use crate::util::req_parse::get_user_id;

// 获取文章订阅状态
#[get("/article/subscribe_status")]
pub async fn get_article_subscribe_stataus(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    query: web::Query<ArticleSubscribeStatusQuery>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;
    let article_id = query.id;
    let article_type = &query.r#type;

    let s1 = client
        .prepare_typed_cached(
            "SELECT EXISTS(SELECT 1 FROM igame.user_article_sub WHERE user_id = $1 AND article_id = $2 AND article_type = $3)",
            &[DBType::INT4, DBType::INT4, DBType::INT2],
        )
        .await?;
    let r1 = client
        .query_one(&s1, &[&user_id, &article_id, &article_type.to_int2()])
        .await?;
    let exist: bool = r1.get(0);

    Ok(HttpResponse::Ok().json(json!({ "subscribe": exist })))
}

// 订阅文章
#[post("/article/subscribe")]
pub async fn post_article_subscribe(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    input: web::Json<ArticleSubscribeInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;
    let article_id = input.id;
    let article_type = &input.r#type;

    let s1 = client
        .prepare_typed_cached("INSERT INTO igame.user_article_sub(user_id, article_id, article_type) VALUES($1, $2, $3)", &[DBType::INT4, DBType::INT4, DBType::INT2])
        .await?;
    let result = client
        .execute(&s1, &[&user_id, &article_id, &article_type.to_int2()])
        .await;
    if let Err(e) = result {
        if !is_db_dup_unique_error(&e) {
            return Err(ResponseError::from(e));
        }
    }

    Ok(HttpResponse::Ok().body(Body::Empty))
}

// 取消订阅文章
#[post("/article/unsubscribe")]
pub async fn post_article_unsubscribe(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    input: web::Json<ArticleUnsubscribeInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;
    let article_id = input.id;
    let article_type = &input.r#type;

    let s1 = client
        .prepare_typed_cached("DELETE FROM igame.user_article_sub WHERE user_id = $1 AND article_id = $2 AND article_type = $3", &[DBType::INT4, DBType::INT4, DBType::INT2])
        .await?;
    client
        .execute(&s1, &[&user_id, &article_id, &article_type.to_int2()])
        .await?;

    Ok(HttpResponse::Ok().body(Body::Empty))
}
