use actix_web::{get, web, HttpRequest, HttpResponse};
use deadpool_postgres::{Client, Pool};
use futures::future::{try_join, try_join4};
use serde_json::json;

use crate::db::Type as DBType;
use crate::error::ResponseError;
use crate::model::{
    article::{
        GetArticleCoverOutput, GetArticleCoverOutputItem, GetArticleCoverQuery, GetArticleOutput,
        GetArticlePath,
    },
    role::Permission,
    tag::Tag,
};
use crate::util::req_parse::{get_access_token, get_user_id};

// 获取文章的封面
#[get("/article/covers")]
pub async fn get_article_covers(
    db_pool: web::Data<Pool>,
    query: web::Query<GetArticleCoverQuery>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let r1s: Vec<tokio_postgres::Row>;
    if let Some(depend_app_id) = query.depend_app_id {
        let s1 = client.prepare_typed_cached(
            &format!(
                "WITH temp AS (
                    SELECT article.id, article.title, article.tag_ids, article.view, article.downloaded, article.subscription, article.allowed_exp, article.vertical_image, article.horizontal_image, article.updated_at
                    FROM igame.article AS article
                    INNER JOIN igame.app AS app
                    ON article.app_id = app.id
                    WHERE article.id > $1 
                    AND $2 <@ article.tag_ids 
                    AND app.type = $3 
                    AND app.depend_id = $4 
                    LIMIT $5
                )
                SELECT temp.*, array_agg(tag.value) AS tag_values
                FROM temp
                INNER JOIN igame.tag AS tag
                ON tag.id = ANY(temp.tag_ids)
                GROUP BY temp.id, temp.title, temp.tag_ids, temp.view, temp.downloaded, temp.subscription, temp.allowed_exp, temp.vertical_image, temp.horizontal_image, temp.updated_at
                ORDER BY temp.{}", 
                query.sort_by.to_string()),
                &[DBType::INT4, DBType::INT4_ARRAY, DBType::INT2, DBType::INT4, DBType::INT4]
            ).await?;

        r1s = client
            .query(
                &s1,
                &[
                    &query.last_index,
                    &query.tag_ids,
                    &query.app_type.to_int2(),
                    &depend_app_id,
                    &query.limit,
                ],
            )
            .await?;
    } else {
        let s1 = client.prepare_typed_cached(
            &format!(
                "WITH temp AS (
                    SELECT article.id, article.title, article.tag_ids, article.view, article.downloaded, article.subscription, article.allowed_exp, article.vertical_image, article.horizontal_image, article.updated_at
                    FROM igame.article AS article
                    INNER JOIN igame.app AS app
                    ON article.app_id = app.id
                    WHERE article.id > $1 
                    AND $2 <@ article.tag_ids 
                    AND app.type = $3
                    LIMIT $4
                )
                SELECT temp.*, array_agg(tag.value) AS tag_values
                FROM temp
                INNER JOIN igame.tag AS tag
                ON tag.id = ANY(temp.tag_ids)
                GROUP BY temp.id, temp.title, temp.tag_ids, temp.view, temp.downloaded, temp.subscription, temp.allowed_exp, temp.vertical_image, temp.horizontal_image, temp.updated_at
                ORDER BY temp.{}", 
                query.sort_by.to_string()),
                &[DBType::INT4, DBType::INT4_ARRAY, DBType::INT2, DBType::INT4]
            ).await?;

        r1s = client
            .query(
                &s1,
                &[
                    &query.last_index,
                    &query.tag_ids,
                    &query.app_type.to_int2(),
                    &query.limit,
                ],
            )
            .await?;
    }

    let mut output: GetArticleCoverOutput = Vec::new();
    for r1 in r1s {
        let tag_ids: Vec<i32> = r1.get("tag_ids");
        let mut tags: Vec<Tag> = Vec::new();
        if tag_ids.len() > 0 {
            let tag_values: Vec<&str> = r1.get("tag_values");
            for (index, tag_id) in tag_ids.iter().enumerate() {
                tags.push(Tag {
                    tag_id: *tag_id,
                    value: tag_values[index].to_string(),
                });
            }
        }

        output.push(GetArticleCoverOutputItem {
            article_id: r1.get("id"),
            title: r1.get("title"),
            tags: tags,
            view: r1.get("view"),
            downloaded: r1.get("downloaded"),
            subscription: r1.get("subscription"),
            allowed_exp: r1.get("allowed_exp"),
            vertical_image: r1.get("vertical_image"),
            horizontal_image: r1.get("horizontal_image"),
            updated_at: r1.get("updated_at"),
        })
    }

    Ok(HttpResponse::Ok().json(output))
}

// 获取文章总数量
#[get("/article/amount")]
pub async fn get_article_amount(db_pool: web::Data<Pool>) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let s1 = client
        .prepare_typed_cached("SELECT count(id) FROM igame.article", &[])
        .await?;
    let r1 = client.query_one(&s1, &[]).await?;
    let count: i64 = r1.get("count");
    Ok(HttpResponse::Ok().json(json!({
        "amount": count,
    })))
}

