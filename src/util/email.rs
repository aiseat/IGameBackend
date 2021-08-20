use lettre::{
    message::{
        header::{Header, HeaderName},
        SinglePart,
    },
    AsyncTransport, Message,
};

use crate::email::EMailPool;
use crate::error::ResponseError;
use rand::Rng;

pub fn generate_verify_code() -> String {
    const CHARSET: &[u8] = b"0123456789";
    const PASSWORD_LEN: usize = 4;
    let mut rng = rand::thread_rng();

    let verify_code: String = (0..PASSWORD_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    verify_code
}

#[derive(Clone)]
struct ListUnsubscribeHeader {}

impl Header for ListUnsubscribeHeader {
    fn name() -> HeaderName {
        HeaderName::new_from_ascii_str("List-Unsubscribe")
    }

    fn parse(_: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {})
    }

    fn display(&self) -> String {
        "One-Click".to_string()
    }
}

pub async fn send_email(
    mail_pool: &EMailPool,
    from: &str,
    to: &str,
    subject: &str,
    html: &str,
) -> Result<(), ResponseError> {
    let email = Message::builder()
        .from(from.parse()?)
        .to(to.parse()?)
        .subject(subject)
        .header(ListUnsubscribeHeader {})
        .singlepart(SinglePart::html(html.to_string()))?;
    mail_pool.send(email).await?;
    Ok(())
}
