use actix_web::{get, post, web, HttpRequest, HttpResponse};
use chrono::{DateTime, Datelike, Duration, FixedOffset, Utc};
use deadpool_postgres::{Client, Pool};
use futures::future::{try_join, try_join3, try_join4};

use crate::db::Type as DBType;
use crate::error::{is_db_zero_line_error, ResponseError};
use crate::model::{
    email::VerifyEmailType,
    role::{Permission, Role, RoleID},
    user::{
        GetMyselfOutput, GetUserOutput, GetUserPath, PostNewTokenInput, PostNewTokenOutput,
        PostUserDailyBonusOutput, PostUserInput, PostUserLoginInput, PostUserLoginOutput,
        PostUserOutput, PostUserRegisterInput, PostUserRegisterOutput, PostUserResetPasswordInput,
        PostUserResetPasswordOutput,
    },
};
use crate::util::{hash, jwt, req_parse::get_user_id};

#[get("/user/{user_id}")]
pub async fn get_user(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    path: web::Path<GetUserPath>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;

    let (s1, s2) = try_join(
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
                Permission::GetUser.to_string()
            ),
            &[DBType::INT4],
        ),
        client.prepare_typed_cached(
            "SELECT u.id, u.email, u.nick_name, u.exp, u.coin, u.avatar_url, u.login_at, u.created_at, array_agg(r.id) AS role_ids, array_agg(r.name) AS role_names, array_agg(ur.expire_at) AS role_expire_ats
            FROM igame.user AS u
            INNER JOIN igame.user_role AS ur
            ON u.id = ur.user_id
            INNER JOIN igame.role AS r
            ON ur.role_id = r.id
            WHERE u.id = $1
            GROUP BY u.id, u.email, u.nick_name, u.exp, u.coin, u.avatar_url, u.login_at, u.created_at",
            &[DBType::INT4],
        ),
    )
    .await?;

    // 检查是否有对应权限
    let r1 = client.query_one(&s1, &[&user_id]).await?;
    let has_permission: bool = r1.get(0);
    if !has_permission {
        return Err(ResponseError::permission_err(
            "获取用户信息失败，没有对应权限",
            &format!("[用户ID: {}]没有get_user权限", user_id),
        ));
    }

    // 获取用户信息
    let r2 = client.query_one(&s2, &[&path.user_id]).await?;
    // 转化成Vec<Role>类型
    let mut roles: Vec<Role> = Vec::new();
    let role_ids: Vec<i32> = r2.get("role_ids");
    if role_ids.len() > 0 {
        let role_names: Vec<&str> = r2.get("role_names");
        let role_expire_ats: Vec<Option<DateTime<Utc>>> = r2.get("role_expire_ats");
        for (index, role_id) in role_ids.iter().enumerate() {
            roles.push(Role {
                role_id: *role_id,
                name: role_names[index].to_string(),
                expire_at: role_expire_ats[index],
            })
        }
    }

    Ok(HttpResponse::Ok().json(GetUserOutput {
        user_id: r2.get("id"),
        email: r2.get("email"),
        nick_name: r2.get("nick_name"),
        exp: r2.get("exp"),
        coin: r2.get("coin"),
        roles: roles,
        avatar_url: r2.get("avatar_url"),
        login_at: r2.get("login_at"),
        created_at: r2.get("created_at"),
    }))
}

