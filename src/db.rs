use deadpool_postgres::{
    Config, ManagerConfig, Pool, PoolConfig, RecyclingMethod, Runtime, SslMode,
};
// use openssl::ssl::{SslConnector, SslMethod};
use rustls::{ClientConfig, RootCertStore};
use rustls_pemfile::certs;
use std::time::Duration;
use tokio_postgres::NoTls;
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::config::GLOBAL_CONFIG;

pub type Type = tokio_postgres::types::Type;

pub fn new_db_pool() -> Pool {
    let config = &GLOBAL_CONFIG.pgsql;

    // see https://docs.rs/tokio-postgres/0.7.2/tokio_postgres/config/struct.Config.html
    let mut cfg = Config::new();
    match config.mode.as_str() {
        "uds" => {
            cfg.host = Some(config.host.clone());
        }
        _ => {
            cfg.host = Some(config.host.clone());
            cfg.port = Some(config.port.clone());
        }
    }
    cfg.dbname = Some(config.database_name.clone());
    cfg.user = Some(config.user.clone());
    cfg.password = Some(config.password.clone());
    cfg.application_name = Some(config.application_name.clone());
    cfg.keepalives_idle = Some(Duration::from_secs(config.keepalives_idle));
    cfg.connect_timeout = Some(Duration::from_secs(config.connect_timeout));
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    cfg.pool = Some(PoolConfig::new(100));
    match config.ssl {
        true => {
            cfg.ssl_mode = Some(SslMode::Require);
            let mut root_store = RootCertStore::empty();
            let mut root_pem =
                std::io::Cursor::new(std::fs::read(config.root_cert.clone()).unwrap());
            root_store.add_parsable_certificates(&certs(&mut root_pem).unwrap());
            let client_config = ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(root_store)
                .with_no_client_auth();
            let tls_connector = MakeRustlsConnect::new(client_config);
            let pool = cfg
                .create_pool(Some(Runtime::Tokio1), tls_connector)
                .unwrap();
            pool
        }
        false => {
            cfg.ssl_mode = Some(SslMode::Disable);
            let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls).unwrap();
            pool
        }
    }
}
