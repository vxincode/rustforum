// 版块处理器：包含版块列表展示、版块内帖子浏览（含置顶帖/普通帖/分页）、
// 发帖页面展示（指定版块/通用发帖）、创建帖子（含 AI 内容审查）

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;

use crate::config::AppState;
use crate::middleware::auth::{AuthUser, MaybeUser, can_view_forum_async, can_post_in_forum};
use crate::models::forum::Forum;
use crate::models::thread::ThreadList;
use crate::templates::*;

// 版块列表页面：展示所有启用的版块，按排序字段和 ID 升序排列
pub async fn forum_list(
    MaybeUser(user): MaybeUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let pool = &state.pool;

    let all_forums: Vec<Forum> = sqlx::query_as(
        "SELECT * FROM forums WHERE status = 1 ORDER BY sort_order ASC, id ASC"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 过滤掉用户无权浏览的版块
    let mut forums = Vec::new();
    for f in all_forums {
        if can_view_forum_async(pool, &f, user.as_ref()).await {
            forums.push(f);
        }
    }

    Html(render_forum_list(&forums, user.as_ref())).into_response()
}

// 版块详情页面：展示指定版块的置顶帖、普通帖子列表，支持分页
pub async fn forum_view(
    MaybeUser(user): MaybeUser,
    state: State<AppState>,
    Path(forum_id): Path<i64>,
    Query(page): Query<PageQuery>,
) -> impl IntoResponse {
    let pool = &state.pool;
    let per_page = state.config.threads_per_page;

    // 查询版块信息，仅返回状态正常的版块
    let forum: Option<Forum> = sqlx::query_as("SELECT * FROM forums WHERE id = ? AND status = 1")
        .bind(forum_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    let forum = match forum {
        Some(f) => f,
        None => return Html(render_error("版块不存在")).into_response(),
    };

    // 检查浏览权限
    if !can_view_forum_async(pool, &forum, user.as_ref()).await {
        return Html(render_error("你没有权限浏览此版块")).into_response();
    }

    let page = page.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    // 查询该版块的所有置顶帖（is_top=1），按创建时间倒序
    let sticky_threads: Vec<ThreadList> = sqlx::query_as(
        "SELECT t.*, u.username, u.avatar FROM threads t LEFT JOIN users u ON t.user_id = u.id WHERE t.forum_id = ? AND t.is_top = 1 ORDER BY t.created_at DESC"
    )
    .bind(forum_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 查询该版块的普通帖子（is_top=0），按最后回复时间倒序，支持分页
    let threads: Vec<ThreadList> = sqlx::query_as(
        "SELECT t.*, u.username, u.avatar FROM threads t LEFT JOIN users u ON t.user_id = u.id WHERE t.forum_id = ? AND t.is_top = 0 ORDER BY t.last_post_at DESC LIMIT ? OFFSET ?"
    )
    .bind(forum_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 查询普通帖子总数，用于计算总页数
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM threads WHERE forum_id = ? AND is_top = 0"
    )
    .bind(forum_id)
    .fetch_one(pool)
    .await
    .unwrap_or((0,));

    let total_pages = ((total.0 as f64) / (per_page as f64)).ceil() as i64;

    // 检查发帖权限
    let can_post = match &user {
        Some(u) => can_post_in_forum(pool, &forum, u).await,
        None => false,
    };

    Html(render_forum_view(
        &forum,
        &sticky_threads,
        &threads,
        page,
        total_pages,
        user.as_ref(),
        can_post,
    ))
    .into_response()
}

// 发帖页面（指定版块）：需登录，加载当前版块和所有版块信息供选择
pub async fn new_thread_page(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(forum_id): Path<i64>,
) -> impl IntoResponse {
    let pool = &state.pool;

    let forum: Option<Forum> = sqlx::query_as("SELECT * FROM forums WHERE id = ? AND status = 1")
        .bind(forum_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    let forum = match forum {
        Some(f) => f,
        None => return Html(render_error("版块不存在")).into_response(),
    };

    // 检查发帖权限
    if !can_post_in_forum(pool, &forum, &user).await {
        return Html(render_error("你没有权限在此版块发帖")).into_response();
    }

    // 加载所有版块供用户在发帖时切换
    let all_forums: Vec<Forum> = sqlx::query_as(
        "SELECT * FROM forums WHERE status = 1 ORDER BY sort_order ASC, id ASC"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Html(render_new_thread(&forum, &all_forums, &user)).into_response()
}

// 通用发帖页面：不指定版块，让用户从所有版块中选择
pub async fn new_thread_generic(
    AuthUser(user): AuthUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let all_forums: Vec<Forum> = sqlx::query_as(
        "SELECT * FROM forums WHERE status = 1 ORDER BY sort_order ASC, id ASC"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    if all_forums.is_empty() {
        return Html(render_error("暂无可用版块")).into_response();
    }

    Html(render_new_thread_generic(&all_forums, &user)).into_response()
}

// 创建帖子：验证标题和内容、AI 内容审查、创建帖子记录和首帖、更新计数器
pub async fn create_thread(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(forum_id): Path<i64>,
    Form(form): Form<NewThreadFormQuery>,
) -> impl IntoResponse {
    let pool = &state.pool;

    // 检查禁言状态
    if let Some(msg) = crate::middleware::auth::get_mute_status(pool, user.id).await {
        return Html(render_error(&msg)).into_response();
    }

    // 检查版块存在性和发帖权限
    let forum: Option<Forum> = sqlx::query_as("SELECT * FROM forums WHERE id = ? AND status = 1")
        .bind(forum_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    match forum {
        Some(f) if !can_post_in_forum(pool, &f, &user).await => {
            return Html(render_error("你没有权限在此版块发帖")).into_response();
        }
        None => return Html(render_error("版块不存在")).into_response(),
        _ => {}
    }

    // 验证标题和内容不为空
    if form.title.trim().is_empty() || form.content.trim().is_empty() {
        return Html(render_error("标题和内容不能为空")).into_response();
    }

    // AI 内容安全审查：调用外部 AI 接口检查内容是否合规
    if let Some(reason) = crate::handlers::thread::ai_review_check(pool, &form.content).await {
        return Html(render_error(&format!("内容未通过安全审查：{}，请修改后重新发布", reason))).into_response();
    }

    // 先创建帖子记录
    let result = sqlx::query(
        "INSERT INTO threads (forum_id, user_id, title, last_post_at, last_post_user) VALUES (?, ?, ?, datetime('now'), ?)"
    )
    .bind(forum_id)
    .bind(user.id)
    .bind(&form.title)
    .bind(&user.username)
    .execute(pool)
    .await;

    let thread_id = match result {
        Ok(r) => r.last_insert_rowid(),
        Err(_) => return Html(render_error("发帖失败")).into_response(),
    };

    // 创建首帖（floor=1, is_first=1）
    sqlx::query(
        "INSERT INTO posts (thread_id, forum_id, user_id, content, floor, is_first) VALUES (?, ?, ?, ?, 1, 1)"
    )
    .bind(thread_id)
    .bind(forum_id)
    .bind(user.id)
    .bind(&form.content)
    .execute(pool)
    .await
    .ok();

    // 更新版块计数器：帖子数+1、回复数+1、最后发帖信息
    sqlx::query("UPDATE forums SET thread_count = thread_count + 1, post_count = post_count + 1, last_thread_id = ?, last_post_at = datetime('now'), last_post_user = ? WHERE id = ?")
        .bind(thread_id)
        .bind(&user.username)
        .bind(forum_id)
        .execute(pool)
        .await
        .ok();

    // 更新用户计数器：发帖数+1、回复数+1
    sqlx::query("UPDATE users SET thread_count = thread_count + 1, post_count = post_count + 1 WHERE id = ?")
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

// 发帖表单数据：包含标题和内容
#[derive(Deserialize)]
pub struct NewThreadFormQuery {
    pub title: String,
    pub content: String,
}
