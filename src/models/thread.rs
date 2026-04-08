// 主题帖模型模块
// 定义主题帖（Thread）和主题帖列表项（ThreadList）的数据结构。
// Thread 用于帖子详情页展示，包含通过 JOIN 查询关联的用户名和版块名。
// ThreadList 用于版块内的帖子列表展示，包含作者头像信息。

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// 主题帖实体 —— 对应数据库 threads 表，并包含 JOIN 查询的关联字段
// 用于帖子详情页等需要版块名称的场景
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Thread {
    pub id: i64,                // 主题帖唯一 ID
    pub forum_id: i64,          // 所属版块 ID
    pub user_id: i64,           // 发帖用户 ID
    pub title: String,          // 帖子标题
    pub is_top: i64,            // 是否置顶（1=置顶, 0=普通）
    pub is_closed: i64,         // 是否关闭回复（1=关闭, 0=开放）
    pub is_essence: i64,        // 是否精华帖（1=精华, 0=普通）
    pub view_count: i64,        // 浏览次数
    pub reply_count: i64,       // 回复数量
    pub last_post_at: String,   // 最后回复时间
    pub last_post_user: String, // 最后回复的用户名
    pub created_at: String,     // 创建时间
    pub updated_at: String,     // 更新时间
    // 以下为 JOIN 查询的关联字段
    pub username: Option<String>,   // 发帖用户名（JOIN users 表）
    pub forum_name: Option<String>, // 所属版块名称（JOIN forums 表）
}

// 主题帖列表项 —— 用于版块内的帖子列表展示
// 与 Thread 的区别：不含 forum_name，但包含作者头像
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ThreadList {
    pub id: i64,                // 主题帖唯一 ID
    pub forum_id: i64,          // 所属版块 ID
    pub user_id: i64,           // 发帖用户 ID
    pub title: String,          // 帖子标题
    pub is_top: i64,            // 是否置顶
    pub is_closed: i64,         // 是否关闭回复
    pub is_essence: i64,        // 是否精华帖
    pub view_count: i64,        // 浏览次数
    pub reply_count: i64,       // 回复数量
    pub last_post_at: String,   // 最后回复时间
    pub last_post_user: String, // 最后回复的用户名
    pub created_at: String,     // 创建时间
    pub username: Option<String>, // 发帖用户名（JOIN users 表）
    pub avatar: Option<String>,   // 发帖用户头像路径（JOIN users 表）
}
