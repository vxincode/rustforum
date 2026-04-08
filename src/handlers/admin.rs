// 管理后台处理器：包含仪表盘、版块管理、用户管理、帖子管理、举报管理、
// 黑名单管理、禁言管理、AI 审查测试、系统设置（站点/注册/积分/上传/AI）、
// 邀请码管理、登录日志等功能
// 所有管理接口均需管理员权限（AdminUser 提取器）

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Json, Redirect},
    Form,
};
use serde::Deserialize;
use std::collections::HashMap;

use crate::config::AppState;
use crate::middleware::auth::AdminUser;
use crate::models::blacklist::{BlacklistEntry, MutedUserWithInfo};
use crate::models::forum::Forum;
use crate::models::forum_moderator::ForumModeratorWithUser;
use crate::models::post::Post;
use crate::models::report::ReportWithReporter;
use crate::models::thread::Thread;
use crate::models::user::User;
use crate::templates::*;

// === 仪表盘：展示站点统计数据、待处理举报、今日签到/新用户、最近举报和新用户 ===
pub async fn dashboard(
    _admin: AdminUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let stats = get_stats(&state.pool).await;
    let pending_reports: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reports WHERE status='pending'")
        .fetch_one(&state.pool).await.unwrap_or((0,));
    let today_checkins: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM checkins WHERE date(created_at) = date('now')")
        .fetch_one(&state.pool).await.unwrap_or((0,));
    let today_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE date(created_at) = date('now')")
        .fetch_one(&state.pool).await.unwrap_or((0,));
    let blacklist_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM blacklist")
        .fetch_one(&state.pool).await.unwrap_or((0,));

    let recent_reports: Vec<ReportWithReporter> = sqlx::query_as(
        "SELECT r.*, u.username as reporter_name, NULL as target_title, NULL as target_content FROM reports r LEFT JOIN users u ON r.reporter_id = u.id ORDER BY r.created_at DESC LIMIT 5"
    ).fetch_all(&state.pool).await.unwrap_or_default();

    let recent_users: Vec<User> = sqlx::query_as(
        "SELECT * FROM users ORDER BY id DESC LIMIT 5"
    ).fetch_all(&state.pool).await.unwrap_or_default();

    Html(render_admin_dashboard(
        &stats, pending_reports.0, today_checkins.0, today_users.0, blacklist_count.0,
        &recent_reports, &recent_users,
    )).into_response()
}

