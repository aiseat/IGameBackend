use actix_web::{get, post, web, HttpRequest, HttpResponse};
use deadpool_postgres::{Client, Pool};
use serde_json::json;

use crate::db::Type as DBType;
use crate::error::ResponseError;
use crate::model::notification::{
    GetNotificationOutput, GetNotificationOutputItem, SetNotificationInput,
};
use crate::model::permission::Permission;
use crate::util::req_parse::get_user_id;

// 获取通知
#[get("/notification")]
pub async fn get_notification(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;

    let s1 = client
        .prepare_typed_cached(
            "SELECT n.id, n.title, n.content, n.created_at
            FROM igame.user_notification AS un
            INNER JOIN igame.notification AS n
            ON un.notification_id = n.id
            WHERE un.user_id = $1 AND un.read = false",
            &[DBType::INT4],
        )
        .await?;
    let r1s = client.query(&s1, &[&user_id]).await?;

    let mut output: GetNotificationOutput = Vec::new();
    for r1 in r1s {
        output.push(GetNotificationOutputItem {
            id: r1.get("id"),
            title: r1.get("title"),
            content: r1.get("content"),
            created_at: r1.get("created_at"),
        })
    }

    Ok(HttpResponse::Ok().json(output))
}

// 创建通知
#[post("/notification")]
pub async fn post_notification(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    input: web::Json<SetNotificationInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;

    let (s1, s2, s3) = futures::future::try_join3(
        client.prepare_typed_cached(
            &format!(
                "SELECT bool_or({}) 
                FROM igame.role 
                WHERE id IN (
                    SELECT role_id 
                    FROM igame.user_role 
                    WHERE user_id = $1
                )",
                Permission::CreateNotification.to_string()
            ),
            &[DBType::INT4],
        ),
        client.prepare_typed_cached(
            "WITH n AS (
                INSERT INTO igame.notification(title, content, global) 
                VALUES($1, $2, true) 
                RETURNING id
            ),
            us AS (
                INSERT INTO igame.user_notification(user_id, notification_id)
                SELECT id, (SELECT id FROM n) FROM common.user
            )
            SELECT id FROM n",
            &[DBType::TEXT, DBType::TEXT],
        ),
        client.prepare_typed_cached(
            "WITH n AS (
                INSERT INTO igame.notification(title, content, global) 
                VALUES($1, $2, false) 
                RETURNING id
            ),
            us AS (
                INSERT INTO igame.user_notification(user_id, notification_id)
                SELECT unnest, (SELECT id FROM n) FROM unnest($3::int4[])
            )
            SELECT id FROM n",
            &[DBType::TEXT, DBType::TEXT, DBType::INT4_ARRAY],
        ),
    )
    .await?;

    let r1 = client.query_one(&s1, &[&user_id]).await?;
    let has_permission: bool = r1.get(0);
    if !has_permission {
        return Err(ResponseError::permission_err(
            "没有创建通知的权限",
            &format!(
                "没有{}权限, 用户ID: {}",
                Permission::CreateNotification.to_string(),
                user_id
            ),
        ));
    }

    let notification_id: i32;
    if input.global == true {
        let r2 = client
            .query_one(&s2, &[&input.title, &input.content])
            .await?;
        notification_id = r2.get("id");
    } else {
        let r3 = client
            .query_one(&s3, &[&input.title, &input.content, &input.user_ids])
            .await?;
        notification_id = r3.get("id");
    }

    Ok(HttpResponse::Ok().json(json!({
        "id": notification_id,
    })))
}
