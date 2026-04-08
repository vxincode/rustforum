// JSON API 处理器：提供帖子列表、版块列表、站点统计、搜索、登录/注册/登出、
// 当前用户状态、用户卡片（悬浮弹窗）、发帖、回复等接口
// 所有接口均返回 JSON 格式数据，供前端 AJAX 调用

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::config::AppState;
use crate::middleware::auth::{AuthUser, MaybeUser, can_view_forum_async, can_post_in_forum};
use crate::models::forum::Forum;
use crate::models::thread::ThreadList;
use crate::models::user::User;

// =====================================================================
// 帖子列表 API：支持 latest（最新）、hot（热门）、essence（精华）、按版块筛选
// =====================================================================

// 帖子列表查询参数：tab 控制排序方式，page 分页，forum 按版块筛选
#[derive(Deserialize)]
pub struct ThreadListQuery {
    pub tab: Option<String>,
    pub page: Option<i64>,
    pub forum: Option<i64>,
}

pub async fn api_threads(
    MaybeUser(user): MaybeUser,
    state: State<AppState>,
    Query(q): Query<ThreadListQuery>,
) -> Json<Value> {
    let pool = &state.pool;
    let per_page = state.config.threads_per_page;
    let page = q.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;
    let tab = q.tab.unwrap_or_else(|| "latest".to_string());

    // 获取不可见版块列表
    let all_forums: Vec<Forum> = sqlx::query_as("SELECT * FROM forums")
        .fetch_all(pool).await.unwrap_or_default();
    let hidden_forum_ids: Vec<i64> = {
        let mut ids = Vec::new();
        for f in &all_forums {
            if !can_view_forum_async(pool, f, user.as_ref()).await {
                ids.push(f.id);
            }
        }
        ids
    };
    let exclude = if hidden_forum_ids.is_empty() {
        String::new()
    } else {
        let ids: Vec<String> = hidden_forum_ids.iter().map(|id| id.to_string()).collect();
        format!(" AND t.forum_id NOT IN ({})", ids.join(","))
    };

    // 根据 tab 参数选择不同的查询策略
    let (threads, total): (Vec<ThreadList>, i64) = if tab == "hot" {
        let threads = sqlx::query_as(
            &format!("SELECT t.*, u.username, u.avatar FROM threads t LEFT JOIN users u ON t.user_id = u.id WHERE 1=1{} ORDER BY t.is_top DESC, t.view_count DESC LIMIT ? OFFSET ?", exclude)
        )
        .bind(per_page).bind(offset)
        .fetch_all(pool).await.unwrap_or_default();
        let cnt: (i64,) = sqlx::query_as(
            &format!("SELECT COUNT(*) FROM threads t WHERE 1=1{}", exclude)
        ).fetch_one(pool).await.unwrap_or((0,));
        (threads, cnt.0)
    } else if tab == "essence" {
        let threads = sqlx::query_as(
            &format!("SELECT t.*, u.username, u.avatar FROM threads t LEFT JOIN users u ON t.user_id = u.id WHERE t.is_essence = 1{} ORDER BY t.last_post_at DESC LIMIT ? OFFSET ?", exclude)
        )
        .bind(per_page).bind(offset)
        .fetch_all(pool).await.unwrap_or_default();
        let cnt: (i64,) = sqlx::query_as(
            &format!("SELECT COUNT(*) FROM threads t WHERE t.is_essence = 1{}", exclude)
        ).fetch_one(pool).await.unwrap_or((0,));
        (threads, cnt.0)
    } else if let Some(forum_id) = q.forum {
        let threads = sqlx::query_as(
            &format!("SELECT t.*, u.username, u.avatar FROM threads t LEFT JOIN users u ON t.user_id = u.id WHERE t.forum_id = ?{} ORDER BY t.is_top DESC, t.last_post_at DESC LIMIT ? OFFSET ?", exclude)
        )
        .bind(forum_id).bind(per_page).bind(offset)
        .fetch_all(pool).await.unwrap_or_default();
        let cnt: (i64,) = sqlx::query_as(
            &format!("SELECT COUNT(*) FROM threads t WHERE t.forum_id = ?{}", exclude)
        ).bind(forum_id).fetch_one(pool).await.unwrap_or((0,));
        (threads, cnt.0)
    } else {
        let threads = sqlx::query_as(
            &format!("SELECT t.*, u.username, u.avatar FROM threads t LEFT JOIN users u ON t.user_id = u.id WHERE 1=1{} ORDER BY t.is_top DESC, t.last_post_at DESC LIMIT ? OFFSET ?", exclude)
        )
        .bind(per_page).bind(offset)
        .fetch_all(pool).await.unwrap_or_default();
        let cnt: (i64,) = sqlx::query_as(
            &format!("SELECT COUNT(*) FROM threads t WHERE 1=1{}", exclude)
        ).fetch_one(pool).await.unwrap_or((0,));
        (threads, cnt.0)
    };

    let total_pages = ((total as f64) / (per_page as f64)).ceil() as i64;

    Json(json!({
        "threads": threads.iter().map(|t| json!({
            "id": t.id,
            "forum_id": t.forum_id,
            "title": t.title,
            "username": t.username,
            "avatar": t.avatar,
            "view_count": t.view_count,
            "reply_count": t.reply_count,
            "is_top": t.is_top,
            "is_closed": t.is_closed,
            "is_essence": t.is_essence,
            "created_at": t.created_at,
        })).collect::<Vec<_>>(),
        "page": page,
        "total_pages": total_pages,
        "total": total,
    }))
}

