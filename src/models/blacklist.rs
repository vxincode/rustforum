// 黑名单/禁言模型模块
// 定义三类数据结构：
//   - BlacklistEntry: IP/邮箱黑名单条目，用于阻止特定 IP 或邮箱注册/访问
//   - MutedUser: 禁言用户记录，记录被禁言的用户、原因、过期时间
//   - MutedUserWithInfo: 带关联用户名信息的禁言记录，用于管理后台展示

use sqlx::FromRow;

// 黑名单条目 —— 对应数据库 blacklist 表
// 用于封禁特定 IP 地址或邮箱地址
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct BlacklistEntry {
    pub id: i64,                // 记录唯一 ID
    pub r#type: String,         // 黑名单类型（如 "ip"、"email"）
    pub value: String,          // 黑名单值（具体的 IP 地址或邮箱）
    pub reason: String,         // 加入黑名单的原因
    pub admin_id: Option<i64>,  // 执行封禁的管理员 ID
    pub created_at: String,     // 加入黑名单的时间
}

// 禁言用户记录 —— 对应数据库 muted_users 表
// 记录被禁言的用户信息，支持设置禁言过期时间
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct MutedUser {
    pub id: i64,                    // 记录唯一 ID
    pub user_id: i64,               // 被禁言的用户 ID
    pub reason: String,             // 禁言原因
    pub admin_id: Option<i64>,      // 执行禁言操作的管理员 ID
    pub expires_at: Option<String>, // 禁言到期时间（None 表示永久禁言）
    pub created_at: String,         // 禁言操作的时间
}

// 带用户名信息的禁言记录 —— 用于管理后台展示禁言列表
// 在 MutedUser 基础上通过 JOIN 查询增加了用户名和管理员名
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct MutedUserWithInfo {
    pub id: i64,                    // 记录唯一 ID
    pub user_id: i64,               // 被禁言的用户 ID
    pub reason: String,             // 禁言原因
    pub admin_id: Option<i64>,      // 执行禁言操作的管理员 ID
    pub expires_at: Option<String>, // 禁言到期时间
    pub created_at: String,         // 禁言操作的时间
    // 以下为 JOIN 查询关联的扩展字段
    pub username: String,           // 被禁言用户的用户名（JOIN users 表）
    pub admin_name: Option<String>, // 执行禁言的管理员用户名（JOIN users 表）
}
