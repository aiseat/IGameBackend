use derive_more::Display;
use futures::future::join_all;
use reqwest::{header, ClientBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::{sync::RwLock, time::interval};

use crate::config::{MSGraphConfig, GLOBAL_CONFIG};
use crate::error::ResponseError;

#[derive(Clone)]
pub struct ResourceProviderShare {
    provider: Arc<RwLock<ResourceProvider>>,
    cache_manager: Arc<RwLock<CacheManager>>,
}

impl ResourceProviderShare {
    pub async fn new() -> Self {
        Self {
            provider: Arc::new(RwLock::new(ResourceProvider::new().await)),
            cache_manager: Arc::new(RwLock::new(CacheManager::new())),
        }
    }

    pub async fn get_download_url(
        &self,
        resource_path: &str,
        client_group: &ClientGroup,
        client_ids: Vec<&str>,
    ) -> Result<String, ResponseError> {
        // 查询缓存
        let result = self
            .cache_manager
            .read()
            .await
            .get(client_group, resource_path);
        if let Some(v) = result {
            tracing::info!(
                "[命中缓存]获取下载链接成功, 资源路径: {}, 提供者组: {}",
                resource_path,
                client_group
            );
            return Ok(v);
        }

        let mut default_err = ResponseError::resource_provider_unavailable_err(
            "服务暂不可用，获取下载链接失败，请稍后重试",
            "get_download_url方法的默认错误",
        );
        for client_id in client_ids {
            let result = self
                .provider
                .read()
                .await
                .get_download_url(resource_path, client_id)
                .await;
            match result {
                Ok(url) => {
                    //设置缓存
                    self.cache_manager
                        .write()
                        .await
                        .set(client_group, resource_path, url.as_str());

                    return Ok(url);
                }
                Err(e) => {
                    if !e.is_resource_not_found_err() {
                        // 暂停该提供者
                        // 锁操作，谨慎处理
                        self.provider.write().await.pause_client(client_id);
                        default_err = e;
                    }
                }
            }
        }
        return Err(default_err);
    }

    pub async fn write_to_config_file(&self) {
        self.provider.read().await.write_to_config_file().await;
    }

    //锁操作，谨慎处理
    pub async fn refresh_client_token(&self) {
        {
            let mut need_refresh_client = Vec::new();
            // 每分钟重试一次，最多十次
            let mut interval = interval(Duration::from_secs(1 * 60));
            interval.tick().await;
            for i in 0..10 {
                need_refresh_client = self
                    .provider
                    .write()
                    .await
                    .refresh_client_token(&need_refresh_client)
                    .await;
                if need_refresh_client.len() == 0 {
                    break;
                }
                tracing::error!(
                    "{}个提供者获取refresh_token失败, 将于一分钟后重试, 重试次数: {}/10",
                    need_refresh_client.len(),
                    i + 1
                );
                interval.tick().await;
            }
        }
    }
}

pub struct CacheManager {
    cache: HashMap<(ClientGroup, String), (String, SystemTime)>,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn get(&self, group: &ClientGroup, path: &str) -> Option<String> {
        let value = self.cache.get(&(*group, path.to_string()))?;
        // 缓存将近两小时
        if value.1 + Duration::from_secs(7000) > SystemTime::now() {
            return Some(value.0.clone());
        }
        None
    }

    pub fn set(&mut self, group: &ClientGroup, path: &str, value: &str) {
        self.cache.insert(
            (*group, path.to_string()),
            (value.to_string(), SystemTime::now()),
        );
    }
}

pub struct ResourceProvider {
    clients: HashMap<String, MSGraphClient>,
    client_info_manager: ClientInfoManager,
}

impl ResourceProvider {
    pub async fn new() -> Self {
        let mut clients = HashMap::new();
        let mut client_info_manager = ClientInfoManager::new();
        let msgraph_config = GLOBAL_CONFIG.msgraph.clone();
        for c in msgraph_config.iter() {
            clients.insert(c.id.clone(), MSGraphClient::new(c.clone()));
            client_info_manager.add(&c.id);
        }

        //并行初始化
        let mut client_init_vec: Vec<_> = Vec::new();
        for client in clients.values_mut() {
            client_init_vec.push(client.init());
        }
        join_all(client_init_vec).await;

        Self {
            clients,
            client_info_manager,
        }
    }

    pub async fn get_download_url(
        &self,
        resource_path: &str,
        client_id: &str,
    ) -> Result<String, ResponseError> {
        if self.client_info_manager.is_available(client_id) {
            let result = self.clients[client_id]
                .get_download_url(resource_path)
                .await;
            result
        } else {
            Err(ResponseError::resource_provider_unavailable_err(
                "服务暂不可用，获取下载链接失败，请稍后重试",
                &format!("[提供者{}]处于暂停中", client_id),
            ))
        }
    }

    pub async fn write_to_config_file(&self) {
        let mut msgraph_configs: Vec<MSGraphConfig> = Vec::new();
        for client in self.clients.values() {
            msgraph_configs.push(client.config.clone())
        }
        let mut global_config = GLOBAL_CONFIG.clone();
        global_config.msgraph = msgraph_configs;
        global_config.write_to_file().await;
    }

    pub fn pause_client(&mut self, client_id: &str) {
        self.client_info_manager
            .pause(client_id, Duration::from_secs(3 * 60));
    }

    pub async fn refresh_client_token(&mut self, need_refresh: &Vec<String>) -> Vec<String> {
        let mut need_refresh_clients = Vec::new();
        let mut need_refresh_client_ids = Vec::new();
        for client in self.clients.values_mut() {
            if need_refresh.len() == 0 || need_refresh.contains(&client.config.id) {
                need_refresh_client_ids.push(client.config.id.clone());
                need_refresh_clients.push(client.refresh_token());
            }
        }
        let result_vec = join_all(need_refresh_clients).await;
        let mut need_return_client_ids = Vec::new();
        for (index, result) in result_vec.iter().enumerate() {
            if result.is_err() {
                tracing::error!(
                    "[提供者{}]获取refresh_token失败",
                    need_refresh_client_ids[index].clone()
                );
                need_return_client_ids.push(need_refresh_client_ids[index].clone());
            }
        }
        need_return_client_ids
    }
}

struct ClientInfoManager {
    pause_times: HashMap<String, SystemTime>,
}

impl ClientInfoManager {
    pub fn new() -> Self {
        Self {
            pause_times: HashMap::new(),
        }
    }

    pub fn add(&mut self, client_id: &str) -> usize {
        self.pause_times
            .insert(client_id.to_string(), SystemTime::now());
        self.pause_times.len()
    }

    pub fn is_available(&self, client_id: &str) -> bool {
        if let Some(pause_time) = self.pause_times.get(client_id) {
            return *pause_time < SystemTime::now();
        }
        false
    }

    pub fn pause(&mut self, client_id: &str, duration: Duration) {
        self.pause_times
            .insert(client_id.to_string(), SystemTime::now() + duration);
        tracing::info!("[提供者{}]被暂停{:?}", client_id, duration);
    }
}

pub struct MSGraphClient {
    config: MSGraphConfig,
    graph_api: String,
    oauth_api: String,
    request_client: reqwest::Client,
    drive_id: String,
    access_token: String,
}

impl MSGraphClient {
    pub fn new(config: MSGraphConfig) -> Self {
        let (graph_api, oauth_api) = match config.region.as_str() {
            "china" => (
                "https://microsoftgraph.chinacloudapi.cn/v1.0/".to_string(),
                "https://login.chinacloudapi.cn/common/oauth2/v2.0/".to_string(),
            ),
            // "global"
            _ => (
                "https://graph.microsoft.com/v1.0/".to_string(),
                "https://login.microsoftonline.com/common/oauth2/v2.0/".to_string(),
            ),
        };

        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Accept",
            header::HeaderValue::from_static("application/json;odata.metadata=none"),
        );
        let request_client = ClientBuilder::new()
            .user_agent("ISV|rclone.org|rclone/v1.55.1")
            .default_headers(headers)
            .no_proxy()
            .connect_timeout(Duration::from_secs(config.connect_timeout))
            .timeout(Duration::from_secs(config.whole_timeout))
            .pool_idle_timeout(Duration::from_secs(config.pool_idle_timeout))
            .build()
            .unwrap();

        Self {
            config,
            graph_api,
            oauth_api,
            request_client,
            drive_id: Default::default(),
            access_token: Default::default(),
        }
    }

    async fn init(&mut self) {
        // 重试三次发送请求
        for i in 0..3 {
            let result = self.refresh_token().await;

            match result {
                Ok(_) => break,
                Err(e) => {
                    tracing::error!(
                        "[提供者{}]尝试获取refresh_token失败: {}, 重试次数：{}/3",
                        self.config.id,
                        e,
                        i + 1
                    );
                }
            }
        }

        #[derive(Deserialize)]
        struct Response {
            id: String,
        }

        // 重试三次发送请求
        let mut response: Option<reqwest::Response> = None;
        for i in 0..3 {
            let result = self
                .request_client
                .get(format!("{}{}", self.graph_api, self.config.drive_url))
                .query(&[("$select", "id")])
                .header("Authorization", format!("Bearer {}", self.access_token))
                .send()
                .await;

            match result {
                Ok(v) => {
                    response = Some(v);
                    break;
                }
                Err(e) => {
                    tracing::error!(
                        "[提供者{}]尝试获取drive_id失败: {}, 重试次数：{}/3",
                        self.config.id,
                        e,
                        i + 1
                    );
                }
            }
        }
        let response_de = response.unwrap().json::<Response>().await.unwrap();

        tracing::info!("[提供者{}]获取drive_id成功", self.config.id);

        self.drive_id = response_de.id;
    }

    async fn get_download_url(&self, resource_path: &str) -> Result<String, ResponseError> {
        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "@microsoft.graph.downloadUrl")]
            download_url: String,
        }

        // 重试三次发送请求
        let mut response: Result<reqwest::Response, ResponseError> =
            Err(ResponseError::resource_provider_unavailable_err(
                "获取下载链接失败，请稍后重试",
                &format!("[提供者{}]尝试发送get_download_url请求失败", self.config.id),
            ));
        for i in 0..3 {
            let result = self
                .request_client
                .get(format!(
                    "{}drives/{}/root:{}",
                    self.graph_api, self.drive_id, resource_path
                ))
                .query(&[("$select", "content.downloadUrl")])
                .header("Authorization", format!("Bearer {}", self.access_token))
                .send()
                .await;

            match result {
                Ok(v) => {
                    response = Ok(v);
                    break;
                }
                Err(e) => {
                    tracing::error!(
                        "[提供者{}]尝试发送get_download_url请求失败: {}, 重试次数：{}/3",
                        self.config.id,
                        e,
                        i + 1
                    );
                    response = Err(e.into())
                }
            }
        }
        let response = response?;

        if !response.status().is_success() {
            if response.status() == StatusCode::BAD_REQUEST {
                tracing::debug!(
                    "[提供者{}]没有找到文件, 文件路径：{}, 发送get_download_url的响应状态码：400, 内容: {}",
                    self.config.id,
                    resource_path,
                    response.text().await.unwrap()
                );
                return Err(ResponseError::resource_not_found_err(
                    "资源不存在",
                    "错误状态码400",
                ));
            } else {
                return Err(ResponseError::resource_provider_unavailable_err(
                    "获取下载链接失败，请稍后重试",
                    &format!(
                        "[提供者{}]发送get_download_url的响应状态码不正确: {}, 内容: {}",
                        self.config.id,
                        response.status().as_str(),
                        response.text().await.unwrap(),
                    ),
                ));
            }
        };

        let response_de = response.json::<Response>().await.map_err(|e| {
            tracing::error!("[提供者{}]尝试反序列化文本失败: {}", self.config.id, e);
            e
        })?;

        tracing::info!(
            "[提供者{}]获取下载链接成功, 资源路径: {}",
            self.config.id,
            resource_path
        );
        Ok(response_de.download_url)
    }

    async fn refresh_token(&mut self) -> Result<(), ResponseError> {
        #[derive(Deserialize)]
        struct Response {
            access_token: String,
            refresh_token: String,
        }

        let response = self
            .request_client
            .post(format!("{}token", self.oauth_api))
            .form(&[
                ("client_id", self.config.client_id.clone()),
                ("grant_type", "refresh_token".to_string()),
                ("refresh_token", self.config.refresh_token.clone()),
                ("redirect_uri", self.config.redirect_url.clone()),
                ("client_secret", self.config.client_secret.clone()),
            ])
            .send()
            .await
            .map_err(|e| {
                tracing::error!(
                    "[提供者{}]尝试发送refresh_token请求失败: {}",
                    self.config.id,
                    e
                );
                e
            })?;

        if !response.status().is_success() {
            return Err(ResponseError::resource_provider_unavailable_err(
                "文件服务器获取refresh_token失败",
                &format!(
                    "[提供者{}]获取refresh_token响应状态码不正确， 状态码：{}, 内容: {}",
                    self.config.id,
                    response.status().as_str(),
                    response.text().await.unwrap(),
                ),
            ));
        };

        let response_de = response.json::<Response>().await.map_err(|e| {
            tracing::error!("[提供者{}]尝试反序列化文本失败: {}", self.config.id, e);
            e
        })?;

        self.access_token = response_de.access_token;
        self.config.refresh_token = response_de.refresh_token;

        tracing::info!("[提供者{}]获取refresh_token成功", self.config.id);
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientGroup {
    #[serde(rename = "normal")]
    #[display(fmt = "normal")]
    Normal,
    #[serde(rename = "fast")]
    #[display(fmt = "fast")]
    Fast,
}

impl ClientGroup {
    pub fn to_index(&self) -> usize {
        match self {
            Self::Normal => 0,
            Self::Fast => 1,
        }
    }
}