// =====================================================================
// 版块列表 API：返回所有启用的版块信息
// =====================================================================

pub async fn api_forums(MaybeUser(user): MaybeUser, state: State<AppState>) -> Json<Value> {
    // Try Redis cache first (only for guests, since user-specific visibility varies)
    if user.is_none() {
        if let Some(cached) = crate::cache::get_cached(&state.redis, "api:forums").await {
            if let Ok(val) = serde_json::from_str::<Value>(&cached) {
                return Json(val);
            }
        }
    }

    let pool = &state.pool;
    let all_forums: Vec<Forum> = sqlx::query_as(
        "SELECT * FROM forums WHERE status = 1 ORDER BY sort_order ASC, id ASC"
    )
    .fetch_all(pool).await.unwrap_or_default();

    let mut forums = Vec::new();
    for f in all_forums {
        if can_view_forum_async(pool, &f, user.as_ref()).await {
            forums.push(f);
        }
    }

    let result = json!({
        "forums": forums.iter().map(|f| json!({
            "id": f.id,
            "name": f.name,
            "description": f.description,
            "thread_count": f.thread_count,
            "post_count": f.post_count,
        })).collect::<Vec<_>>(),
    });

    // Cache for guests only, 600s
    if user.is_none() {
        crate::cache::set_cached(&state.redis, "api:forums", &result.to_string(), 600).await;
    }

    Json(result)
}

// =====================================================================
// 站点统计 API：返回用户数、帖子数、回复数、版块数、今日签到数
// =====================================================================

pub async fn api_stats(state: State<AppState>) -> Json<Value> {
    // Try Redis cache first
    if let Some(cached) = crate::cache::get_cached(&state.redis, "api:stats").await {
        if let Ok(val) = serde_json::from_str::<Value>(&cached) {
            return Json(val);
        }
    }

    let users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.pool).await.unwrap_or((0,));
    let threads: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM threads")
        .fetch_one(&state.pool).await.unwrap_or((0,));
    let posts: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE is_first = 1")
        .fetch_one(&state.pool).await.unwrap_or((0,));
    let replies: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE is_first = 0")
        .fetch_one(&state.pool).await.unwrap_or((0,));
    let forums: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM forums")
        .fetch_one(&state.pool).await.unwrap_or((0,));
    let today_checkins: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM checkins WHERE checkin_date = date('now')")
        .fetch_one(&state.pool).await.unwrap_or((0,));

    let result = json!({
        "users": users.0,
        "threads": threads.0,
        "posts": posts.0,
        "replies": replies.0,
        "forums": forums.0,
        "today_checkins": today_checkins.0,
    });

    // Cache for 300s
    crate::cache::set_cached(&state.redis, "api:stats", &result.to_string(), 300).await;

    Json(result)
}

// =====================================================================
// 搜索 API：根据关键词搜索帖子标题，返回最多 20 条结果
// =====================================================================

// 搜索查询参数
#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