// 获取文章的内容
#[get("/article/{article_id}")]
pub async fn get_article(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    path: web::Path<GetArticlePath>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let (s1, s2, s3, s4) = try_join4(
        // 返回完整的文章信息
        client.prepare_typed_cached(
            "WITH temp AS (
                SELECT article.id, article.app_id, article.title, article.description, article.content, article.tag_ids, article.view, article.downloaded, article.subscription, article.allowed_exp, article.vertical_image, article.horizontal_image, article.content_images, article.content_video_thumbs, article.content_videos, article.updated_at, app.depend_id
                FROM igame.article AS article
                INNER JOIN igame.app AS app
                ON article.app_id = app.id
                WHERE article.id = $1
            )
            SELECT temp.*, array_agg(tag.value) AS tag_values
            FROM temp
            INNER JOIN igame.tag AS tag
            ON tag.id = ANY(temp.tag_ids)
            GROUP BY temp.id, temp.app_id, temp.title, temp.description, temp.content, temp.tag_ids, temp.view, temp.downloaded, temp.subscription, temp.allowed_exp, temp.vertical_image, temp.horizontal_image, temp.content_images, temp.content_video_thumbs, temp.content_videos, temp.updated_at, temp.depend_id",
            &[DBType::INT4]
        ),
        // 浏览量+1
        client.prepare_typed_cached(
            "UPDATE igame.article SET view = view + 1 WHERE id = $1",
            &[DBType::INT4]
        ),
        // 获取当前用户的等级
        client.prepare_typed_cached(
            "SELECT exp FROM igame.user WHERE id = $1",
            &[DBType::INT4],
        ),
        // 检验相关权限
        client.prepare_typed_cached(
            &format!(
                "SELECT bool_or({})
                FROM igame.role 
                WHERE id IN (
                    SELECT role_id 
                    FROM igame.user_role 
                    WHERE user_id = $1
                    AND (expire_at IS NULL OR (expire_at IS NOT NULL AND expire_at > now()))
                )",
                Permission::IgnoreExp.to_string()
            ),
            &[DBType::INT4],
        ),
    ).await?;

    let r1 = client.query_one(&s1, &[&path.article_id]).await?;
    let article_id: i32 = r1.get("id");
    let allowed_exp: i32 = r1.get("allowed_exp");
    let is_login = get_access_token(&req).is_some();
    let mut can_view = true;
    // 如果该文章的allowed_exp大于0，那么检验用户的exp
    if allowed_exp > 0 {
        if !is_login {
            can_view = false;
        } else {
            let user_id = get_user_id(&req)?;
            let (r3, r4) = try_join(
                client.query_one(&s3, &[&user_id]),
                client.query_one(&s4, &[&user_id]),
            )
            .await?;
            let exp: i32 = r3.get("exp");
            let can_ignore_exp: bool = r4.get("ignore_exp");
            // 如果用户没有ignore_exp权限，且exp小于文章的allowed_exp
            if !can_ignore_exp && exp < allowed_exp {
                can_view = false;
            }
        }
    }
    // 如果不能浏览，返回错误
    if can_view == false {
        return Err(ResponseError::lack_exp_err(
            "用户等级不足，无法浏览本文章",
            allowed_exp,
            &format!("用户ID：游客，文章ID:{}", article_id),
        ));
    }
    // 浏览量+1
    client.execute(&s2, &[&path.article_id]).await?;
    // 将tag_ids与tag_values转换成Vec<Tag>类型
    let mut tags: Vec<Tag> = Vec::new();
    let tag_ids: Vec<i32> = r1.get("tag_ids");
    if tag_ids.len() > 0 {
        let tag_values: Vec<&str> = r1.get("tag_values");
        for (index, tag_id) in tag_ids.iter().enumerate() {
            tags.push(Tag {
                tag_id: *tag_id,
                value: tag_values[index].to_string(),
            });
        }
    }

    Ok(HttpResponse::Ok().json(GetArticleOutput {
        article_id: r1.get("id"),
        app_id: r1.get("app_id"),
        title: r1.get("title"),
        description: r1.get("description"),
        content: r1.get("content"),
        tags: tags,
        view: r1.get("view"),
        downloaded: r1.get("downloaded"),
        subscription: r1.get("subscription"),
        allowed_exp: r1.get("allowed_exp"),
        vertical_image: r1.get("vertical_image"),
        horizontal_image: r1.get("horizontal_image"),
        content_images: r1.get("content_images"),
        content_video_thumbs: r1.get("content_video_thumbs"),
        content_videos: r1.get("content_videos"),
        updated_at: r1.get("updated_at"),
        depend_id: r1.get("depend_id"),
    }))
}
