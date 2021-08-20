use actix_web::{get, web, HttpResponse};
use deadpool_postgres::{Client, Pool};
use futures::future::try_join;
use serde_json::json;

use crate::db::Type as DBType;
use crate::error::ResponseError;
use crate::model::{
    game_mod::{
        GetModArticleCoverOutput, GetModArticleCoverOutputItem, GetModArticleCoverQuery,
        GetModArticleOutput, GetModArticlePath,
    },
    resource::ResourceSimple,
    tag::Tag,
};

// 获取mod文章的封面
#[get("/mod/covers")]
pub async fn get_mod_article_covers(
    db_pool: web::Data<Pool>,
    query: web::Query<GetModArticleCoverQuery>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let (s1, s2) = try_join(
        client.prepare_typed_cached(
            &format!(
            "WITH a AS (
                SELECT id, tag_ids, title, view, subscription, allowed_exp, vertical_image, game_article_id, game_article_title, updated_at FROM igame.mod_article WHERE id > $1 AND $2 <@ tag_ids LIMIT $3
            )
            SELECT a.id, a.title, a.view, a.subscription, a.allowed_exp, a.vertical_image, a.game_article_id, a.game_article_title, a.updated_at, array_agg(t.id) AS tag_ids, array_agg(t.value) AS tag_values
            FROM a
            INNER JOIN igame.tag AS t
            ON t.type = 2 AND t.id = ANY(a.tag_ids)
            GROUP BY a.id, a.title, a.view, a.subscription, a.allowed_exp, a.vertical_image, a.game_article_id, a.game_article_title, a.updated_at
            ORDER BY a.{}", 
            query.sort_by.to_string()),
            &[DBType::INT4, DBType::INT4_ARRAY, DBType::INT4]
        ),
        client.prepare_typed_cached(
            &format!(
            "WITH a AS (
                SELECT id, tag_ids, title, view, subscription, allowed_exp, vertical_image, game_article_id, game_article_title, updated_at FROM igame.mod_article WHERE game_article_id = $1 AND id > $2 AND $3 <@ tag_ids LIMIT $4
            )
            SELECT a.id, a.title, a.view, a.subscription, a.allowed_exp, a.vertical_image, a.game_article_id, a.game_article_title, a.updated_at, array_agg(t.id) AS tag_ids, array_agg(t.value) AS tag_values
            FROM a
            INNER JOIN igame.tag AS t
            ON t.type = 2 AND t.id = ANY(a.tag_ids)
            GROUP BY a.id, a.title, a.view, a.subscription, a.allowed_exp, a.vertical_image, a.game_article_id, a.game_article_title, a.updated_at
            ORDER BY a.{}", 
            query.sort_by.to_string()),
            &[DBType::INT4, DBType::INT4, DBType::INT4_ARRAY, DBType::INT4]
        ),
    ).await?;

    let r1s: Vec<tokio_postgres::Row>;
    if query.game_article_id.is_none() {
        r1s = client
            .query(&s1, &[&query.last_id, &query.tag_ids, &query.amount])
            .await?;
    } else {
        r1s = client
            .query(
                &s2,
                &[
                    &query.game_article_id.unwrap(),
                    &query.last_id,
                    &query.tag_ids,
                    &query.amount,
                ],
            )
            .await?;
    }

    let mut output: GetModArticleCoverOutput = Vec::new();
    for r1 in r1s {
        let tag_ids: Vec<i32> = r1.get("tag_ids");
        let mut tags: Vec<Tag> = Vec::new();
        if tag_ids.len() > 0 {
            let tag_values: Vec<&str> = r1.get("tag_values");
            for (index, tag_id) in tag_ids.iter().enumerate() {
                tags.push(Tag {
                    id: *tag_id,
                    value: tag_values[index].to_string(),
                });
            }
        }
        output.push(GetModArticleCoverOutputItem {
            id: r1.get("id"),
            tags: tags,
            title: r1.get("title"),
            view: r1.get("view"),
            subscription: r1.get("subscription"),
            allowed_exp: r1.get("allowed_exp"),
            vertical_image: r1.get("vertical_image"),
            game_article_id: r1.get("game_article_id"),
            game_article_title: r1.get("game_article_title"),
            updated_at: r1.get("updated_at"),
        })
    }

    Ok(HttpResponse::Ok().json(output))
}

