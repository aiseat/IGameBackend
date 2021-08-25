use actix_web::{get, web, HttpRequest, HttpResponse};
use deadpool_postgres::{Client, Pool};
use futures::future::{try_join, try_join3, try_join_all};
use serde_json::json;
use std::collections::HashMap;

use crate::db::Type as DBType;
use crate::error::ResponseError;
use crate::model::{
    permission::Permission,
    resource::{
        DependResource, DependResourceWithoutArticleId, GetResourceOutput, GetResourcePath,
        GetResourceUrlPath, GetResourceUrlQuery,
    },
};
use crate::resource_provider::ResourceProviderShare;
use crate::util::{jwt::parse_access_token, req_parse::get_access_token};

#[get("/resource/{id}")]
pub async fn get_resource(
    db_pool: web::Data<Pool>,
    path: web::Path<GetResourcePath>,
) -> Result<HttpResponse, ResponseError> {
    let client: Client = db_pool.get().await?;
    let (s1, s2, s3) = try_join3(
        client.prepare_typed_cached(
            "WITH r1 AS (
                SELECT id, name, allowed_exp, downloaded, full_costs, update_costs, supported_systems, change_log, depend_ids, updated_at
                FROM common.resource AS r
                WHERE r.id = $1
            )
            SELECT r1.id, r1.name, r1.allowed_exp, r1.downloaded, r1.full_costs, r1.update_costs, r1.supported_systems, r1.change_log, r1.updated_at, array_agg(r2.id) AS depend_ids, array_agg(r2.name) AS depend_names, array_agg(r2.app_id) AS depend_app_ids, array_agg(r2.app_type) AS depend_app_types
            FROM r1
            LEFT JOIN common.resource AS r2
            ON r2.id = ANY(r1.depend_ids)
            GROUP BY r1.id, r1.name, r1.allowed_exp, r1.downloaded, r1.full_costs, r1.update_costs, r1.supported_systems, r1.change_log, r1.updated_at",
            &[DBType::INT4]
        ),
        client.prepare_typed_cached(
            "SELECT a.id, a.app_id
            FROM igame.game_article AS a
            WHERE a.app_id = ANY($1)",
            &[DBType::INT4_ARRAY]
        ),
        client.prepare_typed_cached(
            "SELECT a.id, a.app_id
            FROM igame.mod_article AS a
            WHERE a.app_id = ANY($1)",
            &[DBType::INT4_ARRAY]
        )
    ).await?;

    let r1 = client.query_one(&s1, &[&path.id]).await?;
    let depend_ids: Vec<Option<i32>> = r1.get("depend_ids");
    let mut depends: Vec<DependResource> = Vec::new();
    // 判断depend_ids是否有值
    if depend_ids[0].is_some() {
        let depend_names: Vec<&str> = r1.get("depend_names");
        let depend_app_ids: Vec<i32> = r1.get("depend_app_ids");
        let depend_app_types: Vec<i16> = r1.get("depend_app_types");
        let mut depend_map = HashMap::new();
        let mut game_depend_app_ids = Vec::new();
        let mut mod_depend_app_ids = Vec::new();
        for (index, depend_id) in depend_ids.iter().enumerate() {
            match depend_app_types[index] {
                2 => game_depend_app_ids.push(depend_app_ids[index]),
                3 => mod_depend_app_ids.push(depend_app_ids[index]),
                _ => {
                    return Err(ResponseError::unexpected_err(
                        "数据库内部错误",
                        "depend_id的type不满足约定",
                    ))
                }
            };
            depend_map.insert(
                depend_app_ids[index],
                DependResourceWithoutArticleId {
                    id: depend_id.unwrap(),
                    name: depend_names[index].to_string(),
                    app_type: depend_app_types[index],
                },
            );
        }
        let mut r2s = Vec::new();
        let mut r3s = Vec::new();
        if game_depend_app_ids.len() > 0 && mod_depend_app_ids.len() > 0 {
            (r2s, r3s) = try_join(
                client.query(&s2, &[&game_depend_app_ids]),
                client.query(&s3, &[&mod_depend_app_ids]),
            )
            .await?;
        } else if game_depend_app_ids.len() > 0 {
            r2s = client.query(&s2, &[&game_depend_app_ids]).await?;
        } else if mod_depend_app_ids.len() > 0 {
            r3s = client.query(&s3, &[&mod_depend_app_ids]).await?;
        }
        for r2 in r2s {
            let id = r2.get("id");
            let app_id = r2.get("app_id");
            let dr = depend_map.get(&app_id).unwrap();
            depends.push(DependResource {
                id: dr.id,
                name: dr.name.clone(),
                app_type: dr.app_type,
                article_id: id,
            })
        }
        for r3 in r3s {
            let id = r3.get("id");
            let app_id = r3.get("app_id");
            let dr = depend_map.get(&app_id).unwrap();
            depends.push(DependResource {
                id: dr.id,
                name: dr.name.clone(),
                app_type: dr.app_type,
                article_id: id,
            })
        }
    }

    Ok(HttpResponse::Ok().json(GetResourceOutput {
        id: r1.get("id"),
        name: r1.get("name"),
        allowed_exp: r1.get("allowed_exp"),
        downloaded: r1.get("downloaded"),
        full_costs: r1.get("full_costs"),
        update_costs: r1.get("update_costs"),
        supported_systems: r1.get("supported_systems"),
        change_log: r1.get("change_log"),
        updated_at: r1.get("updated_at"),
        depends: depends,
    }))
}

