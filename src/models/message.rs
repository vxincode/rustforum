// 私信模型模块
// 定义站内私信（Message）的数据结构以及发送和回复私信的表单。
// 包含发送者和接收者的关联信息，用于私信收件箱和对话页面展示。

use serde::Deserialize;
use sqlx::FromRow;

// 私信实体 —— 对应数据库 messages 表，并包含 JOIN 关联的用户信息
#[derive(Debug, Clone, FromRow)]
pub struct Message {
    #[allow(dead_code)]
    pub id: i64,                        // 私信唯一 ID
    pub sender_id: i64,                 // 发送者用户 ID
    pub receiver_id: i64,               // 接收者用户 ID
    pub content: String,                // 私信内容
    pub is_read: i64,                   // 是否已读（1=已读, 0=未读）
    pub created_at: String,             // 发送时间
    pub sender_name: Option<String>,    // 发送者用户名（JOIN users 表）
    pub sender_avatar: Option<String>,  // 发送者头像路径（JOIN users 表）
    pub receiver_name: Option<String>,  // 接收者用户名（JOIN users 表）
}

// 发送私信表单 —— 从"撰写新私信"页面的 POST 请求中反序列化
#[derive(Debug, Deserialize)]
pub struct SendMessageForm {
    pub to: String,       // 收件人用户名
    pub content: String,  // 私信内容
}

// 回复私信表单 —— 从对话页面回复消息时的 POST 请求中反序列化
#[derive(Debug, Deserialize)]
pub struct ReplyMessageForm {
    pub content: String,  // 回复内容
}
