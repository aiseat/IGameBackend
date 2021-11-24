use actix_web::{post, web, HttpRequest, HttpResponse};
use deadpool_postgres::{Client, Pool};
use futures::future::try_join;
use serde_json::json;

use crate::config::GLOBAL_CONFIG;
use crate::db::Type as DBType;
use crate::email::EMailPool;
use crate::error::ResponseError;
use crate::model::{
    email::{PostSendVerifyEmailOutput, SendEmailInput, SendVerifyEmailInput, VerifyEmailType},
    role::Permission,
};
use crate::util::{email, req_parse::get_user_id};

#[post("/send_verify_email")]
pub async fn post_send_verify_email(
    db_pool: web::Data<Pool>,
    email_pool: web::Data<EMailPool>,
    input: web::Json<SendVerifyEmailInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let email_addr = &input.email_addr;
    let email_type = &input.email_type;

    let (s1, s2) = try_join(
        client.prepare_typed_cached(
            "SELECT EXISTS(SELECT 1 FROM igame.user WHERE email = $1)",
            &[DBType::TEXT],
        ),
        client.prepare_typed_cached(
            "INSERT INTO igame.verify_email (type, addr, code) VALUES ($1, $2, $3) RETURNING id, created_at",
            &[DBType::INT2, DBType::TEXT, DBType::TEXT],
        ),
    )
    .await?;

    // 从用户表中检查邮箱存在与否
    let r1 = client.query_one(&s1, &[&email_addr]).await?;
    let exist: bool = r1.get(0);
    match email_type {
        VerifyEmailType::UserRegister => {
            if exist {
                return Err(ResponseError::input_err(
                    "邮箱地址已注册，请使用其他邮箱",
                    "UserRegister验证邮件发送失败，igame.user表内的email字段已存在",
                ));
            }
        }
        VerifyEmailType::PasswordReset => {
            if !exist {
                return Err(ResponseError::input_err(
                    "该邮箱不存在，请检查是否填写正确",
                    "PasswordReset验证邮件发送失败，igame.user表内的email字段不存在",
                ));
            }
        }
    }

    //发送验证邮件
    let verify_code = email::generate_verify_code();
    let subject = email_type.to_subject();
    let html = email_type.to_html(&verify_code);
    email::send_email(
        &email_pool,
        &GLOBAL_CONFIG.email.sender,
        email_addr,
        &subject,
        &html,
    )
    .await?;

    //添加verify_email记录
    let r2 = client
        .query_one(&s2, &[&email_type.to_int2(), email_addr, &verify_code])
        .await?;

    Ok(HttpResponse::Ok().json(PostSendVerifyEmailOutput {
        email_id: r2.get("id"),
        created_at: r2.get("created_at"),
    }))
}

#[post("/send_email")]
pub async fn post_send_email(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    email_pool: web::Data<EMailPool>,
    input: web::Json<SendEmailInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;

    let s1 = client
        .prepare_typed_cached(
            &format!(
                "SELECT bool_or({}) 
                FROM igame.role 
                WHERE id IN (
                    SELECT role_id 
                    FROM igame.user_role 
                    WHERE user_id = $1
                    AND (expire_at IS NULL OR (expire_at IS NOT NULL AND expire_at > now()))
                )",
                Permission::SendEmail.to_string()
            ),
            &[DBType::INT4],
        )
        .await?;

    // 检查是否有发送email的权限
    let r1 = client.query_one(&s1, &[&user_id]).await?;
    let has_permission: bool = r1.get(0);
    if !has_permission {
        return Err(ResponseError::permission_err(
            "发送email失败，没有对应权限",
            &format!("[用户ID: {}]没有send_email权限", user_id),
        ));
    }

    //发送验证邮件
    email::send_email(
        &email_pool,
        &GLOBAL_CONFIG.email.sender,
        &input.addr,
        &input.subject,
        &input.html,
    )
    .await?;

    Ok(HttpResponse::Ok().json(json!({ "result": "ok" })))
}
