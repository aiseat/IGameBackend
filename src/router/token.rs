use actix_web::{post, web, HttpResponse};
use deadpool_postgres::{Client, Pool};
use serde_json::json;
use std::time::{Duration, SystemTime};

use crate::db::Type as DBType;
use crate::error::ResponseError;
use crate::model::token::{NewTokenInput, ResetPasswordInput, UserLoginInput, UserRegisterInput};
use crate::model::{email::EmailType, permission::Role};
use crate::util::{hash, is_db_zero_line_error, jwt};

#[post("/login")]
pub async fn post_login(
    db_pool: web::Data<Pool>,
    user_login_input: web::Json<UserLoginInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;

    //准备语句
    let (s1, s2) = futures::future::try_join(
        client.prepare_typed_cached(
            "SELECT id, password FROM common.user WHERE email = $1",
            &[DBType::TEXT],
        ),
        client.prepare_typed_cached(
            "UPDATE common.user SET login_at = $1 WHERE id = $2",
            &[DBType::TIMESTAMPTZ, DBType::INT4],
        ),
    )
    .await?;

    let r1 = client
        .query_one(&s1, &[&user_login_input.email])
        .await
        .map_err(|e| match is_db_zero_line_error(&e) {
            true => ResponseError::input_err("邮箱或密码不正确，请重新输入", "错误的邮箱地址"),
            false => ResponseError::from(e),
        })?;
    let user_id: i32 = r1.get("id");
    let password: Vec<u8> = r1.get("password");
    let same = hash::compare_password(&user_login_input.password, &password)?;
    if !same {
        return Err(ResponseError::input_err(
            "邮箱或密码不正确，请重新输入",
            "错误的密码",
        ));
    }

    client
        .execute(&s2, &[&chrono::Utc::now(), &user_id])
        .await?;

    let access_token = jwt::generate_access_token(user_id)?;
    let password_hex = hex::encode(password);
    let refresh_token = jwt::generate_refresh_token(user_id, &password_hex)?;
    Ok(HttpResponse::Ok().json(json!({
        "user_id": user_id,
        "access_token": access_token,
        "refresh_token": refresh_token
    })))
}

#[post("/register")]
pub async fn post_register(
    db_pool: web::Data<Pool>,
    user_register_input: web::Json<UserRegisterInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;

    //准备语句
    let (s1, s2, s3, s4) = futures::future::try_join4(
        client.prepare_typed_cached(
            "SELECT id, used, code, created_at FROM common.verification_email WHERE type = $1 AND addr = $2 ORDER BY created_at DESC LIMIT 1",
            &[DBType::TEXT, DBType::TEXT]),
        client.prepare_typed_cached(
            "SELECT EXISTS(SELECT 1 FROM common.user WHERE email = $1)",
            &[DBType::TEXT]),
        client.prepare_typed_cached(
            "WITH u AS (INSERT INTO common.user(email, nick_name, password) VALUES($1, $2, $3) RETURNING id)
            INSERT INTO igame.user_role(user_id, role_id) SELECT id, $4 FROM u RETURNING user_id",
            &[DBType::TEXT, DBType::TEXT, DBType::BYTEA, DBType::INT4]),
        client.prepare_typed_cached(
            "UPDATE common.verification_email SET used = TRUE WHERE id = $1",
            &[DBType::INT4])
    ).await?;

    //检查verify_code是否合法
    let r1 = client
        .query_one(
            &s1,
            &[
                &EmailType::UserRegister.to_string(),
                &user_register_input.email,
            ],
        )
        .await
        .map_err(|e| match is_db_zero_line_error(&e) {
            true => ResponseError::input_err(
                "无法验证邮箱，请尝试重新发送邮件",
                "该邮箱没有任何验证记录",
            ),
            false => ResponseError::from(e),
        })?;
    let email_id: i32 = r1.get("id");
    let used: bool = r1.get("used");
    let code: &str = r1.get("code");
    let created_at: SystemTime = r1.get("created_at");
    if used == true {
        return Err(ResponseError::input_err(
            "无法验证邮箱，请尝试重新发送邮件",
            "该验证记录已被使用",
        ));
    }
    if code != user_register_input.verify_code {
        return Err(ResponseError::input_err(
            "无法验证邮箱，请检查验证码是否正确",
            "验证码不匹配",
        ));
    }
    if created_at < SystemTime::now() - Duration::from_secs(2 * 60 * 60) {
        return Err(ResponseError::input_err(
            "无法验证邮箱，请尝试重新发送邮件",
            "验证记录已过期",
        ));
    }

    //检查邮箱是否存在
    let r2 = client.query_one(&s2, &[&user_register_input.email]).await?;
    let exist = r2.get(0);
    if exist {
        return Err(ResponseError::input_err(
            "邮箱地址已注册，请使用其他邮箱",
            "邮箱地址早已存在",
        ));
    }

    //创建新用户
    let hased_password = hash::hash_password(&user_register_input.password);
    let r3 = client
        .query_one(
            &s3,
            &[
                &user_register_input.email,
                &user_register_input.nick_name,
                &hased_password,
                &Role::User.to_i32(),
            ],
        )
        .await?;
    let user_id: i32 = r3.get("user_id");

    //设置verify_code为已使用
    client.execute(&s4, &[&email_id]).await?;

    let access_token = jwt::generate_access_token(user_id)?;
    let refresh_token = jwt::generate_refresh_token(user_id, &hex::encode(hased_password))?;
    Ok(HttpResponse::Ok().json(json!({
        "user_id": user_id,
        "access_token": access_token,
        "refresh_token": refresh_token
    })))
}

