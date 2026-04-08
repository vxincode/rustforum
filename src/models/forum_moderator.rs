// 版块版主模型模块
// 定义版块版主（ForumModerator）的数据结构，对应数据库 forum_moderators 表。

use sqlx::FromRow;

// 版块版主实体 —— 对应数据库 forum_moderators 表
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct ForumModerator {
    pub id: i64,
    pub forum_id: i64,
    pub user_id: i64,
    pub created_at: String,
}

// 版块版主（含用户名） —— 用于后台展示
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct ForumModeratorWithUser {
    pub id: i64,
    pub forum_id: i64,
    pub user_id: i64,
    pub username: String,
    pub created_at: String,
}