#[get("/myself")]
pub async fn get_myself(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;

    let s1 = client.prepare_typed_cached(
        "SELECT u.id, u.email, u.nick_name, u.exp, u.coin, u.avatar_url, u.login_at, u.created_at, array_agg(r.id) AS role_ids, array_agg(r.name) AS role_names, array_agg(ur.expire_at) AS role_expire_ats
        FROM igame.user AS u
        INNER JOIN igame.user_role AS ur
        ON u.id = ur.user_id
        INNER JOIN igame.role AS r
        ON ur.role_id = r.id
        WHERE u.id = $1
        GROUP BY u.id, u.email, u.nick_name, u.exp, u.coin, u.avatar_url, u.login_at, u.created_at",
        &[DBType::INT4],
    ).await?;
    // 获取用户信息
    let r1 = client.query_one(&s1, &[&user_id]).await?;
    // 转化成Vec<Role>类型
    let mut roles: Vec<Role> = Vec::new();
    let role_ids: Vec<i32> = r1.get("role_ids");
    if role_ids.len() > 0 {
        let role_names: Vec<&str> = r1.get("role_names");
        let role_expire_ats: Vec<Option<DateTime<Utc>>> = r1.get("role_expire_ats");
        for (index, role_id) in role_ids.iter().enumerate() {
            roles.push(Role {
                role_id: *role_id,
                name: role_names[index].to_string(),
                expire_at: role_expire_ats[index],
            })
        }
    }

    Ok(HttpResponse::Ok().json(GetMyselfOutput {
        user_id: r1.get("id"),
        email: r1.get("email"),
        nick_name: r1.get("nick_name"),
        exp: r1.get("exp"),
        coin: r1.get("coin"),
        roles: roles,
        avatar_url: r1.get("avatar_url"),
        login_at: r1.get("login_at"),
        created_at: r1.get("created_at"),
    }))
}

#[post("/user/login")]
pub async fn post_user_login(
    db_pool: web::Data<Pool>,
    input: web::Json<PostUserLoginInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;

    let (s1, s2) = try_join(
        // 获取用户id跟密码
        client.prepare_typed_cached(
            "SELECT id, password
            FROM igame.user
            WHERE email = $1",
            &[DBType::TEXT],
        ),
        // 设置login_at时间
        client.prepare_typed_cached(
            "UPDATE igame.user
            SET login_at = $1
            WHERE id = $2",
            &[DBType::TIMESTAMPTZ, DBType::INT4],
        ),
    )
    .await?;

    let r1 =
        client.query_one(&s1, &[&input.email]).await.map_err(|e| {
            match is_db_zero_line_error(&e) {
                true => ResponseError::input_err("邮箱或密码不正确，请重新输入", "错误的邮箱地址"),
                false => ResponseError::from(e),
            }
        })?;
    let user_id: i32 = r1.get("id");
    let password: Vec<u8> = r1.get("password");
    let same = hash::compare_password(&input.password, &password)?;
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
    let refresh_token = jwt::generate_refresh_token(user_id, &hex::encode(password))?;
    Ok(HttpResponse::Ok().json(PostUserLoginOutput {
        user_id,
        access_token,
        refresh_token,
    }))
}

