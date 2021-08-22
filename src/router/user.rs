use actix_web::{get, post, web, HttpRequest, HttpResponse};
use deadpool_postgres::{Client, Pool};
use serde_json::json;
use futures::future::{try_join, try_join3, try_join4};
use chrono::{DateTime, Duration, Utc, FixedOffset, Datelike};

use crate::db::Type as DBType;
use crate::error::ResponseError;
use crate::model::{
    permission::Permission,
    user::{User, GetUserPath, UserCreateInput},
};
use crate::util::{req_parse::get_user_id, hash::hash_password, is_db_zero_line_error};

// GetUser权限
#[get("/user/{id}")]
pub async fn get_user(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    path: web::Path<GetUserPath>
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;

    let (s1, s2) = try_join(
        client.prepare_typed_cached(
        &format!("SELECT bool_or({}) FROM igame.role WHERE id IN (SELECT role_id FROM igame.user_role WEHRE user_id = $1)", Permission::GetUser.to_string()),
        &[DBType::INT4]), 
        client.prepare_typed_cached("SELECT * FROM common.user WHERE id = $1", &[DBType::INT4])
    ).await?;

    //检查是否有对应权限
    let r1 = client.query_one(&s1, &[&user_id]).await?;
    let has_permission: bool = r1.get(0);
    if !has_permission {
        return Err(ResponseError::new_permission_error(
            "No corresponding permissions",
            Some("没有对应权限"),
        ));
    }

    //获取用户信息
    let r2 = client.query_one(&s2, &[&path.id]).await?;
    let user = User{
        id: r2.get("id"),
        email: r2.get("email"),
        nick_name: r2.get("nick_name"),
        created_at: r2.get("created_at"),
        login_at: r2.get("login_at"),
        avatar_url: r2.get("avatar_url"),
        exp: r2.get("exp"),
        coin: r2.get("coin"),
        ..User::default()
    };

    Ok(HttpResponse::Ok().json(user))
}

//CreateUser 权限
#[post("/user")]
pub async fn post_user(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    user_create_input: web::Json<UserCreateInput>,
) -> Result<HttpResponse, ResponseError> {
    let mut client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;

    let (s1, s2, s3, s4) = try_join4(
        client.prepare_typed_cached(
            &format!("SELECT bool_or({}) FROM igame.role WHERE id IN (SELECT role_id FROM igame.user_role WEHRE user_id = $1)", Permission::GetUser.to_string()),
            &[DBType::INT4]),
        client.prepare_typed_cached(
            "SELECT EXISTS(SELECT 1 FROM common.user WHERE email = $1)",
            &[DBType::TEXT]),
        client.prepare_typed_cached(
            "INSERT INTO common.user(email, nick_name, password) VALUES($1, $2, $3) RETURNING id",
            &[DBType::TEXT, DBType::TEXT, DBType::BYTEA]),
        client.prepare_typed_cached(
            "INSERT INTO igame.user_role(user_id, role_id) VALUES($1, $2)",
            &[DBType::INT4, DBType::INT4]),
    ).await?;

    // 检查是否有对应权限
    let r1 = client.query_one(&s1, &[&user_id]).await?;
    let has_permission: bool = r1.get(0);
    if !has_permission {
        return Err(ResponseError::new_permission_error(
            "No corresponding permissions",
            Some("没有对应权限"),
        ));
    }

    //检查邮箱是否存在
    let r2 = client.query_one(&s2, &[&user_create_input.email]).await?;
    let exist = r2.get(0);
    if exist {
        return Err(ResponseError::new_input_error(
            "Email addr is already exist",
            Some("邮箱地址已注册，请使用其他邮箱"),
        ));
    }

    // 添加用户
    let transaction = client.transaction().await?;
    let hased_password = hash_password(&user_create_input.password);
    let r1 = transaction
        .query_one(
            &s3,
            &[
                &user_create_input.email,
                &user_create_input.nick_name,
                &hased_password,
            ],
        )
        .await?;
    let user_id: i32 = r1.get("id");
    transaction.execute(&s4, &[&user_id, &user_create_input.role.to_i32()]).await?;
    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(json!({ "user_id": user_id })))
}

#[get("/myself")]
pub async fn get_myself(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;

    let s1 = client
        .prepare_typed_cached("SELECT id, email, nick_name, created_at, login_at, avatar_url, exp, coin FROM common.user WHERE id = $1", &[DBType::INT4])
        .await?;
    let r1 = client.query_one(&s1, &[&user_id]).await?;
    let user = User {
        id: r1.get("id"),
        email: r1.get("email"),
        nick_name: r1.get("nick_name"),
        created_at: r1.get("created_at"),
        login_at: r1.get("login_at"),
        avatar_url: r1.get("avatar_url"),
        exp: r1.get("exp"),
        coin: r1.get("coin"),
        ..User::default()
    };

    Ok(HttpResponse::Ok().json(user))
}

#[post("/daily_bonus")]
pub async fn post_daily_bonus(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
) -> Result<HttpResponse, ResponseError> {
    let mut client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;

    let (s1, s2, s3) = try_join3(
        client.prepare_typed_cached("SELECT time, count FROM common.daily_bonus WHERE user_id = $1 ORDER BY time DESC LIMIT 1", &[DBType::INT4]),
        client.prepare_typed_cached("INSERT INTO common.daily_bonus(user_id, count) VALUES($1, $2) RETURNING id", &[DBType::INT4, DBType::INT4]),
        client.prepare_typed_cached("UPDATE common.user SET coin = coin + $1, exp = exp + $2 WHERE id = $3 RETURNING coin, exp", &[DBType::INT4, DBType::INT4, DBType::INT4]),
    ).await?;


    // 获取签到累计值
    let count: i32;
    match client.query_one(&s1, &[&user_id]).await {
        Ok(r1) => {
            let count_tmp: i32 = r1.get("count");
            let last_time_utc: DateTime<Utc> = r1.get("time");
            let china_timezone = FixedOffset::east(8*60*60);
            let last_time_china = last_time_utc.with_timezone(&china_timezone);
            let now_time_utc = Utc::now();
            let now_time_china = now_time_utc.with_timezone(&china_timezone);
            if last_time_china.day() == now_time_china.day() {
                return Err(ResponseError::new_already_done_error(
                    &format!("用户已签到, 用户ID: {}", user_id), 
                    Some("本日已签到，无法再次领取奖励")
                ));
            }
            if last_time_china.day() == (now_time_china - Duration::days(1)).day() {
                count = count_tmp + 1;
            } else {
                count = 1;
            }
        },
        Err(e) => {
            match is_db_zero_line_error(&e) {
                true => {
                    count = 1;
                },
                false => return Err(e.into()),
            }
        }
    }

    // 计算本次签到获取的coin与exp
    let added_coin: i32;
    if count > 16 {
        added_coin = 20;
    } else {
        added_coin = 4 + count;
    }
    let added_exp = 5;

    // 启用事务来更新签到后的用户信息，以及插入新的签到行
    let transaction = client.transaction().await?;
    let (r1, r2) = try_join(
        transaction.query_one(&s2, &[&user_id, &count]), 
        transaction.query_one(&s3, &[&added_coin, &added_exp, &user_id])
    ).await?;
    transaction.commit().await?;
    let daily_bonus_id: i32 = r1.get("id");
    let total_coin: i32 = r2.get("coin");
    let total_exp: i32 = r2.get("exp");

    Ok(HttpResponse::Ok().json(json!({
        "daily_bonus_id": daily_bonus_id,
        "added_coin": added_coin,
        "added_exp": added_exp,
        "total_coin": total_coin,
        "total_exp": total_exp,
    })))
}