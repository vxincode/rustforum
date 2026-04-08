// AI 共享处理器：AI Prompt / Skill 分享功能
// 支持创建、编辑、删除、积分兑换、分类筛选、搜索

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;

use crate::config::AppState;
use crate::middleware::auth::{AuthUser, MaybeUser};
use crate::models::ai_share::{AiShare, AiShareList};
use crate::templates::*;

// =====================================================================
// 列表页：支持分类筛选、搜索、分页
// =====================================================================

#[derive(Deserialize)]
pub struct AiShareListQuery {
    pub category: Option<String>,
    pub share_type: Option<String>,
    pub q: Option<String>,
    pub page: Option<i64>,
}

pub async fn ai_share_list(
    MaybeUser(user): MaybeUser,
    state: State<AppState>,
    Query(q): Query<AiShareListQuery>,
) -> impl IntoResponse {
    let pool = &state.pool;
    let per_page: i64 = 12;
    let page = q.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let category = q.category.as_deref().unwrap_or("");
    let share_type = q.share_type.as_deref().unwrap_or("");
    let search = q.q.as_deref().unwrap_or("");

    // Build WHERE conditions
    let mut conditions = vec!["s.status = 1".to_string()];
    if !category.is_empty() {
        conditions.push(format!("s.category = '{}'", category.replace('\'', "''")));
    }
    if !share_type.is_empty() {
        conditions.push(format!("s.share_type = '{}'", share_type.replace('\'', "''")));
    }
    if !search.is_empty() {
        let pat = format!("%{}%", search.replace('%', "\\%").replace('_', "\\_").replace('\'', "''"));
        conditions.push(format!("(s.title LIKE '{}' OR s.description LIKE '{}')", pat, pat));
    }
    let where_sql = conditions.join(" AND ");

    let shares: Vec<AiShareList> = sqlx::query_as(
        &format!("SELECT s.*, u.username, u.avatar FROM ai_shares s LEFT JOIN users u ON s.user_id = u.id WHERE {} ORDER BY s.created_at DESC LIMIT ? OFFSET ?", where_sql)
    )
    .bind(per_page).bind(offset)
    .fetch_all(pool).await.unwrap_or_default();

    let total: (i64,) = sqlx::query_as(
        &format!("SELECT COUNT(*) FROM ai_shares s WHERE {}", where_sql)
    ).fetch_one(pool).await.unwrap_or((0,));

    let total_pages = ((total.0 as f64) / (per_page as f64)).ceil() as i64;

    Html(render_ai_share_list_page(&shares, user.as_ref(), category, share_type, search, page, total_pages)).into_response()
}

// =====================================================================
// 详情页
// =====================================================================