pub async fn api_search(MaybeUser(user): MaybeUser, state: State<AppState>, Query(q): Query<SearchQuery>) -> Json<Value> {
    let pool = &state.pool;
    // 构建 LIKE 模糊搜索模式，转义特殊字符防止注入
    let pattern = format!("%{}%", q.q.replace('%', "\\%").replace('_', "\\_"));

    // 获取不可见版块列表
    let all_forums: Vec<Forum> = sqlx::query_as("SELECT * FROM forums")
        .fetch_all(pool).await.unwrap_or_default();
    let hidden_forum_ids: Vec<i64> = {
        let mut ids = Vec::new();
        for f in &all_forums {
            if !can_view_forum_async(pool, f, user.as_ref()).await {
                ids.push(f.id);
            }
        }
        ids
    };
    let exclude = if hidden_forum_ids.is_empty() {
        String::new()
    } else {
        let ids: Vec<String> = hidden_forum_ids.iter().map(|id| id.to_string()).collect();
        format!(" AND t.forum_id NOT IN ({})", ids.join(","))
    };

    let threads: Vec<ThreadList> = sqlx::query_as(
        &format!("SELECT t.*, u.username, u.avatar FROM threads t LEFT JOIN users u ON t.user_id = u.id WHERE t.title LIKE ?{} ORDER BY t.last_post_at DESC LIMIT 20", exclude)
    )
    .bind(&pattern)
    .fetch_all(pool).await.unwrap_or_default();

    Json(json!({
        "results": threads.iter().map(|t| json!({
            "id": t.id,
            "title": t.title,
            "username": t.username,
            "avatar": t.avatar,
            "reply_count": t.reply_count,
            "view_count": t.view_count,
            "is_top": t.is_top,
            "is_essence": t.is_essence,
        })).collect::<Vec<_>>(),
    }))
}

// =====================================================================
// API 登录/注册/登出：通过 JSON 接口进行认证，登录成功后设置 Cookie
// =====================================================================

// API 登录表单数据
#[derive(Deserialize)]
pub struct ApiLoginForm {
    pub username: String,
    pub password: String,
}

// API 注册表单数据
#[derive(Deserialize)]
pub struct ApiRegisterForm {
    pub username: String,
    pub email: String,
    pub password: String,
    #[serde(rename = "password_confirm")]
    pub password_confirm: String,
    pub invite_code: Option<String>,
}

// API 登录：验证用户名密码，支持 IP 限流，返回 JSON 结果和 Set-Cookie 头
pub async fn api_login(
    state: State<AppState>,
    headers: axum::http::HeaderMap,
    Json(form): Json<ApiLoginForm>,
) -> impl IntoResponse {
    let ip = crate::middleware::rate_limit::extract_ip_from_headers(&headers);

    // 登录限流检查
    if let Err(count) = crate::middleware::rate_limit::check_login_rate(&ip) {
        tracing::warn!("API login rate limited for IP: {} ({} attempts)", ip, count);
        return (StatusCode::TOO_MANY_REQUESTS, Json(json!({
            "ok": false, "error": "登录尝试过于频繁，请 5 分钟后再试"
        }))).into_response();
    }

    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE username = ? AND status = 1")
        .bind(&form.username)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None);

    let user = match user {
        Some(u) => u,
        None => {
            crate::middleware::rate_limit::record_failed_login(&ip);
            return (StatusCode::OK, Json(json!({"ok": false, "error": "用户名或密码错误"}))).into_response();
        }
    };

    if !bcrypt::verify(&form.password, &user.password_hash).unwrap_or(false) {
        crate::middleware::rate_limit::record_failed_login(&ip);
        return (StatusCode::OK, Json(json!({"ok": false, "error": "用户名或密码错误"}))).into_response();
    }

    crate::middleware::rate_limit::clear_login_attempts(&ip);

    let session_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sessions (id, user_id, expires_at) VALUES (?, ?, datetime('now', '+7 days'))")
        .bind(&session_id).bind(user.id)
        .execute(&state.pool).await.ok();

    (
        StatusCode::OK,
        [
            (header::SET_COOKIE, format!("session_id={}; Path=/; HttpOnly; Max-Age=604800; SameSite=Lax", session_id)),
        ],
        Json(json!({
            "ok": true,
            "user": { "id": user.id, "username": user.username, "group_id": user.group_id }
        })),
    ).into_response()
}

