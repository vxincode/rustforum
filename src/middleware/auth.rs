// 认证中间件模块
// 提供 Axum 请求提取器（Extractor），用于从 HTTP 请求中提取并验证用户身份。
// 包含四种级别的认证守卫：
//   - AuthUser:      要求已登录，未登录则重定向到登录页
//   - AdminUser:     要求管理员身份（group_id == 1）
//   - MaybeUser:     可选认证，已登录则获取用户信息，未登录也不拦截
//   - ModeratorUser: 要求版主或管理员身份（group_id 为 1 或 2）

use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{Html, IntoResponse, Redirect, Response},
};

use crate::config::AppState;
use crate::models::forum::Forum;
use crate::models::user::User;

// 已登录用户提取器 —— 要求请求必须携带有效的会话信息
// 若未登录则自动重定向到 /auth/login 页面
pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        // 从请求中提取用户信息
        let user = extract_user_from_request(parts, &state.pool).await;
        match user {
            Some(u) => Ok(AuthUser(u)),
            // 未找到有效会话，重定向到登录页
            None => Err(Redirect::to("/auth/login").into_response()),
        }
    }
}

// 管理员专用守卫 —— 仅允许 group_id == 1 的管理员用户通过
#[allow(dead_code)]
pub struct AdminUser(pub User);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user: Option<User> = extract_user_from_request(parts, &state.pool).await;
        match user {
            Some(u) if u.is_admin() => Ok(AdminUser(u)),
            // 已登录但非管理员，返回 403 权限不足
            Some(_) => Err(Html("403 - 权限不足".to_string()).into_response()),
            // 未登录，重定向到登录页
            None => Err(Redirect::to("/auth/login").into_response()),
        }
    }
}

// 可选用户提取器 —— 无论是否登录都放行
// 页面可以同时为已登录和未登录用户展示不同内容
pub struct MaybeUser(pub Option<User>);

impl FromRequestParts<AppState> for MaybeUser {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = extract_user_from_request(parts, &state.pool).await;
        // 始终返回 Ok，用户信息可能为 None
        Ok(MaybeUser(user))
    }
}

// 版主/管理员守卫 —— 允许 group_id 为 1（管理员）或 2（版主）的用户通过
#[allow(dead_code)]
pub struct ModeratorUser(pub User);

impl FromRequestParts<AppState> for ModeratorUser {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user: Option<User> = extract_user_from_request(parts, &state.pool).await;
        match user {
            // 仅允许管理员（group_id=1）和版主（group_id=2）
            Some(u) if u.group_id == 1 || u.group_id == 2 => Ok(ModeratorUser(u)),
            // 已登录但权限不足
            Some(_) => Err(Html("403 - 权限不足".to_string()).into_response()),
            // 未登录，重定向到登录页
            None => Err(Redirect::to("/auth/login").into_response()),
        }
    }
}

// 从 HTTP 请求中提取用户信息的核心函数
// 解析流程：获取 session_id -> 查询 sessions 表 -> 查询 users 表
async fn extract_user_from_request(parts: &mut Parts, pool: &sqlx::SqlitePool) -> Option<User> {
    // 优先从 X-Session-Id 请求头获取（用于 JSON API 调用），
    // 其次从 cookie 中解析 session_id（用于浏览器请求）
    let session_id = parts.headers.get("X-Session-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            // 从 cookie 中查找 session_id 字段
            let cookies = parts.headers.get("cookie")?.to_str().ok()?;
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix("session_id=").map(|s| s.to_string())
            })
        })?;

    // 根据 session_id 查询对应的用户 ID，同时检查会话是否已过期
    let row: (i64,) = sqlx::query_as(
        "SELECT user_id FROM sessions WHERE id = ? AND expires_at > datetime('now')"
    )
    .bind(&session_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;

    // 根据用户 ID 查询用户完整信息，同时检查用户状态是否正常（status=1）
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ? AND status = 1")
        .bind(row.0)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()?;

    Some(user)
}

// =====================================================================
// 版块权限检查函数
// =====================================================================

/// 检查用户是否可以浏览指定版块（同步快速检查，不含版主表查询）
/// view_perm: 0=所有人, 1=登录用户, 2=版主+, 3=仅管理员
#[allow(dead_code)]
pub fn can_view_forum(forum: &Forum, user: Option<&User>) -> bool {
    match forum.view_perm {
        0 => true,                                    // 所有人可见
        1 => user.is_some(),                          // 需要登录
        2 => user.map_or(false, |u| u.group_id <= 2), // 版主及以上
        3 => user.map_or(false, |u| u.group_id == 1), // 仅管理员
        _ => true,
    }
}

/// 检查用户是否可以浏览指定版块（异步版本，含版主表查询）
/// view_perm=2 时还需检查是否是该版块的版主
pub async fn can_view_forum_async(
    pool: &sqlx::SqlitePool,
    forum: &Forum,
    user: Option<&User>,
) -> bool {
    match forum.view_perm {
        0 => true,
        1 => user.is_some(),
        2 => {
            if let Some(u) = user {
                if u.group_id <= 2 { return true; }
                is_forum_moderator(pool, forum.id, u.id).await
            } else {
                false
            }
        }
        3 => user.map_or(false, |u| u.group_id == 1),
        _ => true,
    }
}

/// 检查用户是否可以在指定版块发帖
/// post_perm: 0=所有用户, 1=版主+, 2=仅管理员
pub async fn can_post_in_forum(
    pool: &sqlx::SqlitePool,
    forum: &Forum,
    user: &User,
) -> bool {
    match forum.post_perm {
        0 => true,                                    // 所有用户可发帖
        1 => {
            // 版主及以上或该版块的指定版主
            if user.group_id <= 2 { return true; }
            is_forum_moderator(pool, forum.id, user.id).await
        }
        2 => user.group_id == 1,                      // 仅管理员
        _ => true,
    }
}

/// 检查用户是否是指定版块的版主
pub async fn is_forum_moderator(
    pool: &sqlx::SqlitePool,
    forum_id: i64,
    user_id: i64,
) -> bool {
    let result: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM forum_moderators WHERE forum_id = ? AND user_id = ?"
    )
    .bind(forum_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    result.is_some()
}

/// 检查用户是否被禁言（包括已过期的禁言记录也会被忽略）
/// 返回 Some(原因) 如果用户被禁言，None 如果未被禁言
pub async fn get_mute_status(pool: &sqlx::SqlitePool, user_id: i64) -> Option<String> {
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT reason, expires_at FROM muted_users WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let (reason, expires_at) = match row {
        Some(r) => r,
        None => return None,
    };

    // 如果有过期时间，检查是否已过期
    if let Some(ref exp) = &expires_at {
        let now: (String,) = sqlx::query_as("SELECT datetime('now')")
            .fetch_one(pool)
            .await
            .ok()?;
       if *exp < now.0 {
            // 已过期，删除记录并返回 None
            sqlx::query("DELETE FROM muted_users WHERE user_id = ?")
                .bind(user_id)
                .execute(pool)
                .await
                .ok();
            return None;
        }
    }

    let display_reason = if reason.is_empty() { "管理员禁言".to_string() } else { reason };
    let expires_text = match expires_at {
        Some(t) => format!("（到期：{}）", t.chars().take(16).collect::<String>()),
        None => "（永久禁言）".to_string(),
    };
    Some(format!("你已被禁言：{}{}", display_reason, expires_text))
}
