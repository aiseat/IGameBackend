mod article;
mod email;
mod game_article;
mod mod_article;
mod notification;
mod resource;
mod tag;
mod token;
mod user;

pub fn register(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service((email::post_send_verify_email, email::post_send_email));
    cfg.service((
        user::get_user,
        user::get_myself,
        user::post_user,
        user::post_daily_bonus,
    ));
    cfg.service((
        token::post_login,
        token::post_register,
        token::post_new_token,
        token::post_reset_password,
    ));
    cfg.service((
        article::get_article_subscribe_stataus,
        article::post_article_subscribe,
        article::post_article_unsubscribe,
    ));
    cfg.service((
        game_article::get_game_article_covers,
        game_article::get_game_article_size,
        game_article::get_game_article,
    ));
    cfg.service((
        mod_article::get_mod_article_covers,
        mod_article::get_mod_article_size,
        mod_article::get_mod_article,
    ));
    cfg.service((resource::get_resource, resource::get_resource_url));
    cfg.service(tag::get_tags);
    cfg.service((
        notification::get_notification,
        notification::post_notification,
    ));
}
