// 首页处理器：展示最新帖子、热门帖子，支持分页浏览
// 作为论坛入口页面，加载两种帖子列表供前端展示

use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse},
};
use serde::Deserialize;

use crate::config::AppState;
use crate::middleware::auth::{MaybeUser, can_view_forum_async};
use crate::models::forum::Forum;
use crate::models::thread::ThreadList;
use crate::templates::*;

// 分页查询参数，page 为可选的页码
#[derive(Deserialize)]
pub struct PageQuery {
    pub page: Option<i64>,
}

// 首页处理器：获取最新帖子和热门帖子并渲染首页
// 支持未登录用户访问（MaybeUser），登录用户会显示用户信息
pub async fn index(
    MaybeUser(user): MaybeUser,
    state: State<AppState>,
    Query(q): Query<PageQuery>,
) -> impl IntoResponse {
    let pool = &state.pool;
    let per_page = 20i64; // 每页显示 20 条帖子
    let page = q.page.unwrap_or(1).max(1); // 页码最小为 1
    let offset = (page - 1) * per_page;

    // 获取所有版块的浏览权限，构建不可见版块列表
    let all_forums: Vec<Forum> = sqlx::query_as("SELECT * FROM forums")
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    let mut hidden_forum_ids: Vec<i64> = Vec::new();
    for f in &all_forums {
        if !can_view_forum_async(pool, f, user.as_ref()).await {
            hidden_forum_ids.push(f.id);
        }
    }

    // 构建 NOT IN 子句（如果没有隐藏版块则不过滤）
    let where_clause = if hidden_forum_ids.is_empty() {
        String::new()
    } else {
        let ids: Vec<String> = hidden_forum_ids.iter().map(|id| id.to_string()).collect();
        format!("WHERE t.forum_id NOT IN ({})", ids.join(","))
    };

    // 查询最新帖子：置顶帖优先，再按最后回复时间倒序排列
    let recent_threads: Vec<ThreadList> = sqlx::query_as(
        &format!("SELECT t.*, u.username, u.avatar FROM threads t LEFT JOIN users u ON t.user_id = u.id {} ORDER BY t.is_top DESC, t.last_post_at DESC LIMIT ? OFFSET ?", where_clause)
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 查询热门帖子：置顶帖优先，再按浏览量倒序排列，最多 10 条
    let hot_threads: Vec<ThreadList> = sqlx::query_as(
        &format!("SELECT t.*, u.username, u.avatar FROM threads t LEFT JOIN users u ON t.user_id = u.id {} ORDER BY t.is_top DESC, t.view_count DESC LIMIT 10", where_clause)
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 查询帖子总数，用于计算总页数
    let total: (i64,) = if hidden_forum_ids.is_empty() {
        sqlx::query_as("SELECT COUNT(*) FROM threads")
            .fetch_one(pool)
            .await
            .unwrap_or((0,))
    } else {
        let ids: Vec<String> = hidden_forum_ids.iter().map(|id| id.to_string()).collect();
        sqlx::query_as(&format!("SELECT COUNT(*) FROM threads WHERE forum_id NOT IN ({})", ids.join(",")))
            .fetch_one(pool)
            .await
            .unwrap_or((0,))
    };
    let total_pages = ((total.0 as f64) / (per_page as f64)).ceil() as i64;

    Html(render_index(&recent_threads, &hot_threads, user.as_ref(), page, total_pages)).into_response()
}
