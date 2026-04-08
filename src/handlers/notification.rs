// 通知处理器：包含通知创建、通知列表获取、标记已读（单条/全部）、
// 帖子/回复的行内编辑 API（JSON 格式）、帖子内容获取 API
// 通知类型包括：reply（回复）、quote（引用）、message（私信）

use axum::{
    extract::{Path, State},
    response::Json,
};
use serde_json::{json, Value};
use sqlx::Row;

use crate::config::AppState;
use crate::middleware::auth::AuthUser;

// 创建通知：被其他处理器调用，向指定用户发送通知
// 如果目标用户就是操作者本人则跳过（不通知自己）
pub async fn create_notification(
    pool: &sqlx::SqlitePool,
    user_id: i64,
    ntype: &str,
    from_user_id: i64,
    from_username: &str,
    thread_id: Option<i64>,
    post_id: Option<i64>,
    content: &str,
) {
    // 不通知自己
    if user_id == from_user_id {
        return;
    }
    sqlx::query(
        "INSERT INTO notifications (user_id, type, from_user_id, from_username, thread_id, post_id, content) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(user_id)
    .bind(ntype)
    .bind(from_user_id)
    .bind(from_username)
    .bind(thread_id)
    .bind(post_id)
    .bind(content)
    .execute(pool)
    .await
    .ok();
}

// 通知列表 API：返回当前用户最近 15 条通知、未读通知数和未读私信数
pub async fn api_notifications(
    AuthUser(user): AuthUser,
    state: State<AppState>,
) -> Json<Value> {
    let rows = sqlx::query(
        "SELECT id, user_id, type, from_user_id, from_username, thread_id, post_id, content, is_read, created_at FROM notifications WHERE user_id = ? AND type != 'message' ORDER BY created_at DESC LIMIT 15"
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let notifications: Vec<Value> = rows.iter().map(|r| json!({
        "id": r.get::<i64, _>("id"),
        "type": r.get::<String, _>("type"),
        "from_user_id": r.get::<i64, _>("from_user_id"),
        "from_username": r.get::<String, _>("from_username"),
        "thread_id": r.get::<Option<i64>, _>("thread_id"),
        "post_id": r.get::<Option<i64>, _>("post_id"),
        "content": r.get::<String, _>("content"),
        "created_at": r.get::<String, _>("created_at"),
    })).collect();

    // 所有通知都是未读的（已读即删除），返回数量作为未读数
    let notif_count = notifications.len() as i64;

    // 同时查询未读私信数量
    let msg_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM messages WHERE receiver_id = ? AND is_read = 0"
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or((0,));

    Json(json!({
        "notifications": notifications,
        "unread_count": notif_count,
        "unread_messages": msg_count.0,
        "total_unread": notif_count + msg_count.0,
    }))
}

// 切除单条通知（已读后删除）
pub async fn api_notification_read(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(notif_id): Path<i64>,
) -> Json<Value> {
    sqlx::query("DELETE FROM notifications WHERE id = ? AND user_id = ?")
        .bind(notif_id)
        .bind(user.id)
        .execute(&state.pool)
        .await
        .ok();

    Json(json!({"ok": true}))
}

// 清除所有通知（全部已读后删除）
pub async fn api_notification_read_all(
    AuthUser(user): AuthUser,
    state: State<AppState>,
) -> Json<Value> {
    sqlx::query("DELETE FROM notifications WHERE user_id = ?")
        .bind(user.id)
        .execute(&state.pool)
        .await
        .ok();

    Json(json!({"ok": true}))
}

// 帖子回复行内编辑 API（JSON）：验证权限后更新回复内容，不能编辑首帖
pub async fn api_edit_post(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(post_id): Path<i64>,
    Json(body): Json<serde_json::Value>,
) -> Json<Value> {
    let content = body.get("content").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if content.is_empty() {
        return Json(json!({"ok": false, "error": "内容不能为空"}));
    }

    // 获取帖子信息并验证权限
    let post = sqlx::query(
        "SELECT p.user_id, p.is_first, p.thread_id FROM posts p WHERE p.id = ?"
    )
    .bind(post_id)
    .fetch_optional(&state.pool)
    .await;

    let post = match post {
        Ok(Some(row)) => (
            row.get::<i64, _>("user_id"),
            row.get::<i64, _>("is_first"),
            row.get::<i64, _>("thread_id"),
        ),
        _ => return Json(json!({"ok": false, "error": "回复不存在"})),
    };

    let (owner_id, is_first, thread_id) = post;

    if owner_id != user.id {
        return Json(json!({"ok": false, "error": "无权编辑此回复"}));
    }

    if is_first == 1 {
        return Json(json!({"ok": false, "error": "请编辑主题来修改首帖"}));
    }

    sqlx::query("UPDATE posts SET content = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(&content)
        .bind(post_id)
        .execute(&state.pool)
        .await
        .ok();

    Json(json!({"ok": true, "thread_id": thread_id}))
}

// 帖子主题行内编辑 API（JSON）：验证权限后同时更新标题和首帖内容
pub async fn api_edit_thread(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(thread_id): Path<i64>,
    Json(body): Json<serde_json::Value>,
) -> Json<Value> {
    let title = body.get("title").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let content = body.get("content").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();

    if title.is_empty() || content.is_empty() {
        return Json(json!({"ok": false, "error": "标题和内容不能为空"}));
    }

    let thread = sqlx::query(
        "SELECT user_id FROM threads WHERE id = ?"
    )
    .bind(thread_id)
    .fetch_optional(&state.pool)
    .await;

    let owner_id = match thread {
        Ok(Some(row)) => row.get::<i64, _>("user_id"),
        _ => return Json(json!({"ok": false, "error": "帖子不存在"})),
    };

    if owner_id != user.id {
        return Json(json!({"ok": false, "error": "无权编辑此帖子"}));
    }

    sqlx::query("UPDATE threads SET title = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(&title)
        .bind(thread_id)
        .execute(&state.pool)
        .await
        .ok();

    sqlx::query("UPDATE posts SET content = ?, updated_at = datetime('now') WHERE thread_id = ? AND is_first = 1")
        .bind(&content)
        .bind(thread_id)
        .execute(&state.pool)
        .await
        .ok();

    Json(json!({"ok": true, "thread_id": thread_id}))
}

// 获取帖子内容 API：用于行内编辑时加载原始内容
pub async fn api_get_post(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(post_id): Path<i64>,
) -> Json<Value> {
    let post = sqlx::query(
        "SELECT p.id, p.content, p.user_id, p.is_first, p.thread_id, t.title as thread_title FROM posts p LEFT JOIN threads t ON p.thread_id = t.id WHERE p.id = ?"
    )
    .bind(post_id)
    .fetch_optional(&state.pool)
    .await;

    match post {
        Ok(Some(row)) => {
            let owner_id: i64 = row.get("user_id");
            if owner_id != user.id {
                return Json(json!({"ok": false, "error": "无权编辑"}));
            }
            Json(json!({
                "ok": true,
                "post": {
                    "id": row.get::<i64, _>("id"),
                    "content": row.get::<String, _>("content"),
                    "is_first": row.get::<i64, _>("is_first"),
                    "thread_id": row.get::<i64, _>("thread_id"),
                    "thread_title": row.get::<String, _>("thread_title"),
                }
            }))
        }
        _ => Json(json!({"ok": false, "error": "帖子不存在"})),
    }
}
