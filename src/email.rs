use lettre::transport::smtp::{
    authentication,
    client::{Certificate, Tls, TlsParametersBuilder},
    PoolConfig,
};
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use std::time::Duration;

use crate::config::GLOBAL_CONFIG;

pub type EMailPool = AsyncSmtpTransport<Tokio1Executor>;

pub fn new_email_pool() -> AsyncSmtpTransport<Tokio1Executor> {
    let config = &GLOBAL_CONFIG.email;

    let creds = authentication::Credentials::new(config.username.clone(), config.password.clone());
    let cert =
        Certificate::from_pem(&std::fs::read(config.root_cert.clone()).unwrap()[..]).unwrap();
    // Open a remote connection to gmail
    AsyncSmtpTransport::<Tokio1Executor>::relay(config.addr.as_str())
        .unwrap()
        .credentials(creds)
        .tls(Tls::Wrapper(
            TlsParametersBuilder::new(config.addr.clone())
                .add_root_certificate(cert)
                .build_rustls()
                .unwrap(),
        ))
        // see https://docs.rs/lettre/0.10.0-rc.3/lettre/transport/smtp/struct.PoolConfig.html
        .pool_config(
            PoolConfig::new()
                .min_idle(config.min_idle)
                .max_size(config.max_size)
                .idle_timeout(Duration::from_secs(config.idle_timeout)),
        )
        .build()
}
