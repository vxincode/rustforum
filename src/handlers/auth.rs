// 认证处理器：包含登录页面展示、登录验证、登出、注册页面展示、注册处理
// 支持登录限流（防暴力破解）、邀请码注册、邮箱验证、登录日志记录、Cookie 会话管理

use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
    Form,
};

use crate::config::AppState;
use crate::middleware::auth::MaybeUser;
use crate::models::user::RegisterForm;
use crate::templates::*;

// 从 settings 表中读取指定配置项的值
pub async fn get_setting(pool: &sqlx::SqlitePool, key: &str) -> Option<String> {
    sqlx::query_as::<_, (String,)>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|v| v.0)
}

// 登录页面：如果已登录则重定向到首页，否则渲染登录表单
pub async fn login_page(MaybeUser(user): MaybeUser) -> impl IntoResponse {
    if user.is_some() {
        return Redirect::to("/").into_response();
    }
    Html(render_login()).into_response()
}

// 登录处理：验证用户名密码，包含 IP 限流、登录日志记录、会话创建
pub async fn login(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Form(form): Form<crate::models::user::LoginForm>,
) -> impl IntoResponse {
    let pool = &state.pool;

    // 登录限流检查：同一 IP 在 5 分钟内最多允许一定次数的登录尝试
    let ip = crate::middleware::rate_limit::extract_ip_from_headers(&headers);
    if let Err(count) = crate::middleware::rate_limit::check_login_rate(&ip) {
        tracing::warn!("Login rate limited for IP: {} ({} attempts)", ip, count);
        return Html(render_error("登录尝试过于频繁，请 5 分钟后再试")).into_response();
    }

    // 根据用户名查找用户（仅查询状态正常的用户 status=1）
    let user: Option<crate::models::user::User> = sqlx::query_as(
        "SELECT * FROM users WHERE username = ? AND status = 1"
    )
    .bind(&form.username)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let user = match user {
        Some(u) => u,
        None => {
            // 用户不存在时，记录失败的登录尝试并写入日志
            crate::middleware::rate_limit::record_failed_login(&ip);
            let ua = headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
            let _ = sqlx::query(
                "INSERT INTO login_logs (user_id, username, ip, user_agent, action, success) VALUES (0, ?, ?, ?, 'login', 0)"
            ).bind(&form.username).bind(&ip).bind(&ua).execute(pool).await;
            return Html(render_error("用户名或密码错误")).into_response();
        }
    };

    // 使用 bcrypt 验证密码
    let valid = bcrypt::verify(&form.password, &user.password_hash).unwrap_or(false);
    if !valid {
        // 密码错误时，记录失败尝试和日志
        crate::middleware::rate_limit::record_failed_login(&ip);
        let ua = headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        let _ = sqlx::query(
            "INSERT INTO login_logs (user_id, username, ip, user_agent, action, success) VALUES (?, ?, ?, ?, 'login', 0)"
        ).bind(user.id).bind(&user.username).bind(&ip).bind(&ua).execute(pool).await;
        return Html(render_error("用户名或密码错误")).into_response();
    }

    // 登录成功：清除该 IP 的限流计数
    crate::middleware::rate_limit::clear_login_attempts(&ip);

    // 更新用户的最后登录 IP、时间和 User-Agent
    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let _ = sqlx::query(
        "UPDATE users SET last_login_ip = ?, last_login_at = datetime('now'), last_login_ua = ?, updated_at = updated_at WHERE id = ?"
    )
    .bind(&ip)
    .bind(&ua)
    .bind(user.id)
    .execute(pool)
    .await;

    // 记录成功的登录日志
    let _ = sqlx::query(
        "INSERT INTO login_logs (user_id, username, ip, user_agent, action, success) VALUES (?, ?, ?, ?, 'login', 1)"
    )
    .bind(user.id)
    .bind(&user.username)
    .bind(&ip)
    .bind(&ua)
    .execute(pool)
    .await;

    // 创建会话：生成 UUID 作为 session_id，有效期 7 天
    let token = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sessions (id, user_id, expires_at) VALUES (?, ?, datetime('now', '+7 days'))")
        .bind(&token)
        .bind(user.id)
        .execute(pool)
        .await
        .ok();

    // 设置 Cookie 并重定向到首页
    let mut resp = Redirect::to("/").into_response();
    let cookie = format!("session_id={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800", token);
    if let Ok(val) = cookie.parse() {
        resp.headers_mut().insert(axum::http::header::SET_COOKIE, val);
    }
    resp
}

