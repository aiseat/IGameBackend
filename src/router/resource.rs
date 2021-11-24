use actix_web::{get, web, HttpRequest, HttpResponse};
use deadpool_postgres::{Client, Pool};
use futures::future::{try_join, try_join3, try_join4, try_join_all};

use crate::db::Type as DBType;
use crate::error::ResponseError;
use crate::model::{
    resource::{
        GetBriefResourcesOutput, GetBriefResourcesOutputItem, GetBriefResourcesPath,
        GetBriefResourcesQuery, GetResourceOutput, GetResourcePath, GetResourceUrlOutput,
        GetResourceUrlPath,
    },
    role::Permission,
};
use crate::resource_provider::ResourceProviderShare;
use crate::util::{jwt::parse_access_token, req_parse::get_access_token};

// 获取指定app的多个简短资源信息
#[get("/app/{app_id}/brief_resources")]
pub async fn get_brief_resources(
    db_pool: web::Data<Pool>,
    path: web::Path<GetBriefResourcesPath>,
    query: web::Query<GetBriefResourcesQuery>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let s1 = client
        .prepare_typed_cached(
            "SELECT id, name, version, allowed_exp
            FROM igame.resource
            WHERE app_id = $1 AND id > $2 LIMIT $3",
            &[DBType::INT4, DBType::INT4, DBType::INT4],
        )
        .await?;
    let r1s = client
        .query(&s1, &[&path.app_id, &query.last_index, &query.limit])
        .await?;
    let mut output: GetBriefResourcesOutput = Vec::new();
    for r1 in r1s {
        output.push(GetBriefResourcesOutputItem {
            resource_id: r1.get("id"),
            name: r1.get("name"),
            version: r1.get("version"),
            allowed_exp: r1.get("allowed_exp"),
        })
    }

    Ok(HttpResponse::Ok().json(output))
}

// 获取指定资源的详细信息
#[get("/resource/{resource_id}")]
pub async fn get_resource(
    db_pool: web::Data<Pool>,
    path: web::Path<GetResourcePath>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let s1 = client.prepare_typed_cached(
            "SELECT id, app_id, name, description, version, allowed_exp, downloaded, normal_download_cost, fast_download_cost, install_cost, normal_provider_ids, fast_provider_ids, updated_at
            FROM igame.resource
            WHERE id = $1",
            &[DBType::INT4]
        ).await?;

    let r1 = client.query_one(&s1, &[&path.resource_id]).await?;
    // 判断后端提供者id是否存在，如果不存在，禁止下载
    let normal_provider_ids: Vec<String> = r1.get("normal_provider_ids");
    let mut can_normal_download = true;
    if normal_provider_ids.is_empty() {
        can_normal_download = false;
    }
    let fast_provider_ids: Vec<String> = r1.get("fast_provider_ids");
    let mut can_fast_download = true;
    if fast_provider_ids.is_empty() {
        can_fast_download = false;
    }
    // 返回结果
    Ok(HttpResponse::Ok().json(GetResourceOutput {
        resource_id: r1.get("id"),
        app_id: r1.get("app_id"),
        name: r1.get("name"),
        description: r1.get("description"),
        version: r1.get("version"),
        allowed_exp: r1.get("allowed_exp"),
        downloaded: r1.get("downloaded"),
        normal_download_cost: r1.get("normal_download_cost"),
        fast_download_cost: r1.get("fast_download_cost"),
        install_cost: r1.get("install_cost"),
        can_normal_download,
        can_fast_download,
        updated_at: r1.get("updated_at"),
    }))
}