// API 注册：支持邀请码验证，创建新用户后返回 JSON 结果
pub async fn api_register(state: State<AppState>, Json(form): Json<ApiRegisterForm>) -> Json<Value> {
    let pool = &state.pool;

    // 如果系统要求邀请码，进行验证
    let invite_required: String = sqlx::query_as("SELECT value FROM settings WHERE key = 'invite_required'")
        .fetch_optional(pool).await.unwrap_or(None)
        .map(|(v,)| v).unwrap_or_else(|| "0".to_string());
    if invite_required == "1" {
        let code = form.invite_code.as_deref().unwrap_or("").trim().to_string();
        if code.is_empty() {
            return Json(json!({"ok": false, "error": "请输入邀请码"}));
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
            return Json(json!({"ok": false, "error": "邀请码无效或已过期"}));
        }
        sqlx::query("UPDATE invite_codes SET used_count = used_count + 1 WHERE code = ?")
            .bind(&code)
            .execute(pool).await.ok();
    }

    if form.username.trim().is_empty() || form.password.len() < 6 {
        return Json(json!({"ok": false, "error": "用户名不能为空，密码至少6位"}));
    }
    if form.password != form.password_confirm {
        return Json(json!({"ok": false, "error": "两次密码不一致"}));
    }

    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE username = ? OR email = ?")
        .bind(&form.username).bind(&form.email)
        .fetch_optional(&state.pool).await.unwrap_or(None);

    if exists.is_some() {
        return Json(json!({"ok": false, "error": "用户名或邮箱已存在"}));
    }

    let hash = match bcrypt::hash(&form.password, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => return Json(json!({"ok": false, "error": "系统错误，请重试"})),
    };
    match sqlx::query("INSERT INTO users (username, email, password_hash, group_id, status) VALUES (?, ?, ?, 3, 1)")
        .bind(&form.username).bind(&form.email).bind(&hash)
        .execute(&state.pool).await
    {
        Ok(_) => Json(json!({"ok": true})),
        Err(_) => Json(json!({"ok": false, "error": "注册失败，请重试"})),
    }
}

// =====================================================================
// API 登出：清除浏览器 Cookie
// =====================================================================

pub async fn api_logout() -> impl IntoResponse {
    (
        StatusCode::OK,
        [
            (header::SET_COOKIE, "session_id=; Path=/; HttpOnly; Max-Age=0; SameSite=Lax".to_string()),
        ],
        Json(json!({"ok": true})),
    )
}

// =====================================================================
// 当前用户状态 API：返回是否已登录及用户基本信息
// =====================================================================

pub async fn api_me(MaybeUser(user): MaybeUser) -> Json<Value> {
    match user {
        Some(u) => Json(json!({
            "ok": true,
            "logged_in": true,
            "user": { "id": u.id, "username": u.username, "group_id": u.group_id }
        })),
        None => Json(json!({"ok": true, "logged_in": false, "user": null})),
    }
}

// =====================================================================
// 用户卡片 API：根据用户 ID 返回用户信息，用于前端悬浮弹窗展示
// =====================================================================

pub async fn api_user_card(
    state: State<AppState>,
    Path(user_id): Path<i64>,
) -> Json<Value> {
    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ? AND status = 1")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();

    match user {
        Some(u) => Json(json!({
            "ok": true,
            "user": {
                "id": u.id,
                "username": u.username,
                "avatar": u.avatar,
                "group_name": u.group_name(),
                "post_count": u.post_count,
                "thread_count": u.thread_count,
                "credits": u.credits,
                "signature": u.signature,
                "join_date": u.created_at.chars().take(10).collect::<String>(),
            }
        })),
        None => Json(json!({"ok": false})),
    }
}

// =====================================================================
// API 发帖：通过 JSON 创建新帖子，需登录
// =====================================================================

// API 发帖表单数据
#[derive(Deserialize)]
pub struct ApiNewThreadForm {
    pub title: String,
    pub content: String,
}

