use actix_web::{http::header, HttpRequest};

use crate::error::ResponseError;
use crate::util::jwt::parse_access_token;

pub fn get_access_token(req: &HttpRequest) -> Option<&str> {
    let header_maps = req.headers();
    let value = header_maps.get(header::AUTHORIZATION)?.to_str().ok()?;
    if value.len() < 7 {
        return None;
    }
    Some(&value[7..])
}

pub fn get_user_id(req: &HttpRequest) -> Result<i32, ResponseError> {
    let access_token = get_access_token(&req).ok_or(ResponseError::new_input_error(
        "无法从头部获取access_token",
        Some("无法获取用户凭证"),
    ))?;
    let user_id = parse_access_token(access_token)?.user_id;
    Ok(user_id)
}