// === 版块管理：展示所有版块列表 ===
pub async fn forums(
    _admin: AdminUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let forums: Vec<Forum> = sqlx::query_as(
        "SELECT * FROM forums ORDER BY sort_order ASC, id ASC"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // 加载所有版块的版主信息
    let moderators: Vec<ForumModeratorWithUser> = sqlx::query_as(
        "SELECT fm.id, fm.forum_id, fm.user_id, u.username, fm.created_at FROM forum_moderators fm LEFT JOIN users u ON fm.user_id = u.id ORDER BY fm.forum_id, fm.id"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Html(render_admin_forums(&forums, &moderators)).into_response()
}

// 创建版块：插入新版块记录
pub async fn create_forum(
    _admin: AdminUser,
    state: State<AppState>,
    Form(form): Form<CreateForumFormQuery>,
) -> impl IntoResponse {
    if form.name.trim().is_empty() {
        return Html(render_error("版块名称不能为空")).into_response();
    }

    sqlx::query(
        "INSERT INTO forums (name, description, sort_order) VALUES (?, ?, ?)"
    )
    .bind(&form.name)
    .bind(&form.description)
    .bind(form.sort_order.unwrap_or(0))
    .execute(&state.pool)
    .await
    .ok();

    crate::cache::invalidate(&state.redis, &["api:forums", "api:stats"]).await;

    Redirect::to("/admin/forums?saved=1").into_response()
}

// 编辑版块：更新版块名称、描述、排序和状态
pub async fn edit_forum(
    _admin: AdminUser,
    state: State<AppState>,
    Path(forum_id): Path<i64>,
    Form(form): Form<EditForumFormQuery>,
) -> impl IntoResponse {
    sqlx::query(
        "UPDATE forums SET name = ?, description = ?, sort_order = ?, status = ?, view_perm = ?, post_perm = ? WHERE id = ?"
    )
    .bind(&form.name)
    .bind(&form.description)
    .bind(form.sort_order.unwrap_or(0))
    .bind(form.status.unwrap_or(1))
    .bind(form.view_perm.unwrap_or(0))
    .bind(form.post_perm.unwrap_or(0))
    .bind(forum_id)
    .execute(&state.pool)
    .await
    .ok();

    crate::cache::invalidate(&state.redis, &["api:forums", "api:stats"]).await;

    Redirect::to("/admin/forums?saved=1").into_response()
}

// 删除版块：同时删除该版块下的所有帖子和回复
pub async fn delete_forum(
    _admin: AdminUser,
    state: State<AppState>,
    Path(forum_id): Path<i64>,
) -> impl IntoResponse {
    sqlx::query("DELETE FROM posts WHERE forum_id = ?")
        .bind(forum_id)
        .execute(&state.pool)
        .await
        .ok();

    sqlx::query("DELETE FROM threads WHERE forum_id = ?")
        .bind(forum_id)
        .execute(&state.pool)
        .await
        .ok();

    sqlx::query("DELETE FROM forums WHERE id = ?")
        .bind(forum_id)
        .execute(&state.pool)
        .await
        .ok();

    crate::cache::invalidate(&state.redis, &["api:forums", "api:stats"]).await;

    Redirect::to("/admin/forums?saved=1").into_response()
}

// === 用户管理：展示所有用户列表及禁言状态 ===
pub async fn users(
    _admin: AdminUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let users: Vec<User> = sqlx::query_as(
        "SELECT * FROM users ORDER BY id ASC"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let muted: Vec<(i64,)> = sqlx::query_as("SELECT user_id FROM muted_users")
        .fetch_all(&state.pool).await.unwrap_or_default();
    let muted_ids: Vec<i64> = muted.into_iter().map(|(id,)| id).collect();

    Html(render_admin_users(&users, &muted_ids)).into_response()
}

// 切换用户状态：启用/禁用用户（status 在 0 和 1 之间切换）
pub async fn toggle_user_status(
    _admin: AdminUser,
    state: State<AppState>,
    Path(user_id): Path<i64>,
) -> impl IntoResponse {
    // 查询当前状态
    let current: Option<(i64, String)> = sqlx::query_as(
        "SELECT status, username FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    sqlx::query("UPDATE users SET status = CASE WHEN status = 1 THEN 0 ELSE 1 END WHERE id = ?")
        .bind(user_id)
        .execute(&state.pool)
        .await
        .ok();

    // 发送站内通知
    if let Some((status, _username)) = current {
        let msg = if status == 1 {
            format!("你的账号已被管理员封禁，如有疑问请联系管理员。")
        } else {
            format!("你的账号已被管理员解封，欢迎回来！")
        };
        crate::handlers::notification::create_notification(
            &state.pool, user_id, "system", _admin.0.id, &_admin.0.username, None, None, &msg,
        ).await;
    }

    Redirect::to("/admin/users?saved=1").into_response()
}

// 设置用户组：修改用户的 group_id（管理员/版主/普通用户）
pub async fn set_user_group(
    _admin: AdminUser,
    state: State<AppState>,
    Path((user_id, group_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    let group_name = match group_id {
        1 => "管理员",
        2 => "版主",
        _ => "普通会员",
    };
    sqlx::query("UPDATE users SET group_id = ? WHERE id = ?")
        .bind(group_id)
        .bind(user_id)
        .execute(&state.pool)
        .await
        .ok();

    crate::handlers::notification::create_notification(
        &state.pool, user_id, "system", _admin.0.id, &_admin.0.username, None, None,
        &format!("你的用户组已被管理员变更为「{}」。", group_name),
    ).await;

    Redirect::to("/admin/users?saved=1").into_response()
}

// === 帖子管理（管理员）：分页展示所有帖子，支持按页浏览 ===
pub async fn threads_page(
    _admin: AdminUser,
    state: State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let page: i64 = params.get("page").and_then(|v| v.parse().ok()).unwrap_or(1);
    let per_page: i64 = 30;
    let offset = (page - 1) * per_page;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM threads")
        .fetch_one(&state.pool).await.unwrap_or((0,));
    let total_pages = ((total.0 as f64) / (per_page as f64)).ceil() as i64;

    let rows: Vec<AdminThreadRow> = sqlx::query_as(
        "SELECT t.id, t.title, u.username as author_name, f.name as forum_name, t.reply_count, t.is_top, t.is_essence, t.is_closed, t.created_at FROM threads t LEFT JOIN users u ON t.user_id = u.id LEFT JOIN forums f ON t.forum_id = f.id ORDER BY t.created_at DESC LIMIT ? OFFSET ?"
    )
    .bind(per_page).bind(offset)
    .fetch_all(&state.pool).await.unwrap_or_default();

    Html(render_admin_threads(&rows, page, total_pages)).into_response()
}

// 切换帖子置顶状态
pub async fn toggle_sticky(
    _admin: AdminUser,
    state: State<AppState>,
    Path(thread_id): Path<i64>,
) -> impl IntoResponse {
    sqlx::query("UPDATE threads SET is_top = CASE WHEN is_top = 1 THEN 0 ELSE 1 END, updated_at = datetime('now') WHERE id = ?")
        .bind(thread_id)
        .execute(&state.pool)
        .await
        .ok();

    // 重定向回帖子页面
    Redirect::to(&format!("/thread/{}", thread_id)).into_response()
}

// 切换帖子关闭状态（关闭后不可回复）
pub async fn toggle_close(
    _admin: AdminUser,
    state: State<AppState>,
    Path(thread_id): Path<i64>,
) -> impl IntoResponse {
    sqlx::query("UPDATE threads SET is_closed = CASE WHEN is_closed = 1 THEN 0 ELSE 1 END, updated_at = datetime('now') WHERE id = ?")
        .bind(thread_id)
        .execute(&state.pool)
        .await
        .ok();

    Redirect::to(&format!("/thread/{}", thread_id)).into_response()
}

// 切换帖子精华状态
pub async fn toggle_essence(
    _admin: AdminUser,
    state: State<AppState>,
    Path(thread_id): Path<i64>,
) -> impl IntoResponse {
    sqlx::query("UPDATE threads SET is_essence = CASE WHEN is_essence = 1 THEN 0 ELSE 1 END, updated_at = datetime('now') WHERE id = ?")
        .bind(thread_id)
        .execute(&state.pool)
        .await
        .ok();

    Redirect::to(&format!("/thread/{}", thread_id)).into_response()
}

// 管理员删除帖子：同时删除所有回复，更新版块和用户计数器
pub async fn admin_delete_thread(
    _admin: AdminUser,
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
        Some(t) => t,
        None => return Html(render_error("帖子不存在")).into_response(),
    };

    let post_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE thread_id = ?")
        .bind(thread_id)
        .fetch_one(pool)
        .await
        .unwrap_or((0,));

    sqlx::query("DELETE FROM posts WHERE thread_id = ?")
        .bind(thread_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM threads WHERE id = ?")
        .bind(thread_id).execute(pool).await.ok();
    sqlx::query("UPDATE forums SET thread_count = thread_count - 1, post_count = post_count - ? WHERE id = ?")
        .bind(post_count.0).bind(thread.forum_id).execute(pool).await.ok();
    sqlx::query("UPDATE users SET thread_count = thread_count - 1, post_count = post_count - ? WHERE id = ?")
        .bind(post_count.0).bind(thread.user_id).execute(pool).await.ok();

    Redirect::to("/admin/threads?saved=1").into_response()
}

// === 移动帖子：将帖子从一个版块移动到另一个版块 ===

// 移动帖子表单数据
#[derive(Deserialize)]
pub struct MoveThreadForm {
    pub target_forum_id: i64,
}

// 移动帖子页面：展示当前帖子和目标版块选择
pub async fn move_thread_page(
    _admin: AdminUser,
    state: State<AppState>,
    Path(thread_id): Path<i64>,
) -> impl IntoResponse {
    let pool = &state.pool;

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

    let current_forum: Option<Forum> = sqlx::query_as("SELECT * FROM forums WHERE id = ?")
        .bind(thread.forum_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    let current_forum = match current_forum {
        Some(f) => f,
        None => return Html(render_error("当前版块不存在")).into_response(),
    };

    let forums: Vec<Forum> = sqlx::query_as("SELECT * FROM forums WHERE status = 1 ORDER BY sort_order ASC, id ASC")
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    Html(render_move_thread(&thread, &current_forum, &forums)).into_response()
}

// 执行移动帖子操作：更新帖子和回复的版块归属，调整两个版块的计数器
pub async fn move_thread(
    _admin: AdminUser,
    state: State<AppState>,
    Path(thread_id): Path<i64>,
    Form(form): Form<MoveThreadForm>,
) -> impl IntoResponse {
    let pool = &state.pool;

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

    // 校验目标版块存在且启用
    let target: Option<Forum> = sqlx::query_as("SELECT * FROM forums WHERE id = ? AND status = 1")
        .bind(form.target_forum_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    let _target = match target {
        Some(f) => f,
        None => return Html(render_error("目标版块不存在或已关闭")).into_response(),
    };

    // 如果目标版块就是当前版块，直接重定向回去
    if form.target_forum_id == thread.forum_id {
        return Redirect::to(&format!("/thread/{}", thread_id)).into_response();
    }

    // 计算帖子下的回复数（含首帖）
    let post_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE thread_id = ?")
        .bind(thread_id)
        .fetch_one(pool)
        .await
        .unwrap_or((0,));

    // 更新 threads 表
    sqlx::query("UPDATE threads SET forum_id = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(form.target_forum_id)
        .bind(thread_id)
        .execute(pool)
        .await
        .ok();

    // 更新 posts 表
    sqlx::query("UPDATE posts SET forum_id = ? WHERE thread_id = ?")
        .bind(form.target_forum_id)
        .bind(thread_id)
        .execute(pool)
        .await
        .ok();

    // 原版块计数减少
    sqlx::query("UPDATE forums SET thread_count = thread_count - 1, post_count = post_count - ? WHERE id = ?")
        .bind(post_count.0)
        .bind(thread.forum_id)
        .execute(pool)
        .await
        .ok();

    // 目标版块计数增加
    sqlx::query("UPDATE forums SET thread_count = thread_count + 1, post_count = post_count + ? WHERE id = ?")
        .bind(post_count.0)
        .bind(form.target_forum_id)
        .execute(pool)
        .await
        .ok();

    Redirect::to(&format!("/thread/{}", thread_id)).into_response()
}

// 管理员删除回复：删除单条回复，更新帖子/版块/用户计数器
pub async fn admin_delete_post(
    _admin: AdminUser,
    state: State<AppState>,
    Path(post_id): Path<i64>,
) -> impl IntoResponse {
    let pool = &state.pool;

    let post: Option<Post> = sqlx::query_as(
        "SELECT p.*, u.username, u.avatar, u.group_id FROM posts p LEFT JOIN users u ON p.user_id = u.id WHERE p.id = ?"
    )
    .bind(post_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let post = match post {
        Some(p) => p,
        None => return Html(render_error("回复不存在")).into_response(),
    };

    let thread_id = post.thread_id;
    sqlx::query("DELETE FROM posts WHERE id = ?").bind(post_id).execute(pool).await.ok();

    if post.is_first == 0 {
        sqlx::query("UPDATE threads SET reply_count = reply_count - 1 WHERE id = ?")
            .bind(thread_id).execute(pool).await.ok();
    }
    sqlx::query("UPDATE forums SET post_count = post_count - 1 WHERE id = ?")
        .bind(post.forum_id).execute(pool).await.ok();
    sqlx::query("UPDATE users SET post_count = post_count - 1 WHERE id = ?")
        .bind(post.user_id).execute(pool).await.ok();

    Redirect::to(&format!("/thread/{}", thread_id)).into_response()
}

// === 举报管理：展示举报列表，支持按状态筛选 ===

// 举报查询参数
#[derive(Deserialize)]
pub struct ReportsQuery {
    pub status: Option<String>,
}

// 举报列表页面：按状态统计各类型举报数量，支持按状态筛选
pub async fn reports_page(
    _admin: AdminUser,
    state: State<AppState>,
    Query(params): Query<ReportsQuery>,
) -> impl IntoResponse {
    let status_filter = params.status.as_deref().unwrap_or("all");

    // 统计各状态的举报数量
    let counts_raw: Vec<(String, i64)> = sqlx::query_as(
        "SELECT status, COUNT(*) as cnt FROM reports GROUP BY status"
    ).fetch_all(&state.pool).await.unwrap_or_default();

    // (pending, reviewing, resolved, dismissed)
    let mut counts = (0i64, 0i64, 0i64, 0i64);
    for (s, c) in &counts_raw {
        match s.as_str() {
            "pending" => counts.0 = *c,
            "reviewing" => counts.1 = *c,
            "resolved" => counts.2 = *c,
            "dismissed" => counts.3 = *c,
            _ => {}
        }
    }

    let reports: Vec<ReportWithReporter> = if status_filter == "all" {
        sqlx::query_as(
            "SELECT r.*, u.username as reporter_name, NULL as target_title, NULL as target_content FROM reports r LEFT JOIN users u ON r.reporter_id = u.id ORDER BY r.created_at DESC LIMIT 50"
        ).fetch_all(&state.pool).await.unwrap_or_default()
    } else {
        sqlx::query_as(
            "SELECT r.*, u.username as reporter_name, NULL as target_title, NULL as target_content FROM reports r LEFT JOIN users u ON r.reporter_id = u.id WHERE r.status = ? ORDER BY r.created_at DESC LIMIT 50"
        ).bind(status_filter).fetch_all(&state.pool).await.unwrap_or_default()
    };

    Html(render_admin_reports(&reports, status_filter, counts)).into_response()
}

// 举报操作表单数据
#[derive(Deserialize)]
pub struct ReportActionForm {
    pub action: String,
    pub note: Option<String>,
}

// 处理举报：将举报状态更新为 resolved（已解决）、dismissed（已驳回）或 reviewing（审查中）
pub async fn report_action(
    _admin: AdminUser,
    state: State<AppState>,
    Path(report_id): Path<i64>,
    Form(form): Form<ReportActionForm>,
) -> impl IntoResponse {
    let new_status = match form.action.as_str() {
        "resolve" => "resolved",
        "dismiss" => "dismissed",
        _ => "reviewing",
    };
    sqlx::query("UPDATE reports SET status = ?, admin_note = ?, resolved_at = datetime('now') WHERE id = ?")
        .bind(new_status)
        .bind(form.note.unwrap_or_default())
        .bind(report_id)
        .execute(&state.pool)
        .await
        .ok();

    Redirect::to("/admin/reports?saved=1").into_response()
}

// === 黑名单管理：展示 IP/用户黑名单和禁言用户列表 ===
pub async fn blacklist_page(
    _admin: AdminUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let entries: Vec<BlacklistEntry> = sqlx::query_as(
        "SELECT * FROM blacklist ORDER BY created_at DESC"
    ).fetch_all(&state.pool).await.unwrap_or_default();

    let muted: Vec<MutedUserWithInfo> = sqlx::query_as(
        "SELECT m.*, u.username, a.username as admin_name FROM muted_users m LEFT JOIN users u ON m.user_id = u.id LEFT JOIN users a ON m.admin_id = a.id ORDER BY m.created_at DESC"
    ).fetch_all(&state.pool).await.unwrap_or_default();

    Html(render_admin_blacklist(&entries, &muted)).into_response()
}

// 添加黑名单表单数据
#[derive(Deserialize)]
pub struct AddBlacklistForm {
    pub r#type: String,
    pub value: String,
    pub reason: Option<String>,
}

// 添加黑名单条目：支持 IP 和用户名类型
pub async fn add_blacklist(
    _admin: AdminUser,
    state: State<AppState>,
    admin: AdminUser,
    Form(form): Form<AddBlacklistForm>,
) -> impl IntoResponse {
    if form.value.trim().is_empty() {
        return Html(render_error("值不能为空")).into_response();
    }
    sqlx::query("INSERT OR IGNORE INTO blacklist (type, value, reason, admin_id) VALUES (?, ?, ?, ?)")
        .bind(&form.r#type)
        .bind(form.value.trim())
        .bind(form.reason.unwrap_or_default())
        .bind(admin.0.id)
        .execute(&state.pool)
        .await
        .ok();

    Redirect::to("/admin/blacklist?saved=1").into_response()
}

// 移除黑名单条目
pub async fn remove_blacklist(
    _admin: AdminUser,
    state: State<AppState>,
    Path(entry_id): Path<i64>,
) -> impl IntoResponse {
    sqlx::query("DELETE FROM blacklist WHERE id = ?")
        .bind(entry_id)
        .execute(&state.pool)
        .await
        .ok();

    Redirect::to("/admin/blacklist?saved=1").into_response()
}

// === 禁言管理：禁言/解禁用户 ===

// 禁言表单数据
#[derive(Deserialize)]
pub struct MuteForm {
    pub days: Option<i64>,
    pub reason: Option<String>,
}

// 禁言用户：设置禁言天数和原因，days=0 表示永久禁言
pub async fn mute_user(
    _admin: AdminUser,
    state: State<AppState>,
    Path(user_id): Path<i64>,
    Form(form): Form<MuteForm>,
) -> impl IntoResponse {
    let days = form.days.unwrap_or(7);
    let expires = if days == 0 {
        None
    } else {
        let exp: (String,) = sqlx::query_as("SELECT datetime('now', '+' || ? || ' days')")
            .bind(days).fetch_one(&state.pool).await.unwrap_or(("9999-12-31".to_string(),));
        Some(exp.0)
    };

    let reason_text = form.reason.unwrap_or_default();
    let duration_text = if days == 0 { "永久禁言".to_string() } else { format!("禁言 {} 天", days) };
    let notice_msg = if reason_text.is_empty() {
        format!("你已被管理员{}。", duration_text)
    } else {
        format!("你已被管理员{}，原因：{}", duration_text, reason_text)
    };

    sqlx::query("INSERT OR REPLACE INTO muted_users (user_id, reason, admin_id, expires_at) VALUES (?, ?, ?, ?)")
        .bind(user_id)
        .bind(&reason_text)
        .bind(_admin.0.id)
        .bind(&expires)
        .execute(&state.pool)
        .await
        .ok();

    crate::handlers::notification::create_notification(
        &state.pool, user_id, "system", _admin.0.id, &_admin.0.username, None, None, &notice_msg,
    ).await;

    Redirect::to("/admin/users?saved=1").into_response()
}

// 解禁用户：删除禁言记录
pub async fn unmute_user(
    _admin: AdminUser,
    state: State<AppState>,
    Path(user_id): Path<i64>,
) -> impl IntoResponse {
    sqlx::query("DELETE FROM muted_users WHERE user_id = ?")
        .bind(user_id)
        .execute(&state.pool)
        .await
        .ok();

    crate::handlers::notification::create_notification(
        &state.pool, user_id, "system", _admin.0.id, &_admin.0.username, None, None,
        "你已被管理员解除禁言，现在可以正常发言了。",
    ).await;

    Redirect::to("/admin/blacklist?saved=1").into_response()
}

// === AI 审查测试：管理员可在后台测试 AI 内容审查功能 ===
pub async fn review_page(
    _admin: AdminUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let settings = get_settings_map(&state.pool).await;
    Html(render_admin_review(&settings)).into_response()
}

// AI 审查测试表单数据
#[derive(Deserialize)]
pub struct ReviewForm {
    pub content: String,
}

// AI 内容审查测试：调用外部 AI API 对内容进行安全检测，返回审查结果
pub async fn review_content(
    _admin: AdminUser,
    state: State<AppState>,
    Form(form): Form<ReviewForm>,
) -> impl IntoResponse {
    let settings = get_settings_map(&state.pool).await;
    let enabled = settings.get("ai_review_enabled").map(|v| v.as_str()).unwrap_or("0");
    if enabled != "1" {
        return Json(serde_json::json!({"safe": true, "level": "safe", "reason": "AI 审查未启用"})).into_response();
    }

    let api_url = settings.get("ai_review_api_url").cloned().unwrap_or_default();
    let api_key = settings.get("ai_review_api_key").cloned().unwrap_or_default();
    let prompt = settings.get("ai_review_prompt").cloned().unwrap_or_default();
    let model = settings.get("ai_review_model").cloned().unwrap_or_else(|| "gpt-4o-mini".to_string());

    if api_url.is_empty() {
        return Json(serde_json::json!({"safe": true, "level": "safe", "reason": "API URL 未配置"})).into_response();
    }

    // 调用 AI API 进行内容审查
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": prompt},
            {"role": "user", "content": form.content}
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
            // 尝试解析 AI API 返回的 JSON 响应
            if let Ok(full) = serde_json::from_str::<serde_json::Value>(&text) {
                let content_text = full["choices"][0]["message"]["content"].as_str().unwrap_or("");
                // 尝试将 AI 返回的内容解析为 JSON 格式
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content_text) {
                    Json(parsed).into_response()
                } else {
                    // 无法解析为 JSON 时，将原始内容作为原因返回
                    Json(serde_json::json!({
                        "safe": false,
                        "level": "warning",
                        "reason": content_text
                    })).into_response()
                }
            } else {
                Json(serde_json::json!({"safe": true, "level": "safe", "reason": "无法解析API响应"})).into_response()
            }
        }
        Err(e) => {
            Json(serde_json::json!({"safe": true, "level": "safe", "reason": format!("API调用失败: {}", e)})).into_response()
        }
    }
}

// === 系统设置：包含站点设置、注册设置、积分设置、上传设置、AI 审查设置 ===

// 设置入口页：重定向到站点设置子页面
pub async fn settings_page() -> impl IntoResponse {
    Redirect::to("/admin/settings/site")
}

// 站点设置页面：展示站点名称、描述、关键词、页脚文字等配置
pub async fn settings_site_page(
    _admin: AdminUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let settings = get_settings_map(&state.pool).await;
    Html(render_settings_site(&settings)).into_response()
}

// 设置表单数据（动态字段使用 HashMap 接收）
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct SettingsForm {
    pub key: Option<String>,
    // 动态字段以 HashMap 形式传入
}

// 保存站点设置并刷新全局配置缓存
pub async fn settings_site_save(
    _admin: AdminUser,
    state: State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    save_settings(&state.pool, &form, &["site_name", "site_description", "site_keywords", "site_footer_text"]).await;
    // 刷新全局配置缓存，使新设置立即生效
    crate::site_config::load_from_db(&state.pool).await;
    crate::cache::invalidate(&state.redis, &["api:stats"]).await;
    Redirect::to("/admin/settings/site?saved=1").into_response()
}

// 注册设置页面
pub async fn settings_register_page(
    _admin: AdminUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let settings = get_settings_map(&state.pool).await;
    Html(render_settings_register(&settings)).into_response()
}

// 保存注册设置
pub async fn settings_register_save(
    _admin: AdminUser,
    state: State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    save_settings(&state.pool, &form, &["allow_register", "invite_required"]).await;
    Redirect::to("/admin/settings/register?saved=1").into_response()
}

// 积分设置页面
pub async fn settings_credits_page(
    _admin: AdminUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let settings = get_settings_map(&state.pool).await;
    Html(render_settings_credits(&settings)).into_response()
}

// 保存积分设置
pub async fn settings_credits_save(
    _admin: AdminUser,
    state: State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    save_settings(&state.pool, &form, &["credits_checkin", "credits_thread", "credits_reply", "credits_essence"]).await;
    Redirect::to("/admin/settings/credits?saved=1").into_response()
}

// 上传设置页面
pub async fn settings_upload_page(
    _admin: AdminUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let settings = get_settings_map(&state.pool).await;
    Html(render_settings_upload(&settings)).into_response()
}

// 保存上传设置
pub async fn settings_upload_save(
    _admin: AdminUser,
    state: State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    save_settings(&state.pool, &form, &["max_avatar_size"]).await;
    Redirect::to("/admin/settings/upload?saved=1").into_response()
}

// AI 审查设置页面
pub async fn settings_ai_page(
    _admin: AdminUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let settings = get_settings_map(&state.pool).await;
    Html(render_settings_ai(&settings)).into_response()
}

// 保存 AI 审查设置
pub async fn settings_ai_save(
    _admin: AdminUser,
    state: State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    save_settings(&state.pool, &form, &["ai_review_enabled", "ai_review_api_url", "ai_review_api_key", "ai_review_prompt"]).await;
    Redirect::to("/admin/settings/ai?saved=1").into_response()
}

// === 邮件设置 ===

// 邮件设置页面
pub async fn settings_email_page(
    _admin: AdminUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let settings = get_settings_map(&state.pool).await;
    Html(render_settings_email(&settings)).into_response()
}

// 保存邮件设置
pub async fn settings_email_save(
    _admin: AdminUser,
    state: State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    save_settings(&state.pool, &form, &[
        "email_enabled", "email_provider", "email_from_name", "email_from_address",
        "email_sendflare_api_url", "email_sendflare_api_key",
        "email_smtp_host", "email_smtp_port", "email_smtp_username", "email_smtp_password", "email_smtp_encryption",
        "email_verification_enabled", "email_verify_expire_hours", "site_url",
    ]).await;
    Redirect::to("/admin/settings/email?saved=1").into_response()
}

// 测试邮件发送 API
#[derive(Deserialize)]
pub struct TestEmailForm {
    pub to: String,
}

pub async fn settings_email_test(
    _admin: AdminUser,
    state: State<AppState>,
    Json(form): Json<TestEmailForm>,
) -> impl IntoResponse {
    if form.to.trim().is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "请输入收件人地址"}));
    }
    let (ok, message) = crate::email::test_email_send(&state.pool, form.to.trim()).await;
    Json(serde_json::json!({"ok": ok, "message": message}))
}

// === 辅助函数和数据结构 ===

// 站点统计数据结构
#[derive(Debug)]
pub struct SiteStats {
    pub total_users: i64,
    pub total_threads: i64,
    pub total_posts: i64,
    pub total_forums: i64,
}

// 获取站点统计数据（用户数、帖子数、回复数、版块数）
async fn get_stats(pool: &sqlx::SqlitePool) -> SiteStats {
    let users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool).await.unwrap_or((0,));
    let threads: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM threads")
        .fetch_one(pool).await.unwrap_or((0,));
    let posts: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts")
        .fetch_one(pool).await.unwrap_or((0,));
    let forums: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM forums")
        .fetch_one(pool).await.unwrap_or((0,));

    SiteStats {
        total_users: users.0,
        total_threads: threads.0,
        total_posts: posts.0,
        total_forums: forums.0,
    }
}

// 从数据库读取所有设置项为 HashMap
async fn get_settings_map(pool: &sqlx::SqlitePool) -> HashMap<String, String> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(pool).await.unwrap_or_default();
    rows.into_iter().collect()
}

// 批量保存设置项到数据库
async fn save_settings(pool: &sqlx::SqlitePool, form: &HashMap<String, String>, keys: &[&str]) {
    for key in keys {
        if let Some(value) = form.get(*key) {
            sqlx::query("UPDATE settings SET value = ? WHERE key = ?")
                .bind(value)
                .bind(key)
                .execute(pool)
                .await
                .ok();
        }
    }
}

// 管理后台帖子行数据结构（用于帖子管理列表展示）
#[derive(Debug, sqlx::FromRow)]
pub struct AdminThreadRow {
    pub id: i64,
    pub title: String,
    pub author_name: String,
    pub forum_name: String,
    pub reply_count: i64,
    pub is_top: i64,
    pub is_essence: i64,
    pub is_closed: i64,
    pub created_at: String,
}

// 创建版块表单数据
#[derive(Deserialize)]
pub struct CreateForumFormQuery {
    pub name: String,
    pub description: String,
    pub sort_order: Option<i64>,
}

// 编辑版块表单数据
#[derive(Deserialize)]
pub struct EditForumFormQuery {
    pub name: String,
    pub description: String,
    pub sort_order: Option<i64>,
    pub status: Option<i64>,
    pub view_perm: Option<i64>,
    pub post_perm: Option<i64>,
}

// =====================================================================
// 邀请码管理：展示邀请码列表、批量生成邀请码、删除邀请码
// =====================================================================

// 邀请码行数据结构
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct InviteCodeRow {
    pub id: i64,
    pub code: String,
    pub created_by: i64,
    pub max_uses: i64,
    pub used_count: i64,
    pub created_at: String,
}

// 邀请码列表页面
pub async fn invite_codes_page(_admin: AdminUser, State(state): State<AppState>) -> impl IntoResponse {
    let codes: Vec<InviteCodeRow> = sqlx::query_as(
        "SELECT id, code, created_by, max_uses, used_count, created_at FROM invite_codes ORDER BY created_at DESC"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Html(crate::templates::render_admin_invite_codes(&codes, _admin.0.id)).into_response()
}

// 创建邀请码表单数据
#[derive(Deserialize)]
pub struct CreateInviteCodeForm {
    pub count: Option<i64>,
    pub max_uses: Option<i64>,
}

// 批量生成邀请码：每个邀请码为 12 位随机字符串，支持设置最大使用次数
pub async fn create_invite_codes(
    _admin: AdminUser,
    State(state): State<AppState>,
    Form(form): Form<CreateInviteCodeForm>,
) -> impl IntoResponse {
    let count = form.count.unwrap_or(1).max(1).min(100);
    let max_uses = form.max_uses.unwrap_or(1).max(1);

    for _ in 0..count {
        // 生成 12 位随机邀请码
        let code = uuid::Uuid::new_v4().to_string().replace('-', "").chars().take(12).collect::<String>();
        sqlx::query("INSERT INTO invite_codes (code, created_by, max_uses) VALUES (?, ?, ?)")
            .bind(&code)
            .bind(_admin.0.id)
            .bind(max_uses)
            .execute(&state.pool)
            .await
            .ok();
    }

    Redirect::to("/admin/invite-codes?saved=1").into_response()
}

// 删除邀请码
pub async fn delete_invite_code(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    sqlx::query("DELETE FROM invite_codes WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await
        .ok();
    Redirect::to("/admin/invite-codes?saved=1").into_response()
}

// === 登录日志管理：展示用户登录记录，支持按用户筛选和分页 ===

// 登录日志行数据结构
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct LoginLogRow {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub ip: String,
    pub user_agent: String,
    pub action: String,
    pub success: i64,
    pub created_at: String,
}

// 登录日志列表页面：支持按用户 ID 筛选，按时间倒序分页展示
pub async fn login_logs_page(
    _admin: AdminUser,
    state: State<AppState>,
    Query(q): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let page: i64 = q.get("page").and_then(|v| v.parse().ok()).unwrap_or(1).max(1);
    let per_page: i64 = 30;
    let offset = (page - 1) * per_page;

    // 如果指定了 user_id 参数，则按用户筛选；否则展示所有日志
    let (logs, total) = if let Some(uid) = q.get("user_id").and_then(|v| v.parse::<i64>().ok()) {
        let logs: Vec<LoginLogRow> = sqlx::query_as(
            "SELECT * FROM login_logs WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(uid).bind(per_page).bind(offset)
        .fetch_all(&state.pool).await.unwrap_or_default();
        let cnt: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM login_logs WHERE user_id = ?")
            .bind(uid).fetch_one(&state.pool).await.unwrap_or((0,));
        (logs, cnt.0)
    } else {
        let logs: Vec<LoginLogRow> = sqlx::query_as(
            "SELECT * FROM login_logs ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(per_page).bind(offset)
        .fetch_all(&state.pool).await.unwrap_or_default();
        let cnt: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM login_logs")
            .fetch_one(&state.pool).await.unwrap_or((0,));
        (logs, cnt.0)
    };

    let total_pages = ((total as f64) / (per_page as f64)).ceil() as i64;
    Html(render_admin_login_logs(&logs, page, total_pages)).into_response()
}

// === 版主管理：为版块分配/移除版主 ===

// 分配版主表单数据
#[derive(Deserialize)]
pub struct AddModeratorForm {
    pub user_id: i64,
}

// 为版块添加版主
pub async fn add_forum_moderator(
    _admin: AdminUser,
    state: State<AppState>,
    Path(forum_id): Path<i64>,
    Form(form): Form<AddModeratorForm>,
) -> impl IntoResponse {
    // 检查用户是否存在
    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ? AND status = 1")
        .bind(form.user_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();
    if user.is_none() {
        return Html(render_error("用户不存在")).into_response();
    }

    sqlx::query("INSERT OR IGNORE INTO forum_moderators (forum_id, user_id) VALUES (?, ?)")
        .bind(forum_id)
        .bind(form.user_id)
        .execute(&state.pool)
        .await
        .ok();

    Redirect::to("/admin/forums?saved=1").into_response()
}

// 移除版块版主
pub async fn remove_forum_moderator(
    _admin: AdminUser,
    state: State<AppState>,
    Path((forum_id, user_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    sqlx::query("DELETE FROM forum_moderators WHERE forum_id = ? AND user_id = ?")
        .bind(forum_id)
        .bind(user_id)
        .execute(&state.pool)
        .await
        .ok();

    Redirect::to("/admin/forums?saved=1").into_response()
}
