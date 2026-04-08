// 回复帖模型模块
// 定义帖子回复（Post）的数据结构，对应数据库 posts 表。
// 除基本帖子信息外，还包含通过 JOIN 查询关联的用户详细信息，
// 如用户名、头像、用户组、签名、头衔和徽章等，用于帖子详情页展示。

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// 回复帖实体 —— 对应数据库 posts 表，并包含 JOIN 关联的用户信息字段
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Post {
    pub id: i64,                // 帖子唯一 ID
    pub thread_id: i64,         // 所属主题帖 ID
    pub forum_id: i64,          // 所属版块 ID
    pub user_id: i64,           // 发帖用户 ID
    pub content: String,        // 帖子内容（Markdown 格式）
    pub floor: i64,             // 楼层数（1 楼为主题帖）
    pub is_first: i64,          // 是否为主题帖首帖（1=首帖, 0=回复）
    pub created_at: String,     // 创建时间
    pub updated_at: String,     // 更新时间（编辑后更新）
    // 以下为 JOIN 查询关联的用户信息字段
    pub username: Option<String>,       // 发帖用户名
    pub avatar: Option<String>,         // 用户头像路径
    pub group_id: Option<i64>,          // 用户组 ID（用于显示身份标识）
    pub signature: Option<String>,      // 用户个人签名
    pub custom_title: Option<String>,   // 用户自定义头衔
    pub epithet: Option<String>,        // 用户称号/徽章文字
    pub epithet_color: Option<String>,  // 用户称号/徽章颜色
    pub user_status: Option<i64>,       // 用户账号状态（0=封禁, 1=正常）
    pub user_muted: Option<String>,     // 禁言原因（NULL=未禁言）
}
