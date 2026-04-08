// 签到系统处理器：包含签到、签到状态查询、积分排行榜、新用户列表、友情链接 API
// 签到支持连续签到奖励机制，连续签到天数越多获得的积分越高

use axum::{
    extract::State,
    response::{IntoResponse, Json},
    http::StatusCode,
};
use serde_json::{json, Value};

use crate::config::AppState;
use crate::middleware::auth::{AuthUser, MaybeUser};
use crate::models::user::User;

// =====================================================================
// 签到 API：需登录，每天只能签到一次，支持连续签到奖励
// =====================================================================

pub async fn api_checkin(
    AuthUser(user): AuthUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let pool = &state.pool;

    // 检查今天是否已经签到过
    let today: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM checkins WHERE user_id = ? AND checkin_date = date('now')"
    )
    .bind(user.id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    if today.is_some() {
        return (StatusCode::OK, Json(json!({"ok": false, "error": "今日已签到"})));
    }

    // 检查昨天是否签到过，用于计算连续签到天数
    let yesterday: Option<(i64,)> = sqlx::query_as(
        "SELECT streak FROM checkins WHERE user_id = ? AND checkin_date = date('now', '-1 day') ORDER BY created_at DESC LIMIT 1"
    )
    .bind(user.id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let streak = match yesterday {
        Some((prev_streak,)) => prev_streak + 1,
        None => 1,
    };

    // 根据连续签到天数计算获得的积分：连续 7 天以上 15 分，3-6 天 10 分，2 天 7 分，1 天 5 分
    let credits = if streak >= 7 {
        15 // 5 base + 10 bonus
    } else if streak >= 3 {
        10 // 5 base + 5 bonus
    } else if streak >= 2 {
        7 // 5 base + 2 bonus
    } else {
        5
    };

    // 插入签到记录
    sqlx::query(
        "INSERT INTO checkins (user_id, credits_gained, streak) VALUES (?, ?, ?)"
    )
    .bind(user.id)
    .bind(credits)
    .bind(streak)
    .execute(pool)
    .await
    .ok();

    // 更新用户积分
    sqlx::query("UPDATE users SET credits = credits + ? WHERE id = ?")
        .bind(credits)
        .bind(user.id)
        .execute(pool)
        .await
        .ok();

    // 查询更新后的用户积分
    let updated: Option<(i64,)> = sqlx::query_as("SELECT credits FROM users WHERE id = ?")
        .bind(user.id)
        .fetch_one(pool)
        .await
        .ok();

    (StatusCode::OK, Json(json!({
        "ok": true,
        "credits_gained": credits,
        "streak": streak,
        "total_credits": updated.map(|(c,)| c).unwrap_or(0),
    })))
}

// =====================================================================
// 签到状态 API：登录可选，返回今日是否已签到、连续签到天数、当前积分
// =====================================================================

pub async fn api_checkin_status(
    MaybeUser(user): MaybeUser,
    state: State<AppState>,
) -> Json<Value> {
    let Some(user) = user else {
        return Json(json!({"ok": true, "checked_in": false, "streak": 0, "credits": 0}));
    };

    let pool = &state.pool;

    let today: Option<(i64, i64)> = sqlx::query_as(
        "SELECT streak, credits_gained FROM checkins WHERE user_id = ? AND checkin_date = date('now')"
    )
    .bind(user.id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let last_streak: Option<(i64,)> = sqlx::query_as(
        "SELECT streak FROM checkins WHERE user_id = ? ORDER BY created_at DESC LIMIT 1"
    )
    .bind(user.id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    Json(json!({
        "ok": true,
        "checked_in": today.is_some(),
        "streak": today.map(|(s, _)| s).or(last_streak.map(|(s,)| s)).unwrap_or(0),
        "credits": user.credits,
    }))
}

// =====================================================================
// 积分排行榜 API：返回积分前 10 名的用户及对应等级称号
// =====================================================================

// 排行榜用户数据结构
#[derive(sqlx::FromRow)]
struct LeaderEntry {
    id: i64,
    username: String,
    avatar: String,
    credits: i64,
}

pub async fn api_leaderboard(state: State<AppState>) -> Json<Value> {
    // Try Redis cache
    if let Some(cached) = crate::cache::get_cached(&state.redis, "api:leaderboard").await {
        if let Ok(val) = serde_json::from_str::<Value>(&cached) {
            return Json(val);
        }
    }

    let users: Vec<LeaderEntry> = sqlx::query_as(
        "SELECT id, username, avatar, credits FROM users WHERE status = 1 ORDER BY credits DESC, id ASC LIMIT 10"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let result = json!({
        "users": users.iter().map(|u| {
            let rank_title = match u.credits {
                0..=49 => "新手上路",
                50..=199 => "初级会员",
                200..=499 => "中级会员",
                500..=999 => "高级会员",
                1000..=1999 => "资深会员",
                2000..=4999 => "钻石会员",
                _ => "传奇会员",
            };
            json!({
                "id": u.id,
                "username": u.username,
                "avatar": u.avatar,
                "credits": u.credits,
                "rank_title": rank_title,
            })
        }).collect::<Vec<_>>(),
    });

    crate::cache::set_cached(&state.redis, "api:leaderboard", &result.to_string(), 300).await;
    Json(result)
}

// =====================================================================
// 新用户列表 API：返回最近注册的 5 个用户
// =====================================================================

pub async fn api_new_users(state: State<AppState>) -> Json<Value> {
    // Try Redis cache
    if let Some(cached) = crate::cache::get_cached(&state.redis, "api:users:recent").await {
        if let Ok(val) = serde_json::from_str::<Value>(&cached) {
            return Json(val);
        }
    }

    let users: Vec<User> = sqlx::query_as(
        "SELECT * FROM users WHERE status = 1 ORDER BY created_at DESC LIMIT 5"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let result = json!({
        "users": users.iter().map(|u| json!({
            "id": u.id,
            "username": u.username,
            "avatar": u.avatar,
            "rank_title": u.rank_title(),
            "created_at": u.created_at.chars().take(10).collect::<String>(),
        })).collect::<Vec<_>>(),
    });

    crate::cache::set_cached(&state.redis, "api:users:recent", &result.to_string(), 300).await;
    Json(result)
}

// =====================================================================
// 友情链接 API：返回所有友情链接，按排序字段排列
// =====================================================================

// 友情链接数据结构
#[derive(sqlx::FromRow)]
struct FriendlyLink {
    name: String,
    url: String,
}

pub async fn api_friendly_links(state: State<AppState>) -> Json<Value> {
    // Try Redis cache
    if let Some(cached) = crate::cache::get_cached(&state.redis, "api:links").await {
        if let Ok(val) = serde_json::from_str::<Value>(&cached) {
            return Json(val);
        }
    }

    let links: Vec<FriendlyLink> = sqlx::query_as(
        "SELECT name, url FROM friendly_links ORDER BY sort_order ASC"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let result = json!({
        "links": links.iter().map(|l| json!({
            "name": l.name,
            "url": l.url,
        })).collect::<Vec<_>>(),
    });

    crate::cache::set_cached(&state.redis, "api:links", &result.to_string(), 600).await;
    Json(result)
}