// API 发帖处理：创建帖子记录、首帖、更新版块和用户计数器
pub async fn api_new_thread(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(forum_id): Path<i64>,
    Json(form): Json<ApiNewThreadForm>,
) -> Json<Value> {
    // 检查禁言状态
    if let Some(msg) = crate::middleware::auth::get_mute_status(&state.pool, user.id).await {
        return Json(json!({"ok": false, "error": msg}));
    }

    if form.title.trim().is_empty() || form.content.trim().is_empty() {
        return Json(json!({"ok": false, "error": "标题和内容不能为空"}));
    }

    // 检查版块存在性和发帖权限
    let forum: Option<Forum> = sqlx::query_as("SELECT * FROM forums WHERE id = ? AND status = 1")
        .bind(forum_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();
    match forum {
        Some(f) if !can_post_in_forum(&state.pool, &f, &user).await => {
            return Json(json!({"ok": false, "error": "你没有权限在此版块发帖"}));
        }
        None => return Json(json!({"ok": false, "error": "版块不存在"})),
        _ => {}
    }

    let result = sqlx::query_as::<_, (i64,)>(
        "INSERT INTO threads (forum_id, user_id, title, last_post_at, last_post_user) VALUES (?, ?, ?, datetime('now'), ?) RETURNING id"
    )
    .bind(forum_id).bind(user.id).bind(&form.title).bind(&user.username)
    .fetch_one(&state.pool).await;

    let thread_id = match result {
        Ok((id,)) => id,
        Err(_) => return Json(json!({"ok": false, "error": "发帖失败"})),
    };

    sqlx::query("INSERT INTO posts (thread_id, forum_id, user_id, content, floor, is_first) VALUES (?, ?, ?, ?, 1, 1)")
        .bind(thread_id).bind(forum_id).bind(user.id).bind(&form.content)
        .execute(&state.pool).await.ok();

    sqlx::query("UPDATE forums SET thread_count = thread_count + 1, post_count = post_count + 1, last_thread_id = ?, last_post_at = datetime('now'), last_post_user = ? WHERE id = ?")
        .bind(thread_id).bind(&user.username).bind(forum_id)
        .execute(&state.pool).await.ok();

    sqlx::query("UPDATE users SET thread_count = thread_count + 1, post_count = post_count + 1 WHERE id = ?")
        .bind(user.id).execute(&state.pool).await.ok();

    Json(json!({"ok": true, "thread_id": thread_id}))
}

// =====================================================================
// API 回复：通过 JSON 回复帖子，需登录，自动发送通知给帖子作者
// =====================================================================

// API 回复表单数据
#[derive(Deserialize)]
pub struct ApiReplyForm {
    pub content: String,
}

// API 回复处理：验证帖子状态、创建回复、更新计数、发送通知
pub async fn api_reply(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(thread_id): Path<i64>,
    Json(form): Json<ApiReplyForm>,
) -> Json<Value> {
    // 检查禁言状态
    if let Some(msg) = crate::middleware::auth::get_mute_status(&state.pool, user.id).await {
        return Json(json!({"ok": false, "error": msg}));
    }

    if form.content.trim().is_empty() {
        return Json(json!({"ok": false, "error": "回复内容不能为空"}));
    }

    let thread = sqlx::query_as::<_, crate::models::thread::Thread>(
        "SELECT t.*, u.username, f.name as forum_name FROM threads t LEFT JOIN users u ON t.user_id = u.id LEFT JOIN forums f ON t.forum_id = f.id WHERE t.id = ?"
    ).bind(thread_id).fetch_optional(&state.pool).await;

    let thread = match thread {
        Ok(Some(t)) if t.is_closed == 0 => t,
        Ok(Some(_)) => return Json(json!({"ok": false, "error": "帖子已关闭"})),
        Ok(None) => return Json(json!({"ok": false, "error": "帖子不存在"})),
        Err(e) => {
            tracing::error!("Reply query error: {:?}", e);
            return Json(json!({"ok": false, "error": "数据库错误"}));
        }
    };

    // 检查版块发帖权限
    let forum: Option<Forum> = sqlx::query_as("SELECT * FROM forums WHERE id = ?")
        .bind(thread.forum_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();
    if let Some(f) = &forum {
        if !can_post_in_forum(&state.pool, f, &user).await {
            return Json(json!({"ok": false, "error": "你没有权限在此版块发帖"}));
        }
    }

    let floor: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE thread_id = ?")
        .bind(thread_id).fetch_one(&state.pool).await.unwrap_or((0,));
    let next_floor = floor.0 + 1;

    sqlx::query("INSERT INTO posts (thread_id, forum_id, user_id, content, floor, is_first) VALUES (?, ?, ?, ?, ?, 0)")
        .bind(thread_id).bind(thread.forum_id).bind(user.id).bind(&form.content).bind(next_floor)
        .execute(&state.pool).await.ok();

    sqlx::query("UPDATE threads SET reply_count = reply_count + 1, last_post_at = datetime('now'), last_post_user = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(&user.username).bind(thread_id).execute(&state.pool).await.ok();

    sqlx::query("UPDATE forums SET post_count = post_count + 1, last_thread_id = ?, last_post_at = datetime('now'), last_post_user = ? WHERE id = ?")
        .bind(thread_id).bind(&user.username).bind(thread.forum_id).execute(&state.pool).await.ok();

    sqlx::query("UPDATE users SET post_count = post_count + 1 WHERE id = ?")
        .bind(user.id).execute(&state.pool).await.ok();

    // 给帖子作者发送回复通知
    crate::handlers::notification::create_notification(
        &state.pool,
        thread.user_id,
        "reply",
        user.id,
        &user.username,
        Some(thread_id),
        None,
        &format!("{} 回复了你的帖子", user.username),
    ).await;

    Json(json!({"ok": true, "floor": next_floor}))
}
