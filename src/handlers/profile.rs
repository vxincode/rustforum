// 用户资料处理器：包含个人资料查看/编辑、我的帖子列表、查看他人资料、修改密码
// 支持自定义签名、头衔、头衔颜色等个性化设置

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use bcrypt::verify;
use serde::Deserialize;

use crate::config::AppState;
use crate::middleware::auth::{AuthUser, MaybeUser};
use crate::models::post::Post;
use crate::models::thread::ThreadList;
use crate::models::user::User;
use crate::templates::*;

// =====================================================================
// 个人资料页面：需登录，展示用户信息和最近发帖/回复记录
// =====================================================================

pub async fn profile_page(
    AuthUser(user): AuthUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let pool = &state.pool;

    // 获取用户最近发布的 10 个帖子
    let recent_threads: Vec<ThreadList> = sqlx::query_as(
        "SELECT t.*, u.username, u.avatar FROM threads t LEFT JOIN users u ON t.user_id = u.id WHERE t.user_id = ? ORDER BY t.last_post_at DESC LIMIT 10"
    )
    .bind(user.id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 获取用户最近的 10 条回复（排除首帖）
    let recent_posts: Vec<Post> = sqlx::query_as(
        "SELECT p.*, u.username, u.avatar, u.group_id, u.signature, u.custom_title, u.epithet, u.epithet_color FROM posts p LEFT JOIN users u ON p.user_id = u.id WHERE p.user_id = ? AND p.is_first = 0 ORDER BY p.created_at DESC LIMIT 10"
    )
    .bind(user.id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Html(render_profile(&user, &recent_threads, &recent_posts)).into_response()
}

// =====================================================================
// 我的帖子列表：需登录，分页展示当前用户发布的所有帖子
// =====================================================================

// 分页查询参数
#[derive(Deserialize)]
pub struct PageQuery {
    pub page: Option<i64>,
}

pub async fn my_threads(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Query(q): Query<PageQuery>,
) -> impl IntoResponse {
    let pool = &state.pool;
    let per_page = state.config.threads_per_page;
    let page = q.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let threads: Vec<ThreadList> = sqlx::query_as(
        "SELECT t.*, u.username, u.avatar FROM threads t LEFT JOIN users u ON t.user_id = u.id WHERE t.user_id = ? ORDER BY t.last_post_at DESC LIMIT ? OFFSET ?"
    )
    .bind(user.id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM threads WHERE user_id = ?")
        .bind(user.id)
        .fetch_one(pool)
        .await
        .unwrap_or((0,));

    let total_pages = ((total.0 as f64) / (per_page as f64)).ceil() as i64;

    Html(render_my_threads(&threads, page, total_pages, &user)).into_response()
}

// =====================================================================
// 查看他人资料页面：支持未登录访问，展示目标用户信息和最近活动
// =====================================================================

pub async fn user_profile(
    MaybeUser(current_user): MaybeUser,
    state: State<AppState>,
    Path(user_id): Path<i64>,
) -> impl IntoResponse {
    let pool = &state.pool;

    let target_user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ? AND status = 1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    let target_user = match target_user {
        Some(u) => u,
        None => return Html(render_error("用户不存在")).into_response(),
    };

    // 获取目标用户最近的 10 个帖子
    let recent_threads: Vec<ThreadList> = sqlx::query_as(
        "SELECT t.*, u.username, u.avatar FROM threads t LEFT JOIN users u ON t.user_id = u.id WHERE t.user_id = ? ORDER BY t.last_post_at DESC LIMIT 10"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 获取目标用户最近的 10 条回复
    let recent_posts: Vec<Post> = sqlx::query_as(
        "SELECT p.*, u.username, u.avatar, u.group_id, u.signature, u.custom_title, u.epithet, u.epithet_color FROM posts p LEFT JOIN users u ON p.user_id = u.id WHERE p.user_id = ? AND p.is_first = 0 ORDER BY p.created_at DESC LIMIT 10"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Html(render_user_profile(&target_user, &recent_threads, &recent_posts, current_user.as_ref())).into_response()
}

// =====================================================================
// 资料编辑页面：需登录，展示当前用户的编辑表单
// =====================================================================

pub async fn profile_edit_page(
    AuthUser(user): AuthUser,
    state: State<AppState>,
) -> impl IntoResponse {
    // Check if user has a pending email verification
    let pending: Option<(String,)> = sqlx::query_as(
        "SELECT email FROM email_verifications WHERE user_id = ? AND expires_at > datetime('now')"
    )
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let email_unverified = pending.is_some();
    Html(render_profile_edit(&user, email_unverified)).into_response()
}

// =====================================================================
// 保存资料编辑：需登录，验证邮箱唯一性后更新用户信息
// =====================================================================

// 资料编辑表单数据
#[derive(Deserialize)]
pub struct ProfileEditForm {
    pub email: String,
    pub signature: String,
    pub custom_title: Option<String>,
    pub epithet: Option<String>,
    pub epithet_color: Option<String>,
}

pub async fn profile_edit(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Form(form): Form<ProfileEditForm>,
) -> impl IntoResponse {
    let pool = &state.pool;
    let email = form.email.trim();
    let signature = form.signature.trim();
    let custom_title = form.custom_title.map(|t| t.trim().to_string()).unwrap_or_default();
    let epithet = form.epithet.map(|t| t.trim().to_string()).unwrap_or_default();
    let epithet_color = form.epithet_color.map(|c| c.trim().to_string()).unwrap_or_default();

    if email.is_empty() {
        return Html(render_error("邮箱不能为空")).into_response();
    }

    // 检查邮箱是否已被其他用户使用（排除自己）
    let existing: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM users WHERE email = ? AND id != ?"
    )
    .bind(email)
    .bind(user.id)
    .fetch_one(pool)
    .await
    .unwrap_or((0,));

    if existing.0 > 0 {
        return Html(render_error("该邮箱已被使用")).into_response();
    }

    // Check if email changed and email verification is enabled
    let email_changed = email != user.email;
    let email_verify_enabled = crate::handlers::auth::get_setting(pool, "email_verification_enabled").await.unwrap_or_else(|| "0".to_string()) == "1";

    if email_changed && email_verify_enabled {
        // Delete old pending verifications for this user
        sqlx::query("DELETE FROM email_verifications WHERE user_id = ?")
            .bind(user.id)
            .execute(pool)
            .await
            .ok();

        // Generate 6-digit verification code
        use rand::Rng;
        let code: String = (0..6).map(|_| rand::thread_rng().gen_range(0..10).to_string()).collect();
        let expires_hours = crate::handlers::auth::get_setting(pool, "email_verify_expire_hours").await.unwrap_or_else(|| "24".to_string());
        let hours: i64 = expires_hours.parse().unwrap_or(24);
        sqlx::query(
            "INSERT INTO email_verifications (user_id, token, email, expires_at) VALUES (?, ?, ?, datetime('now', '+' || ? || ' hours'))"
        )
        .bind(user.id)
        .bind(&code)
        .bind(email)
        .bind(hours)
        .execute(pool)
        .await
        .ok();

        // Save other profile fields first (keep old email)
        sqlx::query("UPDATE users SET signature = ?, custom_title = ?, epithet = ?, epithet_color = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(signature)
            .bind(&custom_title)
            .bind(&epithet)
            .bind(&epithet_color)
            .bind(user.id)
            .execute(pool)
            .await
            .ok();

        // Send verification code email
        let site_name = crate::handlers::auth::get_setting(pool, "site_name").await.unwrap_or_else(|| "开发者社区".to_string());
        let subject = format!("{} - 邮箱变更验证码", site_name);
        let body = format!(
            "<h2>{}, 你好</h2>\
             <p>你正在修改绑定的邮箱地址，验证码为：</p>\
             <p style=\"font-size:32px;font-weight:bold;letter-spacing:8px;margin:20px 0;color:#000;\">{}</p>\
             <p style=\"color:#999;font-size:12px;\">验证码 {} 小时内有效，如非本人操作请忽略此邮件。</p>",
            html_escape(&user.username), code, hours,
        );
        let _ = crate::email::send_email(pool, email, &subject, &body).await;

        // Redirect to verification code page (so JS fetch sees redirect as success)
        Redirect::to(&format!("/profile/verify-email?email={}", urlencoding(&email))).into_response()
    } else {
        // Normal save (email unchanged or verification not required)
        sqlx::query("UPDATE users SET email = ?, signature = ?, custom_title = ?, epithet = ?, epithet_color = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(email)
            .bind(signature)
            .bind(&custom_title)
            .bind(&epithet)
            .bind(&epithet_color)
            .bind(user.id)
            .execute(pool)
            .await
            .ok();

        Redirect::to("/profile").into_response()
    }
}

// 邮箱验证码输入页面
#[derive(Deserialize)]
pub struct VerifyEmailQuery {
    pub email: Option<String>,
}

pub async fn verify_email_page(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Query(query): Query<VerifyEmailQuery>,
) -> impl IntoResponse {
    let email = query.email.unwrap_or_default();
    // Check if there's a pending verification
    let pending: Option<(String,)> = sqlx::query_as(
        "SELECT email FROM email_verifications WHERE user_id = ? AND expires_at > datetime('now')"
    )
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let display_email = if !email.is_empty() { email } else { pending.map(|p| p.0).unwrap_or_default() };

    if display_email.is_empty() {
        return Redirect::to("/profile/edit").into_response();
    }

    Html(render_verify_email_code(&display_email)).into_response()
}

// 验证验证码并更新邮箱
#[derive(Deserialize)]
pub struct VerifyCodeForm {
    pub code: String,
}

pub async fn verify_email_code(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Form(form): Form<VerifyCodeForm>,
) -> impl IntoResponse {
    let pool = &state.pool;
    let code = form.code.trim();

    if code.is_empty() {
        return Html(render_error("请输入验证码")).into_response();
    }

    // Find matching verification record
    let ver: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, email FROM email_verifications WHERE user_id = ? AND token = ? AND expires_at > datetime('now')"
    )
    .bind(user.id)
    .bind(code)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match ver {
        Some((_id, new_email)) => {
            // Update email
            sqlx::query("UPDATE users SET email = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(&new_email)
                .bind(user.id)
                .execute(pool)
                .await
                .ok();
            // Delete used verification
            sqlx::query("DELETE FROM email_verifications WHERE user_id = ?")
                .bind(user.id)
                .execute(pool)
                .await
                .ok();

            Html(render_message_page("邮箱变更成功",
                &format!("你的邮箱已成功变更为 <strong>{}</strong>。<br><br>\
                 <a href=\"/profile\" class=\"text-black font-medium hover:underline\">返回个人中心</a>",
                html_escape(&new_email))
            )).into_response()
        }
        None => {
            Html(render_error("验证码错误或已过期，请返回重试")).into_response()
        }
    }
}

// =====================================================================
// 修改密码：需登录，验证旧密码后更新新密码
// =====================================================================

// 修改密码表单数据
#[derive(Deserialize)]
pub struct ChangePasswordForm {
    pub old_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

pub async fn change_password(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Form(form): Form<ChangePasswordForm>,
) -> impl IntoResponse {
    if form.old_password.trim().is_empty() || form.new_password.trim().is_empty() {
        return Html(render_error("密码不能为空")).into_response();
    }

    if form.new_password.len() < 6 {
        return Html(render_error("新密码至少6位")).into_response();
    }

    if form.new_password != form.confirm_password {
        return Html(render_error("两次输入的新密码不一致")).into_response();
    }

    // 使用 bcrypt 验证旧密码是否正确
    let valid = verify(&form.old_password, &user.password_hash).unwrap_or(false);
    if !valid {
        return Html(render_error("旧密码不正确")).into_response();
    }

    // 对新密码进行 bcrypt 加密后更新到数据库
    let new_hash = bcrypt::hash(&form.new_password, bcrypt::DEFAULT_COST).unwrap_or_default();
    sqlx::query("UPDATE users SET password_hash = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(&new_hash)
        .bind(user.id)
        .execute(&state.pool)
        .await
        .ok();

    Redirect::to("/profile").into_response()
}