// 获取指定资源的url
#[get("/resource/{resource_id}/{client_group}/url")]
pub async fn get_resource_url(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    resource_provider: web::Data<ResourceProviderShare>,
    path: web::Path<GetResourceUrlPath>,
) -> Result<HttpResponse, ResponseError> {
    let mut client: Client = db_pool.get().await?;

    let vec_s = try_join_all(vec![
        // s0:获取资源url验证的必要信息
        client.prepare_typed_cached(
            "SELECT app_id, allowed_exp, normal_download_cost, fast_download_cost, normal_provider_ids, fast_provider_ids, file_path FROM igame.resource WHERE id = $1",
            &[DBType::INT4],
        ),
        // s1:获取用户的无限币跟经验值
        client.prepare_typed_cached(
            "SELECT coin, exp FROM igame.user WHERE id = $1",
            &[DBType::INT4],
        ),
        // s2:添加交易记录
        client.prepare_typed_cached(
            "INSERT INTO igame.trade (user_id, type, cost, resource_id) VALUES ($1, $2, $3, $4) RETURNING id",
            &[DBType::INT4, DBType::INT2, DBType::INT4, DBType::INT4],
        ),
        // s3:减少用户的无限币数量
        client.prepare_typed_cached(
            "UPDATE igame.user SET coin = coin - $1 WHERE id = $2 RETURNING coin",
            &[DBType::INT4, DBType::INT4],
        ),
        // s4:资源下载量+1
        client.prepare_typed_cached(
            "UPDATE igame.resource SET downloaded = downloaded + 1 WHERE id = $1 RETURNING downloaded",
            &[DBType::INT4],
        ),
        // s5:文章下载量+1
        client.prepare_typed_cached(
            "UPDATE igame.article SET downloaded = downloaded + 1 WHERE app_id = $1",
            &[DBType::INT4],
        ),
        // s6:检查用户是否可以免费下载，或者无视等级限制
        client.prepare_typed_cached(
            &format!(
                "SELECT bool_or({}) as free_download, bool_or({}) as ignore_exp
                FROM igame.role 
                WHERE id IN (
                    SELECT role_id 
                    FROM igame.user_role 
                    WHERE user_id = $1
                    AND (expire_at IS NULL OR (expire_at IS NOT NULL AND expire_at > now()))
                )",
                Permission::FreeDownload.to_string(), Permission::IgnoreExp.to_string()
            ),
            &[DBType::INT4],
        ),
    ]).await?;

    let download_url: String;
    let trade_id: i32;
    let remain_coin: i32;
    let downloaded: i32;
    if let Some(access_token) = get_access_token(&req) {
        // 如果是登陆用户
        let user_id = parse_access_token(&access_token)?.user_id;
        let (r0, r1, r6) = try_join3(
            client.query_one(&vec_s[0], &[&path.resource_id]),
            client.query_one(&vec_s[1], &[&user_id]),
            client.query_one(&vec_s[6], &[&user_id]),
        )
        .await?;
        let app_id: i32 = r0.get("app_id");
        let allowed_exp: i32 = r0.get("allowed_exp");
        let mut cost: i32 = r0.get(format!("{}_download_cost", &path.provider_group).as_str());
        let provider_ids: Vec<&str> =
            r0.get(format!("{}_provider_ids", &path.provider_group).as_str());
        if provider_ids.is_empty() {
            return Err(ResponseError::input_err(
                "该资源不存在",
                &format!(
                    "[资源ID：{}]{}_provider_ids不存在",
                    &path.resource_id, &path.provider_group,
                ),
            ));
        }
        let resource_path: String = r0.get("file_path");
        let user_coin: i32 = r1.get("coin");
        let user_exp: i32 = r1.get("exp");
        let can_free_download: bool = r6.get("free_download");
        let can_ignore_exp: bool = r6.get("ignore_exp");
        // 如果没有无视等级的权限，且用户等级小于要求等级
        if !can_ignore_exp && user_exp < allowed_exp {
            return Err(ResponseError::lack_exp_err(
                "用户等级不足，无法获取资源下载链接",
                allowed_exp,
                &format!("用户ID: {},资源ID: {}", &user_id, &path.resource_id),
            ));
        }
        // 如果没有免费下载的权限，且用户无限币小于价格
        if !can_free_download && user_coin < cost {
            return Err(ResponseError::lack_coin_err(
                "用户无限币不足，无法获取资源下载链接",
                cost,
                &format!("用户ID: {},资源ID: {}", &user_id, &path.resource_id),
            ));
        }
        // 如果可以免费下载，设置cost为0
        if can_free_download {
            cost = 0;
        }
        let transaction: deadpool_postgres::Transaction;
        (
            download_url,
            (transaction, trade_id, remain_coin, downloaded),
        ) = try_join(
            resource_provider.get_download_url(&resource_path, &path.provider_group, provider_ids),
            async {
                let trade_type: i16 = 1;
                let transaction = client.transaction().await?;
                let (r2, r3, r4, _) = try_join4(
                    transaction.query_one(
                        &vec_s[2],
                        &[&user_id, &trade_type, &cost, &path.resource_id],
                    ),
                    transaction.query_one(&vec_s[3], &[&cost, &user_id]),
                    transaction.query_one(&vec_s[4], &[&path.resource_id]),
                    transaction.execute(&vec_s[5], &[&app_id]),
                )
                .await?;
                return Ok((
                    transaction,
                    r2.get("id"),
                    r3.get("coin"),
                    r4.get("downloaded"),
                ));
            },
        )
        .await?;
        transaction.commit().await?;

        return Ok(HttpResponse::Ok().json(GetResourceUrlOutput {
            download_url,
            trade_id: Some(trade_id),
            remain_coin: Some(remain_coin),
            downloaded,
        }));
    } else {
        // 游客或未通过验证的用户
        let r0 = client.query_one(&vec_s[0], &[&path.resource_id]).await?;
        let app_id: i32 = r0.get("app_id");
        let allowed_exp: i32 = r0.get("allowed_exp");
        let cost: i32 = r0.get(format!("{}_download_cost", &path.provider_group).as_str());
        let provider_ids: Vec<&str> =
            r0.get(format!("{}_provider_ids", &path.provider_group).as_str());
        if provider_ids.is_empty() {
            return Err(ResponseError::input_err(
                "该资源不存在",
                &format!(
                    "[资源ID：{}]{}_provider_ids不存在",
                    &path.resource_id, &path.provider_group,
                ),
            ));
        }
        let resource_path: String = r0.get("file_path");

        if allowed_exp != 0 {
            return Err(ResponseError::lack_exp_err(
                "该资源有等级限制，游客无法获取资源下载链接",
                allowed_exp,
                &format!("用户ID: 游客,资源ID: {}", &path.resource_id),
            ));
        }
        if cost != 0 {
            return Err(ResponseError::lack_coin_err(
                "该资源需要消耗无限币，游客无法获取资源下载链接",
                cost,
                &format!("用户ID: 游客,资源ID: {}", &path.resource_id),
            ));
        }

        (download_url, downloaded) = try_join(
            resource_provider.get_download_url(&resource_path, &path.provider_group, provider_ids),
            async {
                let (r4, _) = try_join(
                    client.query_one(&vec_s[4], &[&path.resource_id]),
                    client.execute(&vec_s[5], &[&app_id]),
                )
                .await?;
                return Ok(r4.get("downloaded"));
            },
        )
        .await?;

        return Ok(HttpResponse::Ok().json(GetResourceUrlOutput {
            download_url,
            trade_id: None,
            remain_coin: None,
            downloaded,
        }));
    }
}