// 登出处理：从数据库删除会话记录，清除浏览器 Cookie
pub async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    // 从请求的 Cookie 中解析 session_id 并从数据库中删除对应会话
    if let Some(cookie_header) = headers.get("cookie") {
        if let Ok(cookies) = cookie_header.to_str() {
            if let Some(token) = cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix("session_id=").map(|s| s.to_string())
            }) {
                sqlx::query("DELETE FROM sessions WHERE id = ?")
                    .bind(&token)
                    .execute(&state.pool)
                    .await
                    .ok();
            }
        }
    }
    // 设置 Cookie 过期（Max-Age=0）并重定向到登录页
    let mut resp = Redirect::to("/auth/login").into_response();
    let cookie = "session_id=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";
    if let Ok(val) = cookie.parse() {
        resp.headers_mut().insert(axum::http::header::SET_COOKIE, val);
    }
    resp
}

// 注册页面：检查是否允许注册及是否需要邀请码，渲染注册表单
pub async fn register_page(MaybeUser(user): MaybeUser, State(state): State<AppState>) -> impl IntoResponse {
    if user.is_some() {
        return Redirect::to("/").into_response();
    }
    let allow_register = get_setting(&state.pool, "allow_register").await.unwrap_or_else(|| "1".to_string()) == "1";
    let invite_required = get_setting(&state.pool, "invite_required").await.unwrap_or_else(|| "0".to_string()) == "1";
    Html(render_register(allow_register, invite_required)).into_response()
}

