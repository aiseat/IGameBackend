use actix_web::{get, web, HttpRequest, HttpResponse};
use deadpool_postgres::{Client, Pool};
use futures::future::{try_join, try_join4};
use serde_json::json;

use crate::db::Type as DBType;
use crate::error::ResponseError;
use crate::model::{
    game_article::{
        GetGameArticleCoverOutput, GetGameArticleCoverOutputItem, GetGameArticleCoverQuery,
        GetGameArticleOutput, GetGameArticlePath,
    },
    permission::Permission,
    resource::ResourceSimple,
    tag::Tag,
};
use crate::util::req_parse::{get_access_token, get_user_id};

// 获取游戏文章的封面
#[get("/game/covers")]
pub async fn get_game_article_covers(
    db_pool: web::Data<Pool>,
    query: web::Query<GetGameArticleCoverQuery>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let s1 = client.prepare_typed_cached(
        &format!(
            "WITH a AS (
                SELECT id, tag_ids, title, view, subscription, allowed_exp, vertical_image, horizontal_image, updated_at FROM igame.game_article WHERE id > $1 AND $2 <@ tag_ids LIMIT $3
            )
            SELECT a.id, a.title, a.view, a.subscription, a.allowed_exp, a.vertical_image, a.horizontal_image, a.updated_at, array_agg(t.id) AS tag_ids, array_agg(t.value) AS tag_values
            FROM  a
            INNER JOIN igame.tag AS t
            ON t.type = 1 AND t.id = ANY(a.tag_ids)
            GROUP BY a.id, a.title, a.view, a.subscription, a.allowed_exp, a.vertical_image, a.horizontal_image, a.updated_at
            ORDER BY a.{}", 
            query.sort_by.to_string()),
            &[DBType::INT4, DBType::INT4_ARRAY, DBType::INT4]
        ).await?;

    let r1s = client
        .query(&s1, &[&query.last_id, &query.tag_ids, &query.amount])
        .await?;

    let mut output: GetGameArticleCoverOutput = Vec::new();
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

        output.push(GetGameArticleCoverOutputItem {
            id: r1.get("id"),
            tags: tags,
            title: r1.get("title"),
            view: r1.get("view"),
            subscription: r1.get("subscription"),
            allowed_exp: r1.get("allowed_exp"),
            vertical_image: r1.get("vertical_image"),
            horizontal_image: r1.get("horizontal_image"),
            updated_at: r1.get("updated_at"),
        })
    }

    Ok(HttpResponse::Ok().json(output))
}

// 获取游戏文章数量
#[get("/game/article/size")]
pub async fn get_game_article_size(
    db_pool: web::Data<Pool>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let s1 = client
        .prepare_typed_cached("SELECT count(id) FROM igame.game_article", &[])
        .await?;
    let r1 = client.query_one(&s1, &[]).await?;
    let count: i64 = r1.get("count");
    Ok(HttpResponse::Ok().json(json!({
        "size": count,
    })))
}

// 获取游戏文章的内容
#[get("/game/article/{id}")]
pub async fn get_game_article(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    path: web::Path<GetGameArticlePath>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let (s1, s2, s3, s4) = try_join4(
        client.prepare_typed_cached(
            "WITH t AS (
                SELECT a.id, array_agg(t.id) AS tag_ids, array_agg(t.value) AS tag_values, a.app_id, a.title, a.description, a.content, a.subscription, a.allowed_exp, a.horizontal_image, a.content_images, a.content_videos, a.content_video_thumbs, a.updated_at
                FROM igame.game_article AS a
                INNER JOIN igame.tag AS t
                ON a.id = $1 AND t.type = 1 AND t.id = ANY(a.tag_ids)
                GROUP BY a.id, a.app_id, a.title, a.description, a.content, a.subscription, a.allowed_exp, a.horizontal_image, a.content_images, a.content_videos, a.content_video_thumbs, a.updated_at
            )
            SELECT t.*, array_agg(r.id) AS resource_ids, array_agg(r.name) AS resource_names, array_agg(r.downloaded) AS resource_downloadeds
            FROM t
            LEFT JOIN common.resource AS r
            ON t.app_id = r.app_id 
            GROUP BY t.id, t.tag_ids, t.tag_values, t.app_id, t.title, t.description, t.content, t.subscription, t.allowed_exp, t.horizontal_image, t.content_images, t.content_videos, t.content_video_thumbs, t.updated_at",
            &[DBType::INT4]
        ),
        client.prepare_typed_cached(
            "UPDATE igame.game_article SET view = view + 1 WHERE id = $1 RETURNING view",
            &[DBType::INT4]
        ),
        client.prepare_typed_cached(
            "SELECT exp FROM common.user WHERE id = $1",
            &[DBType::INT4],
        ),
        client.prepare_typed_cached(
            &format!(
                "SELECT bool_or({}) as ignore_exp
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

    let r1 = client.query_one(&s1, &[&path.id]).await?;

    let article_id: i32 = r1.get("id");
    let allowed_exp: i32 = r1.get("allowed_exp");
    let is_login = get_access_token(&req).is_some();

    if allowed_exp > 0 {
        if !is_login {
            return Err(ResponseError::lack_exp_err(
                "无法浏览本文章",
                allowed_exp,
                &format!("用户ID：游客，文章ID:{}, 文章类型: game", article_id),
            ));
        } else {
            let user_id = get_user_id(&req)?;
            let (r3, r4) = try_join(
                client.query_one(&s3, &[&user_id]),
                client.query_one(&s4, &[&user_id]),
            )
            .await?;
            let exp: i32 = r3.get("exp");
            let can_ignore_exp: bool = r4.get("ignore_exp");
            if !can_ignore_exp && exp < allowed_exp {
                return Err(ResponseError::lack_exp_err(
                    "无法浏览本文章",
                    allowed_exp,
                    &format!("用户ID：{}，文章ID:{}, 文章类型: game", user_id, article_id),
                ));
            }
        }
    }

    let r2 = client.query_one(&s2, &[&path.id]).await?;

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
    let mut downloaded: i32 = 0;
    let mut resources: Vec<ResourceSimple> = Vec::new();
    let resource_ids: Option<Vec<i32>> = r1.get("resource_ids");
    if let Some(ids) = resource_ids {
        let names: Vec<&str> = r1.get("resource_names");
        let downloadeds: Vec<i32> = r1.get("resource_downloadeds");
        for (index, id) in ids.iter().enumerate() {
            downloaded += downloadeds[index];
            resources.push(ResourceSimple {
                id: *id,
                name: names[index].to_string(),
                downloaded: downloadeds[index],
            });
        }
    }

    Ok(HttpResponse::Ok().json(GetGameArticleOutput {
        id: r1.get("id"),
        tags: tags,
        app_id: r1.get("app_id"),
        resources: resources,
        title: r1.get("title"),
        description: r1.get("description"),
        content: r1.get("content"),
        view: r2.get("view"),
        subscription: r1.get("subscription"),
        downloaded: downloaded,
        allowed_exp: allowed_exp,
        horizontal_image: r1.get("horizontal_image"),
        content_images: r1.get("content_images"),
        content_videos: r1.get("content_videos"),
        content_video_thumbs: r1.get("content_video_thumbs"),
        updated_at: r1.get("updated_at"),
    }))
}
