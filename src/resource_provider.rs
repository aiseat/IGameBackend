use derive_more::Display;
use futures::future::join_all;
use rand::{thread_rng, Rng};
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
}

impl ResourceProviderShare {
    pub async fn new() -> Self {
        Self {
            provider: Arc::new(RwLock::new(ResourceProvider::new().await)),
        }
    }

    pub async fn get_download_url(
        &self,
        resource_path: &str,
        client_group: &ClientGroup,
    ) -> Result<String, ResponseError> {
        let mut result: Result<String, ResponseError>;
        let mut client_index: usize;
        loop {
            {
                (result, client_index) = self
                    .provider
                    .read()
                    .await
                    .get_download_url(&resource_path, &client_group)
                    .await?;
            }
            match result {
                Ok(v) => {
                    return Ok(v);
                }
                Err(e) => {
                    // 判断是否为资源不存在错误
                    if e.err_code == 9 {
                        return Err(e);
                    }
                }
            }
            {
                //锁操作，谨慎处理
                self.provider.write().await.pause_client(client_index);
            }
        }
    }

    pub async fn write_to_config_file(&self) {
        self.provider.read().await.write_to_config_file().await;
    }

    //锁操作，谨慎处理
    pub async fn refresh_client_token(&self) {
        {
            let mut need_refresh_client = Vec::new();
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

pub struct ResourceProvider {
    clients: Vec<MSGraphClient>,
    client_info_manager: ClientInfoManager,
}

impl ResourceProvider {
    pub async fn new() -> Self {
        let mut clients = Vec::new();
        let mut client_info_manager = ClientInfoManager::new();
        let msgraph_config = GLOBAL_CONFIG.msgraph.clone();
        for c in msgraph_config.iter() {
            client_info_manager.add(&c.group);
            let client = MSGraphClient::new(c.clone());
            clients.push(client);
        }
        let client_init_vec = clients.iter_mut().map(|c| c.init()).collect::<Vec<_>>();
        join_all(client_init_vec).await;
        Self {
            clients,
            client_info_manager,
        }
    }

    pub async fn get_download_url(
        &self,
        resource_path: &str,
        client_group: &ClientGroup,
    ) -> Result<(Result<String, ResponseError>, usize), ResponseError> {
        let find = self.client_info_manager.get_available_index(client_group);
        if let Some(available_client_index) = find {
            let result = self.clients[available_client_index]
                .get_download_url(resource_path)
                .await;
            Ok((result, available_client_index))
        } else {
            Err(ResponseError::new_internal_error(
                &format!("所有在{}组内的提供者均不可用", client_group),
                Some("服务暂不可用，请稍后再试"),
            ))
        }
    }

    pub async fn write_to_config_file(&self) {
        let mut msgraph_configs: Vec<MSGraphConfig> = Vec::new();
        for client in self.clients.iter() {
            msgraph_configs.push(client.config.clone())
        }
        let mut global_config = GLOBAL_CONFIG.clone();
        global_config.msgraph = msgraph_configs;
        global_config.write_to_file().await;
    }

    pub fn pause_client(&mut self, client_index: usize) {
        self.client_info_manager
            .pause(client_index, Duration::from_secs(3 * 60));
    }

    pub async fn refresh_client_token(&mut self, need_refresh: &Vec<usize>) -> Vec<usize> {
        let mut need_refresh_client = Vec::new();
        let mut need_refresh_index = Vec::new();
        for (index, client) in self.clients.iter_mut().enumerate() {
            if need_refresh.len() == 0 || need_refresh.contains(&index) {
                need_refresh_client.push(client.refresh_token());
                need_refresh_index.push(index);
            }
        }
        let result_vec = join_all(need_refresh_client).await;
        let mut need_return_index = Vec::new();
        for (index, result) in result_vec.iter().enumerate() {
            if result.is_err() {
                need_return_index.push(need_refresh_index[index]);
            }
        }
        need_return_index
    }
}

struct ClientInfoManager {
    groups: HashMap<ClientGroup, Vec<usize>>,
    available_times: Vec<SystemTime>,
}

impl ClientInfoManager {
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
            available_times: Vec::new(),
        }
    }

    pub fn add(&mut self, group: &ClientGroup) -> usize {
        let index = self.available_times.len();
        if let Some(v) = self.groups.get_mut(group) {
            v.push(index);
        } else {
            self.groups.insert(group.clone(), vec![index]);
        }
        self.available_times.push(SystemTime::now());
        index
    }

    pub fn get_available_index(&self, client_group: &ClientGroup) -> Option<usize> {
        let matched_index_vec = self.groups.get(client_group)?;
        let mut available_index_vec = Vec::new();

        for client_index in matched_index_vec {
            if self.available_times[*client_index] < SystemTime::now() {
                available_index_vec.push(client_index);
            }
        }

        // 随机获取一个满足要求的client
        match available_index_vec.len() {
            0 => None,
            1 => Some(*available_index_vec[0]),
            _ => {
                let mut rng = thread_rng();
                let i = rng.gen_range(0..available_index_vec.len());
                Some(*available_index_vec[i])
            }
        }
    }

    pub fn pause(&mut self, index: usize, duration: Duration) {
        tracing::info!("[提供者{}]被暂停{:?}", index, duration);
        self.available_times[index] = SystemTime::now() + duration;
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
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(15))
            .pool_idle_timeout(Duration::from_secs(600))
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
        let mut response: Result<reqwest::Response, ResponseError> = Err(
            ResponseError::new_internal_error("尝试发送get_download_url请求失败", None),
        );
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
                return Err(ResponseError::new_file_not_found_error(
                    &format!(
                        "[提供者{}]没有找到文件, 文件路径：{}, 发送get_download_url的响应状态码：400, 内容: {}",
                        self.config.id,
                        resource_path,
                        response.text().await.unwrap(),
                    ),
                    None,
                ));
            } else {
                return Err(ResponseError::new_internal_error(
                    &format!(
                        "[提供者{}]发送get_download_url的响应状态码不正确: {}, 内容: {}",
                        self.config.id,
                        response.status().as_str(),
                        response.text().await.unwrap(),
                    ),
                    Some("后端文件服务器响应失败，请稍后重试"),
                ));
            }
        };

        let response_de = response.json::<Response>().await.map_err(|e| {
            tracing::error!("[提供者{}]尝试反序列化文本失败: {}", self.config.id, e);
            e
        })?;

        tracing::info!(
            "[提供者{}]获取download_url成功, 资源路径: {}",
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
            return Err(ResponseError::new_internal_error(
                &format!(
                    "[提供者{}]获取refresh_token响应状态码不正确: {}, 内容: {}",
                    self.config.id,
                    response.status().as_str(),
                    response.text().await.unwrap(),
                ),
                None,
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
