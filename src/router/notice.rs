use actix_web::{get, post, web, HttpRequest, HttpResponse};
use deadpool_postgres::{Client, Pool};
use futures::future::{try_join, try_join3};
use serde_json::json;

use crate::db::Type as DBType;
use crate::error::ResponseError;
use crate::model::notice::{
    GetNoticeOutput, GetNoticePath, GetNoticesOutput, GetNoticesOutputItem, PostNoticeInput,
};
use crate::model::role::Permission;
use crate::util::req_parse::get_user_id;

// 获取全部通知
#[get("/notices")]
pub async fn get_notices(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;

    let s1 = client
        .prepare_typed_cached(
            "SELECT n.id, n.title, n.created_at, un.read
            FROM igame.user_notice AS un
            INNER JOIN igame.notice AS n
            ON un.notice_id = n.id
            WHERE un.user_id = $1",
            &[DBType::INT4],
        )
        .await?;
    let r1s = client.query(&s1, &[&user_id]).await?;

    let mut output: GetNoticesOutput = Vec::new();
    for r1 in r1s {
        output.push(GetNoticesOutputItem {
            notice_id: r1.get("id"),
            title: r1.get("title"),
            read: r1.get("read"),
            created_at: r1.get("created_at"),
        })
    }

    Ok(HttpResponse::Ok().json(output))
}

// 获取单个详细通知
#[get("/notice/{notice_id}")]
pub async fn get_notice(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    path: web::Path<GetNoticePath>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;
    let notice_id = path.notice_id;

    let (s1, s2) = try_join(
        client.prepare_typed_cached(
            "SELECT n.id, n.title, n.content, n.created_at, n.read, un.user_id
            FROM igame.user_notice AS un
            INNER JOIN igame.notice AS n
            ON un.notice_id = n.id
            WHERE n.id = $1",
            &[DBType::INT4],
        ),
        client.prepare_typed_cached(
            "UPDATE igame.notice SET read = true WHERE id = $1",
            &[DBType::INT4],
        ),
    )
    .await?;
    let r1 = client.query_one(&s1, &[&user_id]).await?;
    let notice_user_id: i32 = r1.get("user_id");

    if notice_user_id == user_id {
        // 设置为已读
        client.execute(&s2, &[&notice_id]).await?;
        // 返回结果
        Ok(HttpResponse::Ok().json(GetNoticeOutput {
            notice_id: r1.get("id"),
            title: r1.get("title"),
            content: r1.get("content"),
            read: r1.get("read"),
            created_at: r1.get("created_at"),
        }))
    } else {
        Err(ResponseError::permission_err(
            "获取通知失败，该通知不属于你",
            &format!("[用户ID: {}]尝试获取不属于自己的通知", user_id),
        ))
    }
}

// 创建通知
#[post("/notice")]
pub async fn post_notice(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    input: web::Json<PostNoticeInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;

    let (s1, s2, s3) = try_join3(
        // 检查是否有创建通知的权限
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
                Permission::CreateNotice.to_string()
            ),
            &[DBType::INT4],
        ),
        // 创建notice并分发给所有人
        client.prepare_typed_cached(
            "WITH n AS (
                INSERT INTO igame.notice(title, content, send_new_user) 
                VALUES($1, $2, $3) 
                RETURNING id
            ),
            us AS (
                INSERT INTO igame.user_notice(user_id, notice_id)
                SELECT id, (SELECT id FROM n) FROM igame.user
            )
            SELECT id FROM n",
            &[DBType::TEXT, DBType::TEXT, DBType::BOOL],
        ),
        // 创建notice并分发给特定人
        client.prepare_typed_cached(
            "WITH n AS (
                INSERT INTO igame.notice(title, content, send_new_user) 
                VALUES($1, $2, $3) 
                RETURNING id
            ),
            us AS (
                INSERT INTO igame.user_notice(user_id, notice_id)
                SELECT unnest, (SELECT id FROM n) FROM unnest($4::int4[])
            )
            SELECT id FROM n",
            &[DBType::TEXT, DBType::TEXT, DBType::BOOL, DBType::INT4_ARRAY],
        ),
    )
    .await?;

    let r1 = client.query_one(&s1, &[&user_id]).await?;
    let has_permission: bool = r1.get(0);
    if !has_permission {
        return Err(ResponseError::permission_err(
            "创建通知失败，没有对应权限",
            &format!("[用户ID: {}]没有create_notice权限", user_id),
        ));
    }

    let notice_id: i32;
    if let Some(user_ids) = &input.user_ids {
        let r3 = client
            .query_one(
                &s3,
                &[&input.title, &input.content, &input.send_new_user, user_ids],
            )
            .await?;
        notice_id = r3.get("id");
    } else {
        let r2 = client
            .query_one(&s2, &[&input.title, &input.content, &input.send_new_user])
            .await?;
        notice_id = r2.get("id");
    }

    Ok(HttpResponse::Ok().json(json!({
        "id": notice_id,
    })))
}