// 注册处理：验证输入、检查邀请码、检查重复用户名/邮箱、创建用户
// 如果邮箱验证开启，创建用户时 status=0（待验证），发送验证邮件
pub async fn register(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Form(form): Form<RegisterForm>,
) -> impl IntoResponse {
    let pool = &state.pool;

    let invite_required = get_setting(pool, "invite_required").await.unwrap_or_else(|| "0".to_string()) == "1";
    let email_verify_enabled = get_setting(pool, "email_verify_enabled").await.unwrap_or_else(|| "0".to_string()) == "1";

    // 如果系统要求邀请码，验证邀请码是否有效（未过期且未达最大使用次数）
    if invite_required {
        let code = form.invite_code.as_deref().unwrap_or("").trim().to_string();
        if code.is_empty() {
            return Html(render_error("请输入邀请码")).into_response();
        }
        let valid: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM invite_codes WHERE code = ? AND used_count < max_uses AND (expires_at IS NULL OR expires_at > datetime('now'))"
        )
        .bind(&code)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        if valid.is_none() {
            return Html(render_error("邀请码无效或已过期")).into_response();
        }
        // 将邀请码使用次数加 1
        sqlx::query("UPDATE invite_codes SET used_count = used_count + 1 WHERE code = ?")
            .bind(&code)
            .execute(pool).await.ok();
    }

    // 基本输入验证：用户名非空，密码至少 6 位
    if form.username.trim().is_empty() || form.password.len() < 6 {
        return Html(render_error("用户名不能为空且密码不少于6位")).into_response();
    }
    // 验证两次输入的密码是否一致
    if form.password != form.password_confirm {
        return Html(render_error("两次密码不一致")).into_response();
    }

    // 检查用户名或邮箱是否已被注册
    let exists: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM users WHERE username = ? OR email = ?"
    )
    .bind(&form.username)
    .bind(&form.email)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    if exists.is_some() {
        return Html(render_error("用户名或邮箱已存在")).into_response();
    }

    // 使用 bcrypt 加密密码后创建新用户
    let hash = match bcrypt::hash(&form.password, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => return Html(render_error("系统错误，请重试")).into_response(),
    };

    // 如果开启邮箱验证，用户状态为 0（待验证）；否则直接激活（status=1）
    let initial_status = if email_verify_enabled { 0 } else { 1 };

    let result = sqlx::query(
        "INSERT INTO users (username, email, password_hash, group_id, status, last_login_ip, last_login_ua) VALUES (?, ?, ?, 3, ?, ?, ?)"
    )
    .bind(&form.username)
    .bind(&form.email)
    .bind(&hash)
    .bind(initial_status)
    .bind(&crate::middleware::rate_limit::extract_ip_from_headers(&headers))
    .bind(headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or(""))
    .execute(pool)
    .await;

    match result {
        Ok(r) => {
            if email_verify_enabled {
                // 获取新用户 ID
                let user_id = r.last_insert_rowid();
                // 生成验证令牌
                let token = uuid::Uuid::new_v4().to_string().replace('-', "");
                let expires_hours = get_setting(pool, "email_verify_expire_hours").await.unwrap_or_else(|| "24".to_string());
                let hours: i64 = expires_hours.parse().unwrap_or(24);
                sqlx::query(
                    "INSERT INTO email_verifications (user_id, token, email, expires_at) VALUES (?, ?, ?, datetime('now', '+' || ? || ' hours'))"
                )
                .bind(user_id)
                .bind(&token)
                .bind(&form.email)
                .bind(hours)
                .execute(pool)
                .await
                .ok();

                // 发送验证邮件（忽略错误，用户可以重新发送）
                let verify_url = format!("{}/auth/verify?token={}",
                    get_setting(pool, "site_url").await.unwrap_or_else(|| "http://localhost:3000".to_string()),
                    token
                );
                let site_name = get_setting(pool, "site_name").await.unwrap_or_else(|| "开发者社区".to_string());
                let subject = format!("{} - 邮箱验证", site_name);
                let body = format!(
                    "<h2>欢迎注册 {}</h2>\
                     <p>请点击以下链接完成邮箱验证：</p>\
                     <p><a href=\"{}\" style=\"display:inline-block;background:#000;color:#fff;padding:10px 24px;border-radius:8px;text-decoration:none;\">验证邮箱</a></p>\
                     <p style=\"color:#999;font-size:12px;margin-top:20px;\">如果按钮无法点击，请复制以下链接到浏览器打开：<br>{}</p>\
                     <p style=\"color:#999;font-size:12px;\">此链接 {} 小时内有效。</p>",
                    html_escape(&site_name), verify_url, verify_url, hours,
                );
                let _ = crate::email::send_email(pool, &form.email, &subject, &body).await;

                Html(render_message_page("验证邮件已发送", &format!(
                    "我们已向 <strong>{}</strong> 发送了一封验证邮件，请查收并点击验证链接完成注册。<br><br>\
                     <a href=\"/auth/login\" class=\"text-black font-medium hover:underline\">前往登录</a>",
                    html_escape(&form.email)
                ))).into_response()
            } else {
                Redirect::to("/auth/login").into_response()
            }
        }
        Err(_) => Html(render_error("注册失败，请重试")).into_response(),
    }
}

// 邮箱验证页面：通过 token 激活用户账号
pub async fn verify_email(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let pool = &state.pool;
    let token = match params.get("token") {
        Some(t) if !t.is_empty() => t.clone(),
        _ => return Html(render_error("无效的验证链接")).into_response(),
    };

    // 查找有效的验证记录
    let ver: Option<(i64, i64, String)> = sqlx::query_as(
        "SELECT id, user_id, email FROM email_verifications WHERE token = ? AND expires_at > datetime('now')"
    )
    .bind(&token)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match ver {
        Some((_ver_id, user_id, email)) => {
            // 激活用户（如果是注册验证）
            let activated = sqlx::query("UPDATE users SET status = 1 WHERE id = ? AND status = 0")
                .bind(user_id)
                .execute(pool)
                .await
                .ok();

            // If user was already active (email change), update the email
            if activated.is_some() {
                sqlx::query("UPDATE users SET email = ? WHERE id = ? AND status != 0")
                    .bind(&email)
                    .bind(user_id)
                    .execute(pool)
                    .await
                    .ok();
            }

            // 删除已使用的验证令牌
            sqlx::query("DELETE FROM email_verifications WHERE token = ?")
                .bind(&token)
                .execute(pool)
                .await
                .ok();

            // Check if user is now active to show appropriate message
            let user_status: Option<(i64,)> = sqlx::query_as("SELECT status FROM users WHERE id = ?")
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten();

            if user_status.map(|s| s.0).unwrap_or(0) == 0 {
                // Still not activated — this was a registration verify that somehow failed
                Html(render_error("验证失败，请联系管理员")).into_response()
            } else if activated.is_some() && activated.as_ref().unwrap().rows_affected() > 0 {
                // New registration verification
                Html(render_message_page("邮箱验证成功",
                    "你的邮箱已验证成功，现在可以 <a href=\"/auth/login\" class=\"text-black font-medium hover:underline\">登录</a> 了！"
                )).into_response()
            } else {
                // Email change verification
                Html(render_message_page("邮箱验证成功",
                    &format!("你的邮箱已变更为 <strong>{}</strong>。<br><br>\
                     <a href=\"/profile\" class=\"text-black font-medium hover:underline\">返回个人中心</a>",
                    html_escape(&email))
                )).into_response()
            }
        }
        None => Html(render_error("验证链接无效或已过期，请重新注册或联系管理员")).into_response(),
    }
}