// 获取Mod文章数量
#[get("/mod/article/size")]
pub async fn get_mod_article_size(db_pool: web::Data<Pool>) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let s1 = client
        .prepare_typed_cached("SELECT count(id) FROM igame.mod_article", &[])
        .await?;
    let r1 = client.query_one(&s1, &[]).await?;
    let count: i64 = r1.get("count");
    Ok(HttpResponse::Ok().json(json!({
        "size": count,
    })))
}

// 获取mod文章的内容
#[get("/mod/article/{id}")]
pub async fn get_mod_article(
    db_pool: web::Data<Pool>,
    path: web::Path<GetModArticlePath>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let s1 = client.prepare_typed_cached(
        "WITH t AS (
            SELECT a.id, array_agg(t.id) AS tag_ids, array_agg(t.value) AS tag_values, a.app_id, a.title, a.description, a.content, a.view, a.subscription, a.allowed_exp, a.horizontal_image, a.content_images, a.content_videos, a.game_article_id, a.game_article_title, a.updated_at
            FROM igame.mod_article AS a
            INNER JOIN igame.tag AS t
            ON a.id = $1 AND t.type = 2 AND t.id = ANY(a.tag_ids)
            GROUP BY a.id, a.app_id, a.title, a.description, a.content, a.view, a.subscription, a.allowed_exp, a.horizontal_image, a.content_images, a.content_videos, a.game_article_id, a.game_article_title, a.updated_at
        )
        SELECT t.*, array_agg(r.id) AS resource_ids, array_agg(r.name) AS resource_names
        FROM t
        LEFT JOIN common.resource AS r
        ON t.app_id = r.app_id 
        GROUP BY t.id, t.tag_ids, t.tag_values, t.app_id, t.title, t.description, t.content, t.view, t.subscription, t.allowed_exp, t.horizontal_image, t.content_images, t.content_videos, t.game_article_id, t.game_article_title, t.updated_at",
        &[DBType::INT4]
    ).await?;

    let r1 = client.query_one(&s1, &[&path.id]).await?;

    let mut tags: Vec<Tag> = Vec::new();
    let tag_ids: Vec<i32> = r1.get("tag_ids");
    if tag_ids.len() > 0 {
        let tag_values: Vec<&str> = r1.get("tag_values");
        for (index, tag_id) in tag_ids.iter().enumerate() {
            tags.push(Tag {
                id: *tag_id,
                value: tag_values[index].to_string(),
            });
        }
    }
    let mut resources: Vec<ResourceSimple> = Vec::new();
    let resource_ids: Vec<i32> = r1.get("resource_ids");
    if resource_ids.len() > 0 {
        let resource_names: Vec<&str> = r1.get("resource_names");
        for (index, resource_id) in resource_ids.iter().enumerate() {
            resources.push(ResourceSimple {
                id: *resource_id,
                name: resource_names[index].to_string(),
            });
        }
    }

    Ok(HttpResponse::Ok().json(GetModArticleOutput {
        id: r1.get("id"),
        tags: tags,
        app_id: r1.get("app_id"),
        resources: resources,
        title: r1.get("title"),
        description: r1.get("description"),
        content: r1.get("content"),
        view: r1.get("view"),
        subscription: r1.get("subscription"),
        allowed_exp: r1.get("allowed_exp"),
        horizontal_image: r1.get("horizontal_image"),
        content_images: r1.get("content_images"),
        content_videos: r1.get("content_videos"),
        game_article_id: r1.get("game_article_id"),
        game_article_title: r1.get("game_article_title"),
        updated_at: r1.get("updated_at"),
    }))
}