#[get("/resource/{id}/url")]
pub async fn get_resource_url(
    req: HttpRequest,
    db_pool: web::Data<Pool>,
    resource_provider: web::Data<ResourceProviderShare>,
    path: web::Path<GetResourceUrlPath>,
    query: web::Query<GetResourceUrlQuery>,
) -> Result<HttpResponse, ResponseError> {
    let mut client: Client = db_pool.get().await?;
    let resource_id = &path.id;
    let resource_type = &query.r#type;
    let client_group = &query.group;

    let vec_s = try_join_all(vec![
        client.prepare_typed_cached(
            &format!(
                "SELECT allowed_exp, paths, {}_costs AS costs FROM common.resource WHERE id = $1",
                resource_type.to_string()
            ),
            &[DBType::INT4],
        ),
        client.prepare_typed_cached(
            "SELECT coin, exp FROM common.user WHERE id = $1",
            &[DBType::INT4],
        ),
        client.prepare_typed_cached(
            "INSERT INTO igame.trade (user_id, type, value) VALUES ($1, $2, $3) RETURNING id",
            &[DBType::INT4, DBType::INT2, DBType::INT4],
        ),
        client.prepare_typed_cached(
            "UPDATE common.user SET coin = coin - $1 WHERE id = $2 RETURNING coin",
            &[DBType::INT4, DBType::INT4],
        ),
        client.prepare_typed_cached(
            "UPDATE common.resource SET downloaded = downloaded + 1 WHERE id = $1 RETURNING downloaded",
            &[DBType::INT4],
        ),
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
    let downloaded: i32;
    if let Some(access_token) = get_access_token(&req) {
        // 登陆用户
        let user_id = parse_access_token(&access_token)?.user_id;
        let (r1, r2, r6) = try_join3(
            client.query_one(&vec_s[0], &[resource_id]),
            client.query_one(&vec_s[1], &[&user_id]),
            client.query_one(&vec_s[5], &[&user_id]),
        )
        .await?;
        let resource_paths: Vec<&str> = r1.get("paths");
        let resource_path = resource_paths[resource_type.to_index()];
        if resource_path == "" {
            return Err(ResponseError::input_err(
                "该资源不存在",
                &format!(
                    "资源id：{}的路径{}不存在",
                    resource_id,
                    resource_type.to_index()
                ),
            ));
        };
        let allowed_exp: i32 = r1.get("allowed_exp");
        let costs: Vec<i32> = r1.get("costs");
        let cost = costs[client_group.to_index()];
        let user_coin: i32 = r2.get("coin");
        let user_exp: i32 = r2.get("exp");
        let can_free_download: bool = r6.get("free_download");
        let can_ignore_exp: bool = r6.get("ignore_exp");

        if !can_ignore_exp && user_exp < allowed_exp {
            return Err(ResponseError::lack_exp_err(
                "无法获取资源链接",
                &format!(
                    "请求资源id:{}, type:{}, client_group: {}",
                    resource_id, resource_type, client_group
                ),
            ));
        }
        if !can_free_download && user_coin < cost {
            return Err(ResponseError::lack_coin_err(
                "无法获取资源链接",
                &format!(
                    "请求资源id:{}, type:{}, client_group: {}",
                    resource_id, resource_type, client_group
                ),
            ));
        }

        if !can_free_download && cost > 0 {
            // 如果不能免费下载且资源费用大于0，进行交易
            let trade_id: i32;
            let remain_coin: i32;
            let transaction: deadpool_postgres::Transaction;
            (
                download_url,
                (transaction, trade_id, remain_coin, downloaded),
            ) = try_join(
                resource_provider.get_download_url(resource_path, client_group),
                async {
                    let trade_type: i16 = 1;
                    let transaction = client.transaction().await?;
                    let (r3, r4, r5) = try_join3(
                        transaction.query_one(&vec_s[2], &[&user_id, &trade_type, &cost]),
                        transaction.query_one(&vec_s[3], &[&cost, &user_id]),
                        transaction.query_one(&vec_s[4], &[resource_id]),
                    )
                    .await?;
                    return Ok((
                        transaction,
                        r3.get("id"),
                        r4.get("coin"),
                        r5.get("downloaded"),
                    ));
                },
            )
            .await?;
            transaction.commit().await?;

            return Ok(HttpResponse::Ok().json(json!({
                "download_url": download_url,
                "trade_id": trade_id,
                "remain_coin": remain_coin,
                "downloaded": downloaded
            })));
        } else {
            // 费用为0，直接下载
            (download_url, downloaded) = try_join(
                resource_provider.get_download_url(resource_path, client_group),
                async {
                    let r5 = client.query_one(&vec_s[4], &[resource_id]).await?;
                    return Ok(r5.get("downloaded"));
                },
            )
            .await?;

            return Ok(HttpResponse::Ok().json(json!({ 
                "download_url": download_url, 
                "downloaded": downloaded })));
        }
    } else {
        // 游客或未通过验证的用户
        let r1 = client.query_one(&vec_s[0], &[resource_id]).await?;
        let resource_paths: Vec<&str> = r1.get("paths");
        let resource_path = resource_paths[resource_type.to_index()];
        if resource_path == "" {
            return Err(ResponseError::input_err(
                "该资源不存在",
                &format!(
                    "资源id：{}的路径{}不存在",
                    resource_id,
                    resource_type.to_index()
                ),
            ));
        };
        let allowed_exp: i32 = r1.get("allowed_exp");
        let costs: Vec<i32> = r1.get("costs");

        let cost = costs[client_group.to_index()];
        if allowed_exp != 0 || cost != 0 {
            return Err(ResponseError::permission_err(
                "只有登陆用户有权下载本资源",
                &format!(
                    "请求资源id:{}, type:{}, client_group: {}",
                    resource_id, resource_type, client_group
                ),
            ));
        }

        (download_url, downloaded) = try_join(
            resource_provider.get_download_url(resource_path, client_group),
            async {
                let r5 = client.query_one(&vec_s[4], &[resource_id]).await?;
                return Ok(r5.get("downloaded"));
            },
        )
        .await?;

        return Ok(HttpResponse::Ok()
            .json(json!({ "download_url": download_url, "downloaded": downloaded })));
    }
}
