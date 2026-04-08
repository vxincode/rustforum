// 帖子与回复处理器：包含帖子详情查看、回复帖子、编辑帖子/回复、删除帖子/回复
// 还包含 AI 内容安全审查功能，通过外部 API 对发布内容进行合规检测

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;

use crate::config::AppState;
use crate::middleware::auth::{AuthUser, MaybeUser, can_view_forum_async, can_post_in_forum};
use crate::models::forum::Forum;
use crate::models::post::Post;
use crate::models::thread::Thread;
use crate::templates::*;

// AI 内容安全审查：调用外部 AI API 检查内容是否合规
// 返回 None 表示安全通过，返回 Some(reason) 表示被拦截及拦截原因
pub async fn ai_review_check(pool: &sqlx::SqlitePool, content: &str) -> Option<String> {
    // 检查 AI 审查是否启用
    let enabled: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'ai_review_enabled'"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    if enabled.as_ref().map(|v| v.0.as_str()) != Some("1") {
        return None;
    }

    // 从数据库读取 AI 审查的配置项：API 地址、密钥、提示词、模型名称
    let api_url: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'ai_review_api_url'"
    ).fetch_optional(pool).await.ok().flatten();
    let api_url = api_url?.0;
    if api_url.is_empty() { return None; }

    let api_key: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'ai_review_api_key'"
    ).fetch_optional(pool).await.ok().flatten();
    let api_key = api_key.map(|v| v.0).unwrap_or_default();

    let prompt: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'ai_review_prompt'"
    ).fetch_optional(pool).await.ok().flatten();
    let prompt = prompt.map(|v| v.0).unwrap_or_default();

    let model: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'ai_review_model'"
    ).fetch_optional(pool).await.ok().flatten();
    let model = model.map(|v| v.0).unwrap_or_else(|| "gpt-4o-mini".to_string());

    // 构建并发送请求到 AI API（OpenAI 兼容格式）
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": prompt},
            {"role": "user", "content": content}
        ],
        "temperature": 0.1
    });

    let result = client.post(&api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send().await;

    match result {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                let content_str = json.get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("");
                if let Ok(review) = serde_json::from_str::<serde_json::Value>(content_str) {
                    let safe = review.get("safe").and_then(|v| v.as_bool()).unwrap_or(true);
                    if !safe {
                        let reason = review.get("reason").and_then(|v| v.as_str()).unwrap_or("内容未通过安全审查");
                        return Some(reason.to_string());
                    }
                }
            }
            None
        }
        Err(_) => None, // API 调用失败时放行，不阻塞用户操作
    }
}

