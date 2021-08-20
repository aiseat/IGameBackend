use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::resource_provider::ClientGroup;

lazy_static! {
    pub static ref GLOBAL_CONFIG: Config = Config::new_from_file("config.toml");
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub app: AppConfig,
    pub jwt: JWTConfig,
    pub email: EmailConfig,
    pub pgsql: SQLConfig,
    pub msgraph: Vec<MSGraphConfig>,
    #[serde(skip)]
    file_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub mode: String,
    pub addr: String,
    pub thread: usize,
    pub log_level: String,
    pub log_format: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailConfig {
    pub addr: String,
    pub username: String,
    pub password: String,
    pub sender: String,
    pub root_cert: String,
    pub idle_timeout: u64,
    pub min_idle: u32,
    pub max_size: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JWTConfig {
    pub token_secret: String,
    pub access_token_expire: u64,
    pub refresh_token_expire: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SQLConfig {
    pub mode: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database_name: String,
    pub application_name: String,
    pub ssl: bool,
    pub root_cert: String,
    pub connect_timeout: u64,
    pub keepalives_idle: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MSGraphConfig {
    pub id: usize,
    pub connect_timeout: u64,
    pub whole_timeout: u64,
    pub pool_idle_timeout: u64,
    pub group: ClientGroup,
    pub region: String,
    pub client_id: String,
    pub client_secret: String,
    pub drive_url: String,
    pub redirect_url: String,
    pub refresh_token: String,
}

impl Config {
    pub fn new_from_file(file_path: &str) -> Self {
        let mut config: Self =
            toml::from_str(std::fs::read_to_string(file_path).unwrap().as_str()).unwrap();
        config.file_path = file_path.to_string();
        config
    }

    pub async fn write_to_file(&self) {
        let config_string = match toml::to_string_pretty(self) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("序列化为配置文件失败: {}", e);
                return;
            }
        };
        match tokio::fs::write(self.file_path.clone(), config_string).await {
            Ok(_) => tracing::debug!("配置文件写入成功"),
            Err(e) => tracing::error!("配置文件写入失败: {}", e),
        };
    }
}