#[post("/user/register")]
pub async fn post_user_register(
    db_pool: web::Data<Pool>,
    input: web::Json<PostUserRegisterInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;

    let (s1, s2, s3, s4) = try_join4(
        // 获取最近的一次注册验证邮件
        client.prepare_typed_cached(
            "SELECT id, code, used, created_at
            FROM igame.verify_email
            WHERE type = $1 AND addr = $2
            ORDER BY created_at DESC
            LIMIT 1",
            &[DBType::INT2, DBType::TEXT],
        ),
        // 判断用户的邮箱是否存在
        client.prepare_typed_cached(
            "SELECT EXISTS(SELECT 1 FROM igame.user WHERE email = $1)",
            &[DBType::TEXT],
        ),
        // 添加记录到igame.user,igame.user_notice, igame.user_role表中
        client.prepare_typed_cached(
            "WITH
            u AS (
                INSERT INTO igame.user(email, nick_name, password)
                VALUES($1, $2, $3) RETURNING id
            ),
            n AS (
                INSERT INTO igame.user_notice(user_id, notice_id)
                SELECT (SELECT id FROM u), id FROM igame.notice
                WHERE send_new_user = true
            )
            INSERT INTO igame.user_role(user_id, role_id) 
            SELECT id, $4 FROM u RETURNING user_id",
            &[DBType::TEXT, DBType::TEXT, DBType::BYTEA, DBType::INT4],
        ),
        // 将注册验证邮件设置为已使用
        client.prepare_typed_cached(
            "UPDATE igame.verify_email
            SET used = true
            WHERE id = $1",
            &[DBType::INT4],
        ),
    )
    .await?;

    let (r1, r2) = try_join(
        client.query_one(
            &s1,
            &[&VerifyEmailType::UserRegister.to_int2(), &input.email],
        ),
        client.query_one(&s2, &[&input.email]),
    )
    .await
    .map_err(|e| match is_db_zero_line_error(&e) {
        true => ResponseError::input_err(
            "无法验证邮箱，请尝试重新发送邮件",
            &format!("[邮箱地址: {}]未找到对应的注册验证邮件", &input.email),
        ),
        false => ResponseError::from(e),
    })?;
    let email_id: i32 = r1.get("id");
    let code: &str = r1.get("code");
    let used: bool = r1.get("used");
    let created_at: DateTime<Utc> = r1.get("created_at");
    if used == true {
        return Err(ResponseError::input_err(
            "该验证码已被使用，请尝试重新发送邮件",
            &format!("[邮箱地址: {}]注册验证码已被使用", &input.email),
        ));
    }
    if code != input.verify_code {
        return Err(ResponseError::input_err(
            "验证码错误，请尝试重新输入",
            &format!(
                "[邮箱地址: {}]发送的注册验证码{}与记录值不匹配",
                &input.email, &input.verify_code
            ),
        ));
    }
    if created_at < Utc::now() - Duration::hours(2) {
        return Err(ResponseError::input_err(
            "验证码已过期，请尝试重新发送邮件",
            &format!("[邮箱地址: {}]注册验证码已过期", &input.email),
        ));
    }
    let exist: bool = r2.get(0);
    if exist {
        return Err(ResponseError::input_err(
            "该邮箱地址已注册，请使用其他邮箱",
            &format!("[邮箱地址: {}]该地址早已存在", &input.email),
        ));
    }

    let hased_password = hash::hash_password(&input.password);
    let (r3, _) = try_join(
        //创建新用户
        client.query_one(
            &s3,
            &[
                &input.email,
                &input.nick_name,
                &hased_password,
                &RoleID::User.to_i32(),
            ],
        ),
        //设置verify_code为已使用
        client.execute(&s4, &[&email_id]),
    )
    .await?;
    let user_id: i32 = r3.get("user_id");

    let access_token = jwt::generate_access_token(user_id)?;
    let refresh_token = jwt::generate_refresh_token(user_id, &hex::encode(hased_password))?;
    Ok(HttpResponse::Ok().json(PostUserRegisterOutput {
        user_id,
        access_token,
        refresh_token,
    }))
}

#[post("/user/new_token")]
pub async fn post_user_new_token(
    db_pool: web::Data<Pool>,
    input: web::Json<PostNewTokenInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let claims = jwt::parse_refresh_token(&input.refresh_token)?;

    let s1 = client
        .prepare_typed_cached(
            "SELECT password FROM igame.user WHERE id = $1",
            &[DBType::INT4],
        )
        .await?;
    let r1 = client
        .query_one(&s1, &[&claims.user_id])
        .await
        .map_err(|e| match is_db_zero_line_error(&e) {
            true => ResponseError::input_err(
                "用户不存在，请重新登陆",
                &format!("[用户ID: {}]该用户不存在", &claims.user_id),
            ),
            false => ResponseError::from(e),
        })?;
    let password: Vec<u8> = r1.get("password");
    let same = claims.password == hex::encode(password);
    if !same {
        return Err(ResponseError::input_err(
            "密码已更改，请重新登陆",
            &format!("[用户ID: {}]密码不匹配", &claims.user_id),
        ));
    }

    let access_token = jwt::generate_access_token(claims.user_id)?;
    let refresh_token = jwt::generate_refresh_token(claims.user_id, &claims.password)?;
    Ok(HttpResponse::Ok().json(PostNewTokenOutput {
        user_id: claims.user_id,
        access_token,
        refresh_token,
    }))
}

