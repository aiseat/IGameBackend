mod app;
mod app_subscribe;
mod article;
mod email;
mod notice;
mod resource;
mod tag;
mod user;

pub fn register(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(app::get_app);
    cfg.service((
        app_subscribe::get_app_subscribe_status,
        app_subscribe::post_app_subscribe,
        app_subscribe::post_app_unsubscribe,
    ));
    cfg.service((
        article::get_article_covers,
        article::get_article_amount,
        article::get_article,
    ));
    cfg.service((email::post_send_verify_email, email::post_send_email));
    cfg.service((notice::get_notices, notice::get_notice, notice::post_notice));
    cfg.service((
        resource::get_brief_resources,
        resource::get_resource,
        resource::get_resource_url,
    ));
    cfg.service(tag::get_tags);
    cfg.service((
        user::get_user,
        user::get_myself,
        user::post_user_login,
        user::post_user_register,
        user::post_user_new_token,
        user::post_user_reset_password,
        user::post_user,
        user::post_user_daily_bonus,
    ));
}
