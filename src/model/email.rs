use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SendVerifyEmailInput {
    pub email_addr: String,
    pub email_type: VerifyEmailType,
}

#[derive(Debug, Serialize)]
pub struct PostSendVerifyEmailOutput {
    pub email_id: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub enum VerifyEmailType {
    #[serde(rename = "user_register")]
    UserRegister,
    #[serde(rename = "password_reset")]
    PasswordReset,
}

impl VerifyEmailType {
    pub fn to_int2(&self) -> i16 {
        match self {
            Self::UserRegister => 1,
            Self::PasswordReset => 2,
        }
    }

    pub fn to_subject(&self) -> String {
        match self {
            Self::UserRegister => "注册验证邮件",
            Self::PasswordReset => "重置密码验证邮件",
        }
        .to_string()
    }

    pub fn to_html(&self, verify_code: &str) -> String {
        match self {
            Self::UserRegister => format!(
                r##"<!doctypehtml><html lang=zh-CN xmlns=http://www.w3.org/1999/xhtml xmlns:o=urn:schemas-microsoft-com:office:office xmlns:v=urn:schemas-microsoft-com:vml><meta charset=utf-8><meta content="width=device-width"name=viewport><meta content="IE=edge"http-equiv=X-UA-Compatible><meta name=x-apple-disable-message-reformatting><meta content="telephone=no,address=no,email=no,date=no,url=no"name=format-detection><meta content=light name=color-scheme><meta content=light name=supported-color-schemes><title>IGame注册邮件</title><!--[if gte mso 9]><xml><o:officedocumentsettings><o:allowpng><o:pixelsperinch>96</o:pixelsperinch></o:officedocumentsettings></xml><![endif]--><!--[if mso]><style>*{{font-family:sans-serif!important}}</style><![endif]--><!--[if !mso]><!--><!--<![endif]--><style>:root{{color-scheme:light;supported-color-schemes:light}}body,html{{margin:0 auto!important;padding:0!important;height:100%!important;width:100%!important}}*{{-ms-text-size-adjust:100%;-webkit-text-size-adjust:100%}}div[style*="margin: 16px 0"]{{margin:0!important}}#MessageViewBody,#MessageWebViewDiv{{width:100%!important}}table,td{{mso-table-lspace:0!important;mso-table-rspace:0!important}}table{{border-spacing:0!important;border-collapse:collapse!important;table-layout:fixed!important;margin:0 auto!important}}img{{-ms-interpolation-mode:bicubic}}a{{text-decoration:none}}.aBn,.unstyle-auto-detected-links a,a[x-apple-data-detectors]{{border-bottom:0!important;cursor:default!important;color:inherit!important;text-decoration:none!important;font-size:inherit!important;font-family:inherit!important;font-weight:inherit!important;line-height:inherit!important}}.a6S{{display:none!important;opacity:.01!important}}.im{{color:inherit!important}}img.g-img+div{{display:none!important}}@media only screen and (min-device-width:320px) and (max-device-width:374px){{u~div .email-container{{min-width:320px!important}}}}@media only screen and (min-device-width:375px) and (max-device-width:413px){{u~div .email-container{{min-width:375px!important}}}}@media only screen and (min-device-width:414px){{u~div .email-container{{min-width:414px!important}}}}</style><body style=margin:0;padding:0!important;mso-line-height-rule:exactly;background-color:#fff width=100%><center aria-roledescription=email lang=en role=article style=width:100%;background-color:#fff><!--[if mso | IE]><table border=0 cellpadding=0 cellspacing=0 role=presentation width=100% style=background-color:#fff><tr><td><![endif]--><div style=max-height:0;overflow:hidden;mso-hide:all aria-hidden=true>感谢您注册「IGame」账号, 验证码：{}</div><div style=display:none;font-size:1px;line-height:1px;max-height:0;max-width:0;opacity:0;overflow:hidden;mso-hide:all>‌</div><div style="max-width:600px;margin:0 auto;background-image:url(https://cdn.jsdelivr.net/gh/OmegaLo/images@main/email_backgroud.png);background-color:#e74777"class=email-container><!--[if mso]><table border=0 cellpadding=0 cellspacing=0 role=presentation align=center style=background-image:url(https://cdn.jsdelivr.net/gh/OmegaLo/images@main/email_backgroud.png);background-color:#e74777 width=600><tr><td><![endif]--><table border=0 cellpadding=0 cellspacing=0 role=presentation align=center style=margin:auto width=100%><tr><td style=padding-top:72px;text-align:center><img alt=Logo border=0 height=120 src=https://cdn.jsdelivr.net/gh/OmegaLo/images@main/email_logo.png width=120><tr><td><table border=0 cellpadding=0 cellspacing=0 role=presentation width=100%><tr><td style="padding:48px 24px 0 24px;text-align:center;font-size:32px;color:#fff;font-weight:700"><span>感谢您注册</span><span style=padding-top:8px;display:block>「IGame」账号</span><tr><td style="padding:48px 20px 0 20px"><table border=0 cellpadding=0 cellspacing=0 role=presentation align=center style=margin:auto><tr><td style="background:#eec312;color:#000;font-size:18px;padding:8px 40px;font-weight:700"><span>验证码</span></table><tr><td style="padding:0 20px"><table border=0 cellpadding=0 cellspacing=0 role=presentation align=center style=margin:auto><tr><td style="background-color:#fff;color:#000;font-size:56px;font-weight:700;padding:8px 16px;width:260px;text-align:center;border-radius:4px"><span>{}</span></table><tr><td style=padding-top:48px;font-family:sans-serif;font-size:15px;line-height:20px;color:#fff><table border=0 cellpadding=0 cellspacing=0 role=presentation align=center style=margin:auto><tr><td><ul style="padding:0 16px 0 32px;list-style-type:disc"><li style=padding-bottom:8px class=list-item-first>该验证码2小时内有效，如果过期请重新申请验证<li style=padding-bottom:8px>每个邮箱只能成功注册一个账号<li style=padding-bottom:8px class=list-item-last>如果你并没有尝试注册「IGame」账号，请忽略该邮件</ul></table></table></table><table border=0 cellpadding=0 cellspacing=0 role=presentation align=center style=margin:auto width=100%><tr><td style="padding:48px 20px 72px 20px"><table border=0 cellpadding=0 cellspacing=0 role=presentation align=center style=margin:auto><tr><td style=color:#fff;font-size:24px;font-weight:700;padding:16px;width:160px;text-align:center;border-radius:4px;background-color:#843fa1><span>系统邮件</span></table></table><!--[if mso]><![endif]--></div><!--[if mso | IE]><![endif]--></center>"##,
                verify_code, verify_code
            ),
            Self::PasswordReset => format!(
                r##"<!doctypehtml><html lang=zh-CN xmlns=http://www.w3.org/1999/xhtml xmlns:o=urn:schemas-microsoft-com:office:office xmlns:v=urn:schemas-microsoft-com:vml><meta charset=utf-8><meta content="width=device-width"name=viewport><meta content="IE=edge"http-equiv=X-UA-Compatible><meta name=x-apple-disable-message-reformatting><meta content="telephone=no,address=no,email=no,date=no,url=no"name=format-detection><meta content=light name=color-scheme><meta content=light name=supported-color-schemes><title>IGame重置密码邮件</title><!--[if gte mso 9]><xml><o:officedocumentsettings><o:allowpng><o:pixelsperinch>96</o:pixelsperinch></o:officedocumentsettings></xml><![endif]--><!--[if mso]><style>*{{font-family:sans-serif!important}}</style><![endif]--><!--[if !mso]><!--><!--<![endif]--><style>:root{{color-scheme:light;supported-color-schemes:light}}body,html{{margin:0 auto!important;padding:0!important;height:100%!important;width:100%!important}}*{{-ms-text-size-adjust:100%;-webkit-text-size-adjust:100%}}div[style*="margin: 16px 0"]{{margin:0!important}}#MessageViewBody,#MessageWebViewDiv{{width:100%!important}}table,td{{mso-table-lspace:0!important;mso-table-rspace:0!important}}table{{border-spacing:0!important;border-collapse:collapse!important;table-layout:fixed!important;margin:0 auto!important}}img{{-ms-interpolation-mode:bicubic}}a{{text-decoration:none}}.aBn,.unstyle-auto-detected-links a,a[x-apple-data-detectors]{{border-bottom:0!important;cursor:default!important;color:inherit!important;text-decoration:none!important;font-size:inherit!important;font-family:inherit!important;font-weight:inherit!important;line-height:inherit!important}}.a6S{{display:none!important;opacity:.01!important}}.im{{color:inherit!important}}img.g-img+div{{display:none!important}}@media only screen and (min-device-width:320px) and (max-device-width:374px){{u~div .email-container{{min-width:320px!important}}}}@media only screen and (min-device-width:375px) and (max-device-width:413px){{u~div .email-container{{min-width:375px!important}}}}@media only screen and (min-device-width:414px){{u~div .email-container{{min-width:414px!important}}}}</style><body style=margin:0;padding:0!important;mso-line-height-rule:exactly;background-color:#fff width=100%><center aria-roledescription=email lang=en role=article style=width:100%;background-color:#fff><!--[if mso | IE]><table border=0 cellpadding=0 cellspacing=0 role=presentation width=100% style=background-color:#fff><tr><td><![endif]--><div style=max-height:0;overflow:hidden;mso-hide:all aria-hidden=true>您正在尝试重置「IGame」账号密码, 验证码：{}</div><div style=display:none;font-size:1px;line-height:1px;max-height:0;max-width:0;opacity:0;overflow:hidden;mso-hide:all></div><div style="max-width:600px;margin:0 auto;background-image:url(https://cdn.jsdelivr.net/gh/OmegaLo/images@main/email_backgroud.png);background-color:#e74777"class=email-container><!--[if mso]><table border=0 cellpadding=0 cellspacing=0 role=presentation align=center style=background-image:url(https://cdn.jsdelivr.net/gh/OmegaLo/images@main/email_backgroud.png);background-color:#e74777 width=600><tr><td><![endif]--><table border=0 cellpadding=0 cellspacing=0 role=presentation align=center style=margin:auto width=100%><tr><td style=padding-top:72px;text-align:center><img alt=Logo border=0 height=120 src=https://cdn.jsdelivr.net/gh/OmegaLo/images@main/email_logo.png width=120><tr><td><table border=0 cellpadding=0 cellspacing=0 role=presentation width=100%><tr><td style="padding:48px 24px 0 24px;text-align:center;font-size:32px;color:#fff;font-weight:700"><span>您正在尝试重置</span><span style=padding-top:8px;display:block>「IGame」账号密码</span><tr><td style="padding:48px 20px 0 20px"><table border=0 cellpadding=0 cellspacing=0 role=presentation align=center style=margin:auto><tr><td style="background:#eec312;color:#000;font-size:18px;padding:8px 40px;font-weight:700"><span>验证码</span></table><tr><td style="padding:0 20px"><table border=0 cellpadding=0 cellspacing=0 role=presentation align=center style=margin:auto><tr><td style="background-color:#fff;color:#000;font-size:56px;font-weight:700;padding:8px 16px;width:260px;text-align:center;border-radius:4px"><span>{}</span></table><tr><td style=padding-top:48px;font-family:sans-serif;font-size:15px;line-height:20px;color:#fff><table border=0 cellpadding=0 cellspacing=0 role=presentation align=center style=margin:auto><tr><td><ul style="padding:0 16px 0 32px;list-style-type:disc"><li class=list-item-first style=padding-bottom:8px>该验证码2小时内有效，如果过期请重新申请验证<li class=list-item-last style=padding-bottom:8px>如果你并没有尝试重置「IGame」账号密码，请忽略该邮件</ul></table></table></table><table border=0 cellpadding=0 cellspacing=0 role=presentation align=center style=margin:auto width=100%><tr><td style="padding:48px 20px 72px 20px"><table border=0 cellpadding=0 cellspacing=0 role=presentation align=center style=margin:auto><tr><td style=color:#fff;font-size:24px;font-weight:700;padding:16px;width:160px;text-align:center;border-radius:4px;background-color:#843fa1><span>系统邮件</span></table></table><!--[if mso]><![endif]--></div><!--[if mso | IE]><![endif]--></center>"##,
                verify_code, verify_code
            ),
        }
    }
}

#[derive(Deserialize)]
pub struct SendEmailInput {
    pub addr: String,
    pub subject: String,
    pub html: String,
}