#[post["/user/reset_password"]]
pub async fn post_user_reset_password(
    db_pool: web::Data<Pool>,
    input: web::Json<PostUserResetPasswordInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;

    let (s1, s2, s3, s4) = try_join4(
        // 获取最近的一次重置密码验证邮件
        client.prepare_typed_cached(
            "SELECT id, code, used, created_at
            FROM igame.verify_email
            WHERE type = $1 AND addr = $2
            ORDER BY created_at DESC
            LIMIT 1",
            &[DBType::INT2, DBType::TEXT],
        ),
        // 判断用户的邮箱是否存在
        client.prepare_typed_cached(
            "SELECT EXISTS(SELECT 1 FROM igame.user WHERE email = $1)",
            &[DBType::TEXT],
        ),
        // 重置密码
        client.prepare_typed_cached(
            "UPDATE igame.user
            SET password = $1
            WHERE email = $2
            RETURNING id",
            &[DBType::BYTEA, DBType::TEXT],
        ),
        // 设置验证邮件为已使用
        client.prepare_typed_cached(
            "UPDATE igame.verify_email
            SET used = TRUE
            WHERE id = $1",
            &[DBType::INT4],
        ),
    )
    .await?;

    let (r1, r2) = try_join(
        client.query_one(
            &s1,
            &[&VerifyEmailType::UserRegister.to_int2(), &input.email],
        ),
        client.query_one(&s2, &[&input.email]),
    )
    .await
    .map_err(|e| match is_db_zero_line_error(&e) {
        true => ResponseError::input_err(
            "无法验证邮箱，请尝试重新发送邮件",
            &format!("[邮箱地址: {}]未找到对应的重置密码验证邮件", &input.email),
        ),
        false => ResponseError::from(e),
    })?;
    let email_id: i32 = r1.get("id");
    let code: &str = r1.get("code");
    let used: bool = r1.get("used");
    let created_at: DateTime<Utc> = r1.get("created_at");
    if used == true {
        return Err(ResponseError::input_err(
            "该验证码已被使用，请尝试重新发送邮件",
            &format!("[邮箱地址: {}]重置密码验证码已被使用", &input.email),
        ));
    }
    if code != input.verify_code {
        return Err(ResponseError::input_err(
            "验证码错误，请尝试重新输入",
            &format!(
                "[邮箱地址: {}]发送的重置密码验证码{}与记录值不匹配",
                &input.email, &input.verify_code
            ),
        ));
    }
    if created_at < Utc::now() - Duration::hours(2) {
        return Err(ResponseError::input_err(
            "验证码已过期，请尝试重新发送邮件",
            &format!("[邮箱地址: {}]重置密码验证码已过期", &input.email),
        ));
    }
    let exist: bool = r2.get(0);
    if !exist {
        return Err(ResponseError::input_err(
            "该邮箱地址不存在，请使用其他邮箱",
            &format!("[邮箱地址: {}]该地址不存在", &input.email),
        ));
    }

    let hased_password = hash::hash_password(&input.new_password);
    let (r3, _) = try_join(
        //设置新密码
        client.query_one(&s3, &[&hased_password, &input.email]),
        //设置verify_code为已使用
        client.execute(&s4, &[&email_id]),
    )
    .await?;
    let user_id = r3.get("id");

    let access_token = jwt::generate_access_token(user_id)?;
    let refresh_token = jwt::generate_refresh_token(user_id, &hex::encode(hased_password))?;
    Ok(HttpResponse::Ok().json(PostUserResetPasswordOutput {
        user_id,
        access_token,
        refresh_token,
    }))
}

// 创建用户
#[post("/user")]
pub async fn post_user(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    input: web::Json<PostUserInput>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;

    let (s1, s2, s3) = try_join3(
        // 检查是否有创建用户的权限
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
                Permission::CreateUser.to_string()
            ),
            &[DBType::INT4],
        ),
        // 判断用户的邮箱是否存在
        client.prepare_typed_cached(
            "SELECT EXISTS(SELECT 1 FROM igame.user WHERE email = $1)",
            &[DBType::TEXT],
        ),
        // 添加记录到igame.user,igame.user_notice, igame.user_role表中
        client.prepare_typed_cached(
            "WITH
            u AS (
                INSERT INTO igame.user(email, nick_name, password)
                VALUES($1, $2, $3) RETURNING id
            ),
            n AS (
                INSERT INTO igame.user_notice(user_id, notice_id)
                SELECT (SELECT id FROM u), id FROM igame.notice
                WHERE send_new_user = true
            )
            INSERT INTO igame.user_role(user_id, role_id) 
            SELECT id, $4 FROM u RETURNING user_id",
            &[DBType::TEXT, DBType::TEXT, DBType::BYTEA, DBType::INT4],
        ),
    )
    .await?;

    let (r1, r2) = try_join(
        // 检查是否有对应权限
        client.query_one(&s1, &[&user_id]),
        //检查邮箱是否存在
        client.query_one(&s2, &[&input.email]),
    )
    .await?;
    let has_permission: bool = r1.get(0);
    if !has_permission {
        return Err(ResponseError::permission_err(
            "创建用户失败，没有对应权限",
            &format!("[用户ID: {}]没有create_user权限", user_id),
        ));
    }
    let exist: bool = r2.get(0);
    if exist {
        return Err(ResponseError::input_err(
            "该邮箱地址已注册，请使用其他邮箱",
            &format!("[邮箱地址: {}]该地址早已存在", &input.email),
        ));
    }

    // 添加用户
    let hased_password = hash::hash_password(&input.password);
    let r3 = client
        .query_one(
            &s3,
            &[
                &input.email,
                &input.nick_name,
                &hased_password,
                &input.role.to_i32(),
            ],
        )
        .await?;
    let user_id: i32 = r3.get("id");

    Ok(HttpResponse::Ok().json(PostUserOutput { user_id }))
}