pub async fn ai_share_detail(
    MaybeUser(user): MaybeUser,
    state: State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let pool = &state.pool;

    let share: Option<AiShare> = sqlx::query_as(
        "SELECT * FROM ai_shares WHERE id = ? AND status = 1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let Some(share) = share else {
        return Html(render_error("AI 共享内容不存在")).into_response();
    };

    // Get author info
    let author: Option<(String, String)> = sqlx::query_as(
        "SELECT username, avatar FROM users WHERE id = ?"
    )
    .bind(share.user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let (username, avatar) = author.unwrap_or(("未知".to_string(), String::new()));

    // Check if user has purchased or is the author
    let can_view_full = match &user {
        Some(u) if u.id == share.user_id => true,
        Some(u) => {
            let purchased: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM ai_share_purchases WHERE share_id = ? AND user_id = ?"
            )
            .bind(id).bind(u.id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
            purchased.is_some()
        }
        None => false,
    };

    // Also check if it's free
    let can_view_full = can_view_full || share.price == 0;

    Html(render_ai_share_detail_page(&share, &username, &avatar, user.as_ref(), can_view_full)).into_response()
}

// =====================================================================
// 创建页面
// =====================================================================

pub async fn ai_share_create_page(
    AuthUser(_user): AuthUser,
) -> impl IntoResponse {
    Html(render_ai_share_form(None))
}

// =====================================================================
// 提交创建
// =====================================================================

#[derive(Deserialize)]
pub struct AiShareForm {
    pub title: String,
    pub description: String,
    pub content: String,
    pub category: String,
    pub share_type: String,
    pub price: Option<i64>,
}

pub async fn ai_share_create(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Form(form): Form<AiShareForm>,
) -> impl IntoResponse {
    if form.title.trim().is_empty() || form.content.trim().is_empty() {
        return Html(render_error("标题和内容不能为空")).into_response();
    }

    let price = form.price.unwrap_or(0).max(0);

    let result = sqlx::query_as::<_, (i64,)>(
        "INSERT INTO ai_shares (user_id, title, description, content, category, share_type, price) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id"
    )
    .bind(user.id)
    .bind(form.title.trim())
    .bind(form.description.trim())
    .bind(&form.content)
    .bind(&form.category)
    .bind(&form.share_type)
    .bind(price)
    .fetch_one(&state.pool)
    .await;

    match result {
        Ok((id,)) => Redirect::to(&format!("/ai/{}", id)).into_response(),
        Err(e) => {
            tracing::error!("Create AI share error: {:?}", e);
            Html(render_error("创建失败，请重试")).into_response()
        }
    }
}

// =====================================================================
// 编辑页面
// =====================================================================

pub async fn ai_share_edit_page(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let share: Option<AiShare> = sqlx::query_as(
        "SELECT * FROM ai_shares WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    match share {
        Some(s) if s.user_id == user.id || user.is_admin() => {
            Html(render_ai_share_form(Some(&s))).into_response()
        }
        _ => Html(render_error("无权编辑")).into_response(),
    }
}

// =====================================================================
// 提交编辑
// =====================================================================

pub async fn ai_share_edit(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<AiShareForm>,
) -> impl IntoResponse {
    let share: Option<AiShare> = sqlx::query_as(
        "SELECT * FROM ai_shares WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let Some(share) = share else {
        return Html(render_error("内容不存在")).into_response();
    };

    if share.user_id != user.id && !user.is_admin() {
        return Html(render_error("无权编辑")).into_response();
    }

    let price = form.price.unwrap_or(0).max(0);

    sqlx::query(
        "UPDATE ai_shares SET title = ?, description = ?, content = ?, category = ?, share_type = ?, price = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(form.title.trim())
    .bind(form.description.trim())
    .bind(&form.content)
    .bind(&form.category)
    .bind(&form.share_type)
    .bind(price)
    .bind(id)
    .execute(&state.pool)
    .await
    .ok();

    Redirect::to(&format!("/ai/{}", id)).into_response()
}

// =====================================================================
// 删除
// =====================================================================

pub async fn ai_share_delete(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let share: Option<(i64,)> = sqlx::query_as(
        "SELECT user_id FROM ai_shares WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    match share {
        Some((uid,)) if uid == user.id || user.is_admin() => {
            sqlx::query("UPDATE ai_shares SET status = 0 WHERE id = ?")
                .bind(id)
                .execute(&state.pool)
                .await
                .ok();
            Redirect::to("/ai").into_response()
        }
        _ => Html(render_error("无权删除")).into_response(),
    }
}

// =====================================================================
// 积分兑换
// =====================================================================

pub async fn ai_share_purchase(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let pool = &state.pool;

    let share: Option<AiShare> = sqlx::query_as(
        "SELECT * FROM ai_shares WHERE id = ? AND status = 1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let Some(share) = share else {
        return Html(render_error("内容不存在")).into_response();
    };

    // Can't buy own content
    if share.user_id == user.id {
        return Html(render_error("不能兑换自己的内容")).into_response();
    }

    // Free content
    if share.price == 0 {
        return Html(render_error("免费内容无需兑换")).into_response();
    }

    // Check already purchased
    let already: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM ai_share_purchases WHERE share_id = ? AND user_id = ?"
    )
    .bind(id).bind(user.id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if already.is_some() {
        return Html(render_error("你已经兑换过此内容")).into_response();
    }

    // Check credits
    if user.credits < share.price {
        return Html(render_error(&format!("积分不足，需要 {} 积分，当前 {} 积分", share.price, user.credits))).into_response();
    }

    // Transaction: deduct credits + record purchase + increment download count
    let tx = pool.begin().await;
    let Ok(mut tx) = tx else {
        return Html(render_error("系统错误")).into_response();
    };

    // Deduct credits
    let deduct = sqlx::query("UPDATE users SET credits = credits - ? WHERE id = ? AND credits >= ?")
        .bind(share.price).bind(user.id).bind(share.price)
        .execute(&mut *tx)
        .await;

    match deduct {
        Ok(r) if r.rows_affected() > 0 => {}
        _ => {
            tx.rollback().await.ok();
            return Html(render_error("积分不足")).into_response();
        }
    }

    // Record purchase
    sqlx::query("INSERT INTO ai_share_purchases (share_id, user_id, credits_paid) VALUES (?, ?, ?)")
        .bind(id).bind(user.id).bind(share.price)
        .execute(&mut *tx)
        .await
        .ok();

    // Increment download count
    sqlx::query("UPDATE ai_shares SET download_count = download_count + 1 WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .ok();

    tx.commit().await.ok();

    Redirect::to(&format!("/ai/{}", id)).into_response()
}
