use actix_web::{http::StatusCode, HttpResponse, HttpResponseBuilder};
use deadpool_postgres::{tokio_postgres, PoolError};
use derive_more::{Display, Error};
use serde_json::json;

#[derive(Debug, Display, Error)]
#[display(
    fmt = "{{err_code: {}, err_message: {}, internal_message: {}}}",
    err_code,
    err_message,
    internal_message
)]
pub struct ResponseError {
    pub err_code: u16,
    pub err_message: String,
    pub internal_message: String,
    pub status_code: StatusCode,
}

impl ResponseError {
    // 通用错误
    pub fn input_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 1,
            err_message: format!("输入不正确: {}", err_message),
            internal_message: internal_message.to_string(),
            status_code: StatusCode::BAD_REQUEST,
        }
    }

    pub fn permission_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 2,
            err_message: format!("没有相应权限: {}", err_message),
            internal_message: internal_message.to_string(),
            status_code: StatusCode::FORBIDDEN,
        }
    }

    pub fn access_token_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 3,
            err_message: format!("获取用户访问凭证失败: {}", err_message),
            internal_message: internal_message.to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        }
    }

    pub fn refresh_token_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 4,
            err_message: format!("获取用户刷新凭证失败: {}", err_message),
            internal_message: internal_message.to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        }
    }

    pub fn lack_coin_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 5,
            err_message: format!("无限币不足: {}", err_message),
            internal_message: internal_message.to_string(),
            status_code: StatusCode::FORBIDDEN,
        }
    }

    pub fn lack_exp_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 6,
            err_message: format!("用户等级不足: {}", err_message),
            internal_message: internal_message.to_string(),
            status_code: StatusCode::FORBIDDEN,
        }
    }

    pub fn resource_not_found_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 7,
            err_message: format!("没有找到相应资源: {}", err_message),
            internal_message: internal_message.to_string(),
            status_code: StatusCode::BAD_REQUEST,
        }
    }

    pub fn already_done_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 8,
            err_message: format!("该操作已经执行过了: {}", err_message),
            internal_message: internal_message.to_string(),
            status_code: StatusCode::BAD_REQUEST,
        }
    }

    pub fn resource_provider_unavailable_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 9,
            err_message: format!("后台文件服务器不可用: {}", err_message),
            internal_message: internal_message.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn unexpected_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 0,
            err_message: format!("未预计的错误: {}", err_message),
            internal_message: internal_message.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl actix_web::error::ResponseError for ResponseError {
    fn status_code(&self) -> StatusCode {
        self.status_code
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code())
            .json(json!({"err_code": self.err_code, "err_message": self.err_message}))
    }
}

impl From<PoolError> for ResponseError {
    fn from(error: PoolError) -> Self {
        Self {
            err_code: 501,
            err_message: "获取数据库连接失败，请稍后再试".to_string(),
            internal_message: error.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<tokio_postgres::Error> for ResponseError {
    fn from(error: tokio_postgres::Error) -> Self {
        Self {
            err_code: 502,
            err_message: "查询数据库错误，请稍后再试".to_string(),
            internal_message: error.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<jsonwebtoken::errors::Error> for ResponseError {
    fn from(error: jsonwebtoken::errors::Error) -> Self {
        Self {
            err_code: 503,
            err_message: "生成或解析jwt失败，请检查输入是否合法".to_string(),
            internal_message: error.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<lettre::address::AddressError> for ResponseError {
    fn from(error: lettre::address::AddressError) -> Self {
        Self {
            err_code: 504,
            err_message: "邮件发送失败，请检查输入后重试".to_string(),
            internal_message: error.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<lettre::error::Error> for ResponseError {
    fn from(error: lettre::error::Error) -> Self {
        Self {
            err_code: 505,
            err_message: "邮件发送失败，请检查输入后重试".to_string(),
            internal_message: error.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<lettre::transport::smtp::Error> for ResponseError {
    fn from(error: lettre::transport::smtp::Error) -> Self {
        Self {
            err_code: 506,
            err_message: "邮件发送失败，请检查输入后重试".to_string(),
            internal_message: error.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<reqwest::Error> for ResponseError {
    fn from(error: reqwest::Error) -> Self {
        Self {
            err_code: 507,
            err_message: "后台发送请求失败，请稍后重试".to_string(),
            internal_message: format!("reqwest未处理错误: {}", error),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