// 帖子详情页面：展示帖子信息和所有回复，支持分页，浏览量+1
pub async fn thread_view(
    MaybeUser(user): MaybeUser,
    state: State<AppState>,
    Path(thread_id): Path<i64>,
    Query(page): Query<PageQuery>,
) -> impl IntoResponse {
    let pool = &state.pool;
    let per_page = state.config.posts_per_page;

    let thread: Option<Thread> = sqlx::query_as(
        "SELECT t.*, u.username, f.name as forum_name FROM threads t LEFT JOIN users u ON t.user_id = u.id LEFT JOIN forums f ON t.forum_id = f.id WHERE t.id = ?"
    )
    .bind(thread_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let thread = match thread {
        Some(t) => t,
        None => return Html(render_error("帖子不存在")).into_response(),
    };

    // 检查版块浏览权限
    let forum: Option<Forum> = sqlx::query_as("SELECT * FROM forums WHERE id = ?")
        .bind(thread.forum_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
    if let Some(f) = &forum {
        if !can_view_forum_async(pool, f, user.as_ref()).await {
            return Html(render_error("你没有权限浏览此版块")).into_response();
        }
    }

    let page = page.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let posts: Vec<Post> = sqlx::query_as(
        "SELECT p.*, u.username, u.avatar, u.group_id, u.signature, u.custom_title, u.epithet, u.epithet_color, u.status as user_status, (SELECT reason FROM muted_users WHERE user_id = p.user_id) as user_muted FROM posts p LEFT JOIN users u ON p.user_id = u.id WHERE p.thread_id = ? ORDER BY p.floor ASC LIMIT ? OFFSET ?"
    )
    .bind(thread_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 帖子浏览量 +1
    sqlx::query("UPDATE threads SET view_count = view_count + 1 WHERE id = ?")
        .bind(thread_id)
        .execute(pool)
        .await
        .ok();

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE thread_id = ?")
        .bind(thread_id)
        .fetch_one(pool)
        .await
        .unwrap_or((0,));

    let total_pages = ((total.0 as f64) / (per_page as f64)).ceil() as i64;

    Html(render_thread_view(
        &thread,
        &posts,
        page,
        total_pages,
        user.as_ref(),
    ))
    .into_response()
}

// 回复帖子：验证内容、检查帖子是否关闭、AI 审查、创建回复、更新计数、发送通知
pub async fn reply_thread(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(thread_id): Path<i64>,
    Form(form): Form<ReplyFormQuery>,
) -> impl IntoResponse {
    let pool = &state.pool;

    // 检查禁言状态
    if let Some(msg) = crate::middleware::auth::get_mute_status(pool, user.id).await {
        return Html(render_error(&msg)).into_response();
    }

    if form.content.trim().is_empty() {
        return Html(render_error("回复内容不能为空")).into_response();
    }

    let thread: Option<Thread> = sqlx::query_as("SELECT * FROM threads WHERE id = ?")
        .bind(thread_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    let thread = match thread {
        Some(t) if t.is_closed == 0 => t,
        Some(_) => return Html(render_error("帖子已关闭，无法回复")).into_response(),
        None => return Html(render_error("帖子不存在")).into_response(),
    };

    // 检查版块发帖权限
    let forum: Option<Forum> = sqlx::query_as("SELECT * FROM forums WHERE id = ?")
        .bind(thread.forum_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
    if let Some(f) = &forum {
        if !can_post_in_forum(pool, f, &user).await {
            return Html(render_error("你没有权限在此版块发帖")).into_response();
        }
    }

    // AI content review
    if let Some(reason) = ai_review_check(pool, &form.content).await {
        return Html(render_error(&format!("内容未通过安全审查：{}，请修改后重新发布", reason))).into_response();
    }

    // 获取当前帖子已有的楼层数，新回复楼层 = 当前数 + 1
    let floor: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE thread_id = ?")
        .bind(thread_id)
        .fetch_one(pool)
        .await
        .unwrap_or((0,));

    let next_floor = floor.0 + 1;

    // 插入新回复记录（is_first=0 表示非首帖）
    sqlx::query(
        "INSERT INTO posts (thread_id, forum_id, user_id, content, floor, is_first) VALUES (?, ?, ?, ?, ?, 0)"
    )
    .bind(thread_id)
    .bind(thread.forum_id)
    .bind(user.id)
    .bind(&form.content)
    .bind(next_floor)
    .execute(pool)
    .await
    .ok();

    // 更新帖子的回复计数和最后回复信息
    sqlx::query(
        "UPDATE threads SET reply_count = reply_count + 1, last_post_at = datetime('now'), last_post_user = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(&user.username)
    .bind(thread_id)
    .execute(pool)
    .await
    .ok();

    // 更新版块的回复计数和最后发帖信息
    sqlx::query(
        "UPDATE forums SET post_count = post_count + 1, last_thread_id = ?, last_post_at = datetime('now'), last_post_user = ? WHERE id = ?"
    )
    .bind(thread_id)
    .bind(&user.username)
    .bind(thread.forum_id)
    .execute(pool)
    .await
    .ok();

    // 更新用户的回复计数
    sqlx::query("UPDATE users SET post_count = post_count + 1 WHERE id = ?")
        .bind(user.id)
        .execute(pool)
        .await
        .ok();

    // 给帖子作者发送回复通知（不会通知自己）
    crate::handlers::notification::create_notification(
        pool,
        thread.user_id,
        "reply",
        user.id,
        &user.username,
        Some(thread_id),
        None,
        &format!("{} 回复了你的帖子", user.username),
    ).await;

    // 重定向到帖子最后一页的新回复位置
    Redirect::to(&format!("/thread/{}?page={}#floor-{}", thread_id, i64::MAX, next_floor)).into_response()
}

// =====================================================================
// 编辑帖子页面：需登录，仅帖子作者可访问
// =====================================================================

pub async fn edit_thread_page(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(thread_id): Path<i64>,
) -> impl IntoResponse {
    let thread: Option<Thread> = sqlx::query_as("SELECT * FROM threads WHERE id = ?")
        .bind(thread_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();

    let thread = match thread {
        Some(t) if t.user_id == user.id => t,
        Some(_) => return Html(render_error("无权编辑此帖子")).into_response(),
        None => return Html(render_error("帖子不存在")).into_response(),
    };

    // 获取首帖内容用于编辑
    let post: Option<Post> = sqlx::query_as(
        "SELECT p.*, u.username, u.avatar, u.group_id, u.signature, u.custom_title, u.epithet, u.epithet_color FROM posts p LEFT JOIN users u ON p.user_id = u.id WHERE p.thread_id = ? AND p.is_first = 1"
    )
    .bind(thread_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let content = post.map(|p| p.content).unwrap_or_default();

    Html(render_edit_thread(&thread, &content)).into_response()
}

// =====================================================================
// 保存帖子编辑：需登录，仅帖子作者可操作，更新标题和首帖内容
// =====================================================================

// 编辑帖子表单数据
#[derive(Deserialize)]
pub struct EditThreadForm {
    pub title: String,
    pub content: String,
}

pub async fn edit_thread(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(thread_id): Path<i64>,
    Form(form): Form<EditThreadForm>,
) -> impl IntoResponse {
    let pool = &state.pool;

    let thread: Option<Thread> = sqlx::query_as("SELECT * FROM threads WHERE id = ?")
        .bind(thread_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    match thread {
        Some(t) if t.user_id == user.id => t,
        Some(_) => return Html(render_error("无权编辑此帖子")).into_response(),
        None => return Html(render_error("帖子不存在")).into_response(),
    };

    let title = form.title.trim();
    let content = form.content.trim();

    if title.is_empty() {
        return Html(render_error("标题不能为空")).into_response();
    }
    if content.is_empty() {
        return Html(render_error("内容不能为空")).into_response();
    }

    // 更新帖子标题
    sqlx::query("UPDATE threads SET title = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(title)
        .bind(thread_id)
        .execute(pool)
        .await
        .ok();

    // 更新首帖内容
    sqlx::query("UPDATE posts SET content = ?, updated_at = datetime('now') WHERE thread_id = ? AND is_first = 1")
        .bind(content)
        .bind(thread_id)
        .execute(pool)
        .await
        .ok();

    Redirect::to(&format!("/thread/{}", thread_id)).into_response()
}

// =====================================================================
// 删除帖子：需登录，仅帖子作者可操作，同时删除所有回复并更新计数器
// =====================================================================

pub async fn delete_thread(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(thread_id): Path<i64>,
) -> impl IntoResponse {
    let pool = &state.pool;

    let thread: Option<Thread> = sqlx::query_as("SELECT * FROM threads WHERE id = ?")
        .bind(thread_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    let thread = match thread {
        Some(t) if t.user_id == user.id => t,
        Some(_) => return Html(render_error("无权删除此帖子")).into_response(),
        None => return Html(render_error("帖子不存在")).into_response(),
    };

    // 统计帖子下的回复数量，用于更新计数器
    let post_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE thread_id = ?")
        .bind(thread_id)
        .fetch_one(pool)
        .await
        .unwrap_or((0,));

    // 先删除帖子下的所有回复
    sqlx::query("DELETE FROM posts WHERE thread_id = ?")
        .bind(thread_id)
        .execute(pool)
        .await
        .ok();

    // 再删除帖子本身
    sqlx::query("DELETE FROM threads WHERE id = ?")
        .bind(thread_id)
        .execute(pool)
        .await
        .ok();

    // 更新版块计数器
    sqlx::query("UPDATE forums SET thread_count = thread_count - 1, post_count = post_count - ? WHERE id = ?")
        .bind(post_count.0)
        .bind(thread.forum_id)
        .execute(pool)
        .await
        .ok();

    // 更新用户计数器
    sqlx::query("UPDATE users SET thread_count = thread_count - 1, post_count = post_count - ? WHERE id = ?")
        .bind(post_count.0)
        .bind(user.id)
        .execute(pool)
        .await
        .ok();

    Redirect::to(&format!("/forum/{}", thread.forum_id)).into_response()
}

// =====================================================================
// 编辑回复页面：需登录，仅回复作者可访问（不能编辑首帖，首帖需通过编辑帖子功能修改）
// =====================================================================

pub async fn edit_post_page(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(post_id): Path<i64>,
) -> impl IntoResponse {
    let post: Option<Post> = sqlx::query_as(
        "SELECT p.*, u.username, u.avatar, u.group_id, u.signature, u.custom_title, u.epithet, u.epithet_color FROM posts p LEFT JOIN users u ON p.user_id = u.id WHERE p.id = ?"
    )
    .bind(post_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let post = match post {
        Some(p) if p.user_id == user.id && p.is_first == 0 => p,
        Some(p) if p.is_first == 1 => return Html(render_error("请编辑主题来修改首帖")).into_response(),
        Some(_) => return Html(render_error("无权编辑此回复")).into_response(),
        None => return Html(render_error("回复不存在")).into_response(),
    };

    // 获取帖子信息用于面包屑导航
    let thread: Option<Thread> = sqlx::query_as("SELECT * FROM threads WHERE id = ?")
        .bind(post.thread_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();

    let thread_title = thread.map(|t| t.title).unwrap_or_default();

    Html(render_edit_post(&post, &thread_title)).into_response()
}

// =====================================================================
// 保存回复编辑：需登录，仅回复作者可操作
// =====================================================================

// 编辑回复表单数据
#[derive(Deserialize)]
pub struct EditPostForm {
    pub content: String,
}

pub async fn edit_post(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(post_id): Path<i64>,
    Form(form): Form<EditPostForm>,
) -> impl IntoResponse {
    let pool = &state.pool;

    let post: Option<Post> = sqlx::query_as(
        "SELECT p.*, u.username, u.avatar, u.group_id, u.signature, u.custom_title, u.epithet, u.epithet_color FROM posts p LEFT JOIN users u ON p.user_id = u.id WHERE p.id = ?"
    )
    .bind(post_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let thread_id = match post {
        Some(p) if p.user_id == user.id && p.is_first == 0 => p.thread_id,
        Some(_) => return Html(render_error("无权编辑此回复")).into_response(),
        None => return Html(render_error("回复不存在")).into_response(),
    };

    let content = form.content.trim();
    if content.is_empty() {
        return Html(render_error("内容不能为空")).into_response();
    }

    sqlx::query("UPDATE posts SET content = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(content)
        .bind(post_id)
        .execute(pool)
        .await
        .ok();

    Redirect::to(&format!("/thread/{}", thread_id)).into_response()
}

// =====================================================================
// 删除回复：需登录，仅回复作者可操作（不能删除首帖，需通过删除帖子功能）
// =====================================================================

pub async fn delete_post(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(post_id): Path<i64>,
) -> impl IntoResponse {
    let pool = &state.pool;

    let post: Option<Post> = sqlx::query_as(
        "SELECT p.*, u.username, u.avatar, u.group_id, u.signature, u.custom_title, u.epithet, u.epithet_color FROM posts p LEFT JOIN users u ON p.user_id = u.id WHERE p.id = ?"
    )
    .bind(post_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let post = match post {
        Some(p) if p.is_first == 1 => return Html(render_error("不能删除首帖，请删除整个主题")).into_response(),
        Some(p) if p.user_id == user.id => p,
        Some(_) => return Html(render_error("无权删除此回复")).into_response(),
        None => return Html(render_error("回复不存在")).into_response(),
    };

    let thread_id = post.thread_id;
    let forum_id = post.forum_id;

    // 删除回复记录
    sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(post_id)
        .execute(pool)
        .await
        .ok();

    // 更新帖子回复计数
    sqlx::query("UPDATE threads SET reply_count = reply_count - 1 WHERE id = ?")
        .bind(thread_id)
        .execute(pool)
        .await
        .ok();

    // 更新版块帖子计数
    sqlx::query("UPDATE forums SET post_count = post_count - 1 WHERE id = ?")
        .bind(forum_id)
        .execute(pool)
        .await
        .ok();

    // 更新用户帖子计数
    sqlx::query("UPDATE users SET post_count = post_count - 1 WHERE id = ?")
        .bind(user.id)
        .execute(pool)
        .await
        .ok();

    Redirect::to(&format!("/thread/{}", thread_id)).into_response()
}

// 分页查询参数
#[derive(Deserialize)]
pub struct PageQuery {
    pub page: Option<i64>,
}

// 回复表单数据
#[derive(Deserialize)]
pub struct ReplyFormQuery {
    pub content: String,
}