#[post("/new_token")]
pub async fn post_new_token(
    db_pool: web::Data<Pool>,
    input: web::Json<NewTokenInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let claims = jwt::parse_refresh_token(&input.refresh_token)?;

    let s1 = client
        .prepare_typed_cached(
            "SELECT password FROM common.user WHERE id = $1",
            &[DBType::INT4],
        )
        .await?;
    let r1 = client
        .query_one(&s1, &[&claims.user_id])
        .await
        .map_err(|e| match is_db_zero_line_error(&e) {
            true => ResponseError::input_err("用户不存在，请重新登陆", "找不到用户ID对应记录"),
            false => ResponseError::from(e),
        })?;
    let password: Vec<u8> = r1.get("password");
    let same = claims.password == hex::encode(password);
    if !same {
        return Err(ResponseError::input_err(
            "密码已更改，请重新登陆",
            "用户密码不匹配",
        ));
    }

    let access_token = jwt::generate_access_token(claims.user_id)?;
    let refresh_token = jwt::generate_refresh_token(claims.user_id, &claims.password)?;
    Ok(HttpResponse::Ok().json(json!({
        "user_id": claims.user_id,
        "access_token": access_token,
        "refresh_token": refresh_token
    })))
}

#[post["/reset_password"]]
pub async fn post_reset_password(
    db_pool: web::Data<Pool>,
    reset_password_input: web::Json<ResetPasswordInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;

    //准备语句
    let (s1, s2, s3) = futures::future::try_join3(
        client.prepare_typed_cached("SELECT id, used, code, created_at FROM common.verification_email WHERE type = $1 AND addr = $2 ORDER BY created_at DESC LIMIT 1", &[DBType::TEXT, DBType::TEXT]),
        client.prepare_typed_cached("UPDATE common.user SET password = $1 WHERE email = $2 RETURNING id",&[DBType::BYTEA, DBType::TEXT]),
        client.prepare_typed_cached("UPDATE common.verification_email SET used = TRUE WHERE id = $1",&[DBType::INT4])
    ).await?;

    //检查verify_code是否合法
    let r1 = client
        .query_one(
            &s1,
            &[
                &EmailType::PasswordReset.to_string(),
                &reset_password_input.email,
            ],
        )
        .await
        .map_err(|e| match is_db_zero_line_error(&e) {
            true => ResponseError::input_err(
                "无法验证邮箱，请尝试重新发送邮件",
                "该邮箱没有任何验证记录",
            ),
            false => ResponseError::from(e),
        })?;
    let email_id: i32 = r1.get("id");
    let used: bool = r1.get("used");
    let code: &str = r1.get("code");
    let created_at: SystemTime = r1.get("created_at");
    if used == true {
        return Err(ResponseError::input_err(
            "无法验证邮箱，请尝试重新发送邮件",
            "该验证记录已被使用",
        ));
    }
    if code != reset_password_input.verify_code {
        return Err(ResponseError::input_err(
            "无法验证邮箱，请检查验证码是否正确",
            "验证码不匹配",
        ));
    }
    if created_at < SystemTime::now() - Duration::from_secs(2 * 60 * 60) {
        return Err(ResponseError::input_err(
            "无法验证邮箱，请尝试重新发送邮件",
            "验证记录已过期",
        ));
    }

    //设置新密码
    let hased_password = hash::hash_password(&reset_password_input.new_password);
    let r2 = client
        .query_one(&s2, &[&hased_password, &reset_password_input.email])
        .await?;
    let user_id = r2.get("id");

    //设置verify_code为已使用
    client.execute(&s3, &[&email_id]).await?;

    let access_token = jwt::generate_access_token(user_id)?;
    let refresh_token = jwt::generate_refresh_token(user_id, &hex::encode(hased_password))?;
    Ok(HttpResponse::Ok().json(json!({
        "user_id": user_id,
        "access_token": access_token,
        "refresh_token": refresh_token
    })))
}
