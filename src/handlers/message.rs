// 私信处理器：包含收件箱、撰写消息、发送消息、对话详情、回复消息、删除对话、未读消息计数 API
// 消息按对话伙伴分组展示，支持未读消息标记和通知

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect, Json},
    Form,
};
use serde::{Deserialize, Serialize};

use crate::config::AppState;
use crate::middleware::auth::AuthUser;
use crate::models::message::{Message, SendMessageForm, ReplyMessageForm};
use crate::templates::*;

// 撰写消息页面的查询参数，可预填收件人用户名
#[derive(Deserialize)]
pub struct ComposeQuery {
    pub to: Option<String>,
}

// 收件箱页面：展示与所有用户的最新对话列表，按最后消息时间排序
// 每个对话伙伴只显示最新的一条消息
pub async fn inbox(
    AuthUser(user): AuthUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let pool = &state.pool;

    // 查询与每个对话伙伴的最新一条消息（按伙伴分组取最大 ID）
    let conversations: Vec<Message> = sqlx::query_as(
        "SELECT m.*,
            su.username as sender_name, su.avatar as sender_avatar,
            ru.username as receiver_name
         FROM messages m
         LEFT JOIN users su ON m.sender_id = su.id
         LEFT JOIN users ru ON m.receiver_id = ru.id
         WHERE m.id IN (
            SELECT MAX(id) FROM messages
            WHERE sender_id = ? OR receiver_id = ?
            GROUP BY CASE WHEN sender_id = ? THEN receiver_id ELSE sender_id END
         )
         ORDER BY m.created_at DESC"
    )
    .bind(user.id)
    .bind(user.id)
    .bind(user.id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 统计当前用户的未读消息总数
    let unread: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM messages WHERE receiver_id = ? AND is_read = 0"
    )
    .bind(user.id)
    .fetch_one(pool)
    .await
    .unwrap_or((0,));

    Html(render_inbox(&conversations, &user, unread.0)).into_response()
}

// 撰写消息页面：展示发送表单，可通过查询参数预填收件人
pub async fn compose_page(
    AuthUser(user): AuthUser,
    Query(query): Query<ComposeQuery>,
) -> impl IntoResponse {
    Html(render_compose(&user, query.to.as_deref())).into_response()
}

// 发送消息：验证内容和收件人，插入消息记录，发送通知给收件人
pub async fn send_message(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Form(form): Form<SendMessageForm>,
) -> impl IntoResponse {
    let content = form.content.trim();
    let to = form.to.trim();

    if content.is_empty() {
        return Html(render_error("消息内容不能为空")).into_response();
    }
    if to.is_empty() {
        return Html(render_error("请指定收件人")).into_response();
    }

    // 根据用户名查找收件人
    let receiver: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM users WHERE username = ? AND status = 1"
    )
    .bind(to)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let receiver_id = match receiver {
        Some(r) => r.0,
        None => return Html(render_error("用户不存在")).into_response(),
    };

    if receiver_id == user.id {
        return Html(render_error("不能给自己发消息")).into_response();
    }

    sqlx::query(
        "INSERT INTO messages (sender_id, receiver_id, content) VALUES (?, ?, ?)"
    )
    .bind(user.id)
    .bind(receiver_id)
    .bind(content)
    .execute(&state.pool)
    .await
    .ok();

    Redirect::to(&format!("/messages/{}", receiver_id)).into_response()
}

// 对话详情页面：展示当前用户与指定用户的所有消息记录，自动标记未读消息为已读
pub async fn conversation(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(partner_id): Path<i64>,
) -> impl IntoResponse {
    let pool = &state.pool;

    // 获取对话伙伴的用户信息
    let partner: Option<(i64, String, String)> = sqlx::query_as(
        "SELECT id, username, avatar FROM users WHERE id = ? AND status = 1"
    )
    .bind(partner_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let (partner_name, partner_avatar) = match partner {
        Some((_, name, avatar)) => (name, avatar),
        None => return Html(render_error("用户不存在")).into_response(),
    };

    // 查询两个用户之间的所有消息，按时间正序排列
    let messages: Vec<Message> = sqlx::query_as(
        "SELECT m.*,
            su.username as sender_name, su.avatar as sender_avatar,
            ru.username as receiver_name
         FROM messages m
         LEFT JOIN users su ON m.sender_id = su.id
         LEFT JOIN users ru ON m.receiver_id = ru.id
         WHERE (m.sender_id = ? AND m.receiver_id = ?)
            OR (m.sender_id = ? AND m.receiver_id = ?)
         ORDER BY m.created_at ASC"
    )
    .bind(user.id)
    .bind(partner_id)
    .bind(partner_id)
    .bind(user.id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 将对话伙伴发给当前用户的未读消息标记为已读
    sqlx::query(
        "UPDATE messages SET is_read = 1 WHERE sender_id = ? AND receiver_id = ? AND is_read = 0"
    )
    .bind(partner_id)
    .bind(user.id)
    .execute(pool)
    .await
    .ok();

    Html(render_conversation(&messages, &user, partner_id, &partner_name, &partner_avatar)).into_response()
}

// 回复消息：在已有对话中发送新消息，验证收件人存在后插入记录并发送通知
pub async fn reply_message(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(partner_id): Path<i64>,
    Form(form): Form<ReplyMessageForm>,
) -> impl IntoResponse {
    let content = form.content.trim();
    if content.is_empty() {
        return Html(render_error("消息内容不能为空")).into_response();
    }

    // 验证对话伙伴用户是否存在
    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE id = ? AND status = 1")
        .bind(partner_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();

    if exists.is_none() {
        return Html(render_error("用户不存在")).into_response();
    }

    sqlx::query(
        "INSERT INTO messages (sender_id, receiver_id, content) VALUES (?, ?, ?)"
    )
    .bind(user.id)
    .bind(partner_id)
    .bind(content)
    .execute(&state.pool)
    .await
    .ok();

    Redirect::to(&format!("/messages/{}", partner_id)).into_response()
}

// 删除对话：删除当前用户与指定用户之间的所有消息记录
pub async fn delete_conversation(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(partner_id): Path<i64>,
) -> impl IntoResponse {
    sqlx::query(
        "DELETE FROM messages WHERE (sender_id = ? AND receiver_id = ?) OR (sender_id = ? AND receiver_id = ?)"
    )
    .bind(user.id)
    .bind(partner_id)
    .bind(partner_id)
    .bind(user.id)
    .execute(&state.pool)
    .await
    .ok();

    Redirect::to("/messages").into_response()
}

// 未读消息计数 API 响应结构
#[derive(Serialize)]
struct UnreadResponse {
    count: i64,
}

// 未读消息计数 API：返回当前用户的未读私信数量
pub async fn api_unread_count(
    AuthUser(user): AuthUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM messages WHERE receiver_id = ? AND is_read = 0"
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or((0,));

    Json(UnreadResponse { count: count.0 }).into_response()
}
