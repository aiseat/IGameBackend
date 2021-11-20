#![allow(dead_code)]
#![feature(destructuring_assignment)]

use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use std::time::Duration;
use time::macros::format_description;
use tokio::time::interval;
use tracing_subscriber::{filter::LevelFilter, fmt::time::LocalTime, EnvFilter};

use crate::config::GLOBAL_CONFIG;
use crate::resource_provider::ResourceProviderShare;
use crate::tracing_middleware::{CustomRootSpanBuilder, TracingLogger};

mod config;
mod db;
mod email;
mod error;
mod model;
mod resource_provider;
mod router;
mod tracing_middleware;
mod util;

fn main() {
    actix_web::rt::System::with_tokio_rt(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(GLOBAL_CONFIG.app.thread)
            .enable_all()
            .build()
            .unwrap()
    })
    .block_on(async_main());
}

async fn async_main() {
    // 设置日志
    tracing_log::LogTracer::init().unwrap();
    let log_level = match GLOBAL_CONFIG.app.log_level.as_str() {
        "trace" => LevelFilter::TRACE,
        "debug" => LevelFilter::DEBUG,
        "info" => LevelFilter::INFO,
        "warn" => LevelFilter::WARN,
        "error" => LevelFilter::ERROR,
        _ => LevelFilter::INFO,
    };
    let env_filter = EnvFilter::from_default_env()
        .add_directive("rustls=info".parse().unwrap())
        .add_directive("hyper=info".parse().unwrap())
        .add_directive(log_level.into());
    let (stderr, _guard) = tracing_appender::non_blocking(std::io::stderr());
    let subscriber_builder = tracing_subscriber::fmt::Subscriber::builder()
        .with_writer(stderr)
        .with_env_filter(env_filter)
        .with_timer(LocalTime::new(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]"
        )));
    match GLOBAL_CONFIG.app.log_format.as_str() {
        "pretty" => {
            tracing::subscriber::set_global_default(subscriber_builder.pretty().finish()).unwrap()
        }
        "json" => {
            tracing::subscriber::set_global_default(subscriber_builder.json().finish()).unwrap()
        }
        _ => tracing::subscriber::set_global_default(subscriber_builder.finish()).unwrap(),
    }

    run_server().await.unwrap();
}

#[cfg(target_family = "windows")]
async fn run_server() -> std::io::Result<()> {
    // 初始化数据库连接池
    let db_pool = db::new_db_pool();
    // 维持一个与数据库的连接
    {
        #[allow(unused_must_use)]
        {
            db_pool.get().await.unwrap();
        }
    }
    // 初始化邮件服务器连接池
    let email_pool = email::new_email_pool();
    // 初始化资源服务器连接池
    let resource_provider = ResourceProviderShare::new().await;
    {
        resource_provider.write_to_config_file().await;
    }
    // 初始化定时执行服务
    let resource_provider_clone = resource_provider.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(3000));
        interval.tick().await;
        loop {
            interval.tick().await;
            resource_provider_clone.refresh_client_token().await;
            resource_provider_clone.write_to_config_file().await;
        }
    });

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(email_pool.clone()))
            .app_data(web::Data::new(resource_provider.clone()))
            .wrap(middleware::Compress::default())
            .wrap(TracingLogger::<CustomRootSpanBuilder>::new())
            .configure(router::register)
            .default_service(web::route().to(|| HttpResponse::NotFound()))
    })
    .max_connection_rate(1024)
    .workers(GLOBAL_CONFIG.app.thread)
    .bind(&GLOBAL_CONFIG.app.addr)
    .expect(&format!("不能绑定到地址: {}", GLOBAL_CONFIG.app.addr))
    .run()
    .await
}

#[cfg(target_family = "unix")]
async fn run_server() -> std::io::Result<()> {
    // 初始化数据库连接池
    let db_pool = db::new_db_pool();
    // 维持一个与数据库的连接
    {
        db_pool.get().await.unwrap();
    }
    // 初始化邮件服务器连接池
    let email_pool = email::new_email_pool();
    // 初始化资源服务器连接池
    let resource_provider = ResourceProviderShare::new().await;
    {
        resource_provider.write_to_config_file().await;
    }
    // 初始化定时执行服务
    let resource_provider_clone = resource_provider.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(3000));
        interval.tick().await;
        loop {
            interval.tick().await;
            resource_provider_clone.refresh_client_token().await;
            resource_provider_clone.write_to_config_file().await;
        }
    });

    let temp_server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(email_pool.clone()))
            .app_data(web::Data::new(resource_provider.clone()))
            .wrap(middleware::Compress::default())
            .wrap(TracingLogger::<CustomRootSpanBuilder>::new())
            .configure(router::register)
            .default_service(web::route().to(|| HttpResponse::NotFound()))
    })
    .max_connection_rate(1024)
    .workers(GLOBAL_CONFIG.app.thread);
    let server;

    match GLOBAL_CONFIG.app.mode.as_str() {
        "uds" => {
            server = temp_server
                .bind_uds(&GLOBAL_CONFIG.app.addr)
                .expect(&format!("不能绑定到地址: {}", GLOBAL_CONFIG.app.addr))
        }
        _ => {
            server = temp_server
                .bind(&GLOBAL_CONFIG.app.addr)
                .expect(&format!("不能绑定到地址: {}", GLOBAL_CONFIG.app.addr))
        }
    }
    server.run().await
}
