// 版块模型模块
// 定义论坛版块（Forum）的数据结构，对应数据库 forums 表。
// 包含版块基本信息、帖子统计数和最后发帖信息，用于首页版块列表展示。

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// 版块实体 —— 对应数据库 forums 表
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Forum {
    pub id: i64,                    // 版块唯一 ID
    pub name: String,               // 版块名称
    pub description: String,        // 版块描述/简介
    pub sort_order: i64,            // 排序权重（数值越小越靠前）
    pub parent_id: Option<i64>,     // 父版块 ID（None 表示顶级版块）
    pub thread_count: i64,          // 主题帖数量
    pub post_count: i64,            // 回复帖数量（含主题帖）
    pub last_thread_id: Option<i64>, // 最新主题帖 ID
    pub last_post_at: Option<String>, // 最后发帖时间
    pub last_post_user: String,     // 最后发帖的用户名
    pub status: i64,                // 版块状态（1=正常, 0=关闭）
    pub created_at: String,         // 创建时间
    pub view_perm: i64,             // 浏览权限（0=所有人, 1=登录用户, 2=版主+, 3=仅管理员）
    pub post_perm: i64,             // 发帖权限（0=所有用户, 1=版主+, 2=仅管理员）
}