// 重发验证邮件：已登录用户点击重发，如果有待验证的记录则重发，否则提示
pub async fn resend_verify(
    MaybeUser(user): MaybeUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user = match user {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };
    let pool = &state.pool;

    // Find pending verification
    let pending: Option<(String, String)> = sqlx::query_as(
        "SELECT email, token FROM email_verifications WHERE user_id = ? AND expires_at > datetime('now') ORDER BY id DESC LIMIT 1"
    )
    .bind(user.id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match pending {
        Some((email, _old_token)) => {
            // Generate new token
            let token = uuid::Uuid::new_v4().to_string().replace('-', "");
            let expires_hours = get_setting(pool, "email_verify_expire_hours").await.unwrap_or_else(|| "24".to_string());
            let hours: i64 = expires_hours.parse().unwrap_or(24);

            // Delete old and insert new
            sqlx::query("DELETE FROM email_verifications WHERE user_id = ?")
                .bind(user.id)
                .execute(pool).await.ok();
            sqlx::query(
                "INSERT INTO email_verifications (user_id, token, email, expires_at) VALUES (?, ?, ?, datetime('now', '+' || ? || ' hours'))"
            )
            .bind(user.id)
            .bind(&token)
            .bind(&email)
            .bind(hours)
            .execute(pool).await.ok();

            // Send email
            let verify_url = format!("{}/auth/verify?token={}",
                get_setting(pool, "site_url").await.unwrap_or_else(|| "http://localhost:3000".to_string()),
                token
            );
            let site_name = get_setting(pool, "site_name").await.unwrap_or_else(|| "开发者社区".to_string());
            let subject = format!("{} - 邮箱验证", site_name);
            let body = format!(
                "<h2>{}, 你好</h2>\
                 <p>请点击以下链接完成邮箱验证：</p>\
                 <p><a href=\"{}\" style=\"display:inline-block;background:#000;color:#fff;padding:10px 24px;border-radius:8px;text-decoration:none;\">验证邮箱</a></p>\
                 <p style=\"color:#999;font-size:12px;margin-top:20px;\">如果按钮无法点击，请复制以下链接到浏览器打开：<br>{}</p>\
                 <p style=\"color:#999;font-size:12px;\">此链接 {} 小时内有效。</p>",
                html_escape(&user.username), verify_url, verify_url, hours,
            );
            let _ = crate::email::send_email(pool, &email, &subject, &body).await;

            Html(render_message_page("验证邮件已发送", &format!(
                "我们已向 <strong>{}</strong> 重新发送了一封验证邮件。<br><br>\
                 <a href=\"/profile\" class=\"text-black font-medium hover:underline\">返回个人中心</a>",
                html_escape(&email)
            ))).into_response()
        }
        None => Html(render_message_page("无需验证",
            "你的邮箱已经验证过了，或者没有待验证的记录。<br><br>\
             <a href=\"/profile\" class=\"text-black font-medium hover:underline\">返回个人中心</a>"
        )).into_response(),
    }
}