#[post("/user/daily_bonus")]
pub async fn post_user_daily_bonus(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
) -> Result<HttpResponse, ResponseError> {
    let mut client: Client = db_pool.get().await?;
    let user_id = get_user_id(&req)?;

    let (s1, s2, s3) = try_join3(
        // 获取签到记录
        client.prepare_typed_cached(
            "SELECT time, count 
            FROM igame.daily_bonus 
            WHERE user_id = $1 
            ORDER BY time DESC 
            LIMIT 1",
            &[DBType::INT4],
        ),
        // 更新签到记录
        client.prepare_typed_cached(
            "INSERT INTO igame.daily_bonus(user_id, count) 
            VALUES($1, $2) 
            RETURNING id",
            &[DBType::INT4, DBType::INT4],
        ),
        // 更新用户的无限币跟经验
        client.prepare_typed_cached(
            "UPDATE igame.user 
            SET coin = coin + $1, exp = exp + $2 
            WHERE id = $3 
            RETURNING coin, exp",
            &[DBType::INT4, DBType::INT4, DBType::INT4],
        ),
    )
    .await?;

    // 获取累积签到次数
    let count: i32;
    match client.query_one(&s1, &[&user_id]).await {
        Ok(r1) => {
            let count_tmp: i32 = r1.get("count");
            let last_time_utc: DateTime<Utc> = r1.get("time");
            let china_timezone = FixedOffset::east(8 * 60 * 60);
            let last_time_china = last_time_utc.with_timezone(&china_timezone);
            let now_time_utc = Utc::now();
            let now_time_china = now_time_utc.with_timezone(&china_timezone);
            // 最近签到日期是今天
            if last_time_china.day() == now_time_china.day() {
                return Err(ResponseError::already_done_err(
                    "本日已签到，无法再次领取奖励",
                    &format!("用户已签到, 用户ID: {}", user_id),
                ));
            }
            // 最近签到日期是昨天
            if last_time_china.day() == (now_time_china - Duration::days(1)).day() {
                count = count_tmp + 1;
            // 最近签到日期既不是今天也不是昨天
            } else {
                count = 1;
            }
        }
        Err(e) => match is_db_zero_line_error(&e) {
            true => {
                count = 1;
            }
            false => return Err(e.into()),
        },
    }

    // 计算本次签到获取的coin与exp
    let added_coin: i32;
    if count > 30 {
        added_coin = 40;
    } else {
        added_coin = 9 + count;
    }
    let added_exp = 10;

    // 启用事务来更新签到后的用户信息，以及插入新的签到行
    let transaction = client.transaction().await?;
    let (r2, r3) = try_join(
        transaction.query_one(&s2, &[&user_id, &count]),
        transaction.query_one(&s3, &[&added_coin, &added_exp, &user_id]),
    )
    .await?;
    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(PostUserDailyBonusOutput {
        daily_bonus_id: r2.get("id"),
        count,
        added_coin,
        added_exp,
        total_coin: r3.get("coin"),
        total_exp: r3.get("exp"),
    }))
}
