use actix_web::{http::StatusCode, HttpResponse, HttpResponseBuilder};
use deadpool_postgres::{tokio_postgres, PoolError};
use derive_more::{Display, Error};
use serde::Serialize;

#[derive(Debug, Display, Error, Serialize)]
#[display(
    fmt = "{{err_code: {}, err_type:{}, err_message: {}, extra_field: {:?}, internal_message: {}}}",
    err_code,
    err_type,
    err_message,
    extra_field,
    internal_message
)]
pub struct ResponseError {
    pub err_code: u16,
    pub err_type: String,
    pub err_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_field: Option<ExtraField>,
    #[serde(skip_serializing)]
    pub internal_message: String,
    #[serde(skip_serializing)]
    pub status_code: StatusCode,
}

#[derive(Debug, Display, Error, Serialize)]
#[display(fmt = "{{need_exp: {:?}, need_coin: {:?}}}", need_exp, need_coin)]
pub struct ExtraField {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub need_exp: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub need_coin: Option<i32>,
}

impl ResponseError {
    // 通用错误
    pub fn input_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 1,
            err_type: "输入不正确".to_string(),
            err_message: err_message.to_string(),
            extra_field: None,
            internal_message: internal_message.to_string(),
            status_code: StatusCode::BAD_REQUEST,
        }
    }

    pub fn permission_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 2,
            err_type: "没有对应权限".to_string(),
            err_message: err_message.to_string(),
            extra_field: None,
            internal_message: internal_message.to_string(),
            status_code: StatusCode::FORBIDDEN,
        }
    }

    pub fn access_token_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 3,
            err_type: "获取用户访问凭证失败".to_string(),
            err_message: err_message.to_string(),
            extra_field: None,
            internal_message: internal_message.to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        }
    }

    pub fn refresh_token_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 4,
            err_type: "获取用户刷新凭证失败".to_string(),
            err_message: err_message.to_string(),
            extra_field: None,
            internal_message: internal_message.to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        }
    }

    pub fn lack_coin_err(err_message: &str, need_coin: i32, internal_message: &str) -> Self {
        Self {
            err_code: 5,
            err_type: "无限币不足".to_string(),
            err_message: err_message.to_string(),
            extra_field: Some(ExtraField {
                need_exp: None,
                need_coin: Some(need_coin),
            }),
            internal_message: internal_message.to_string(),
            status_code: StatusCode::FORBIDDEN,
        }
    }

    pub fn lack_exp_err(err_message: &str, need_exp: i32, internal_message: &str) -> Self {
        Self {
            err_code: 6,
            err_type: "用户等级不足".to_string(),
            err_message: err_message.to_string(),
            extra_field: Some(ExtraField {
                need_exp: Some(need_exp),
                need_coin: None,
            }),
            internal_message: internal_message.to_string(),
            status_code: StatusCode::FORBIDDEN,
        }
    }

    pub fn resource_not_found_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 7,
            err_type: "没有找到相应资源".to_string(),
            err_message: err_message.to_string(),
            extra_field: None,
            internal_message: internal_message.to_string(),
            status_code: StatusCode::BAD_REQUEST,
        }
    }

    pub fn is_resource_not_found_err(&self) -> bool {
        return self.err_code == 7;
    }

    pub fn already_done_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 8,
            err_type: "该操作已经执行过了".to_string(),
            err_message: err_message.to_string(),
            extra_field: None,
            internal_message: internal_message.to_string(),
            status_code: StatusCode::BAD_REQUEST,
        }
    }

    pub fn resource_provider_unavailable_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 9,
            err_type: "后台文件服务器不可用".to_string(),
            err_message: err_message.to_string(),
            extra_field: None,
            internal_message: internal_message.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn unexpected_err(err_message: &str, internal_message: &str) -> Self {
        Self {
            err_code: 0,
            err_type: "未预期的错误".to_string(),
            err_message: err_message.to_string(),
            extra_field: None,
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
        HttpResponseBuilder::new(self.status_code()).json(self)
    }
}

impl From<PoolError> for ResponseError {
    fn from(error: PoolError) -> Self {
        Self {
            err_code: 501,
            err_type: "数据库连接池错误".to_string(),
            err_message: "获取数据库连接失败，请稍后再试".to_string(),
            extra_field: None,
            internal_message: error.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<tokio_postgres::Error> for ResponseError {
    fn from(error: tokio_postgres::Error) -> Self {
        Self {
            err_code: 502,
            err_type: "数据库查询错误".to_string(),
            err_message: "数据库发生未预期的错误，请稍后再试".to_string(),
            extra_field: None,
            internal_message: error.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<jsonwebtoken::errors::Error> for ResponseError {
    fn from(error: jsonwebtoken::errors::Error) -> Self {
        Self {
            err_code: 503,
            err_type: "jwt未预期的错误".to_string(),
            err_message: "生成或解析jwt失败，请检查输入是否合法".to_string(),
            extra_field: None,
            internal_message: error.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<lettre::address::AddressError> for ResponseError {
    fn from(error: lettre::address::AddressError) -> Self {
        Self {
            err_code: 504,
            err_type: "邮件地址错误".to_string(),
            err_message: "邮件发送失败，请检查输入后重试".to_string(),
            extra_field: None,
            internal_message: error.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<lettre::error::Error> for ResponseError {
    fn from(error: lettre::error::Error) -> Self {
        Self {
            err_code: 505,
            err_type: "邮件系统错误".to_string(),
            err_message: "邮件发送失败，请检查输入后重试".to_string(),
            extra_field: None,
            internal_message: error.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<lettre::transport::smtp::Error> for ResponseError {
    fn from(error: lettre::transport::smtp::Error) -> Self {
        Self {
            err_code: 506,
            err_type: "邮件smtp错误".to_string(),
            err_message: "邮件发送失败，请检查输入后重试".to_string(),
            extra_field: None,
            internal_message: error.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<reqwest::Error> for ResponseError {
    fn from(error: reqwest::Error) -> Self {
        Self {
            err_code: 507,
            err_type: "reqwest未预期的错误".to_string(),
            err_message: "后台发送请求失败，请稍后重试".to_string(),
            extra_field: None,
            internal_message: error.to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
