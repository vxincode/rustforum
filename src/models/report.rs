// 举报模型模块
// 定义举报记录（Report）和带举报人信息的举报记录（ReportWithReporter）。
// Report 对应数据库 reports 表的基本字段，
// ReportWithReporter 在此基础上通过 JOIN 查询关联举报人用户名和被举报内容。

use sqlx::FromRow;

// 举报记录实体 —— 对应数据库 reports 表
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct Report {
    pub id: i64,                    // 举报记录唯一 ID
    pub reporter_id: i64,           // 举报人用户 ID
    pub target_type: String,        // 举报目标类型（如 "post"、"thread"）
    pub target_id: i64,             // 举报目标的 ID
    pub reason: String,             // 举报原因（简短分类）
    pub description: String,        // 举报详细描述
    pub status: String,             // 处理状态（如 "pending"、"resolved"、"dismissed"）
    pub admin_id: Option<i64>,      // 处理该举报的管理员 ID（未处理时为 None）
    pub admin_note: String,         // 管理员处理备注
    pub created_at: String,         // 举报提交时间
    pub resolved_at: Option<String>, // 处理完成时间（未处理时为 None）
}

// 带举报人信息的举报记录 —— 用于管理后台展示举报列表
// 在 Report 基础上增加了举报人用户名和被举报内容
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct ReportWithReporter {
    pub id: i64,                    // 举报记录唯一 ID
    pub reporter_id: i64,           // 举报人用户 ID
    pub target_type: String,        // 举报目标类型
    pub target_id: i64,             // 举报目标的 ID
    pub reason: String,             // 举报原因
    pub description: String,        // 举报详细描述
    pub status: String,             // 处理状态
    pub admin_id: Option<i64>,      // 处理管理员 ID
    pub admin_note: String,         // 管理员处理备注
    pub created_at: String,         // 举报提交时间
    pub resolved_at: Option<String>, // 处理完成时间
    // 以下为 JOIN 查询关联的扩展字段
    pub reporter_name: String,          // 举报人用户名（JOIN users 表）
    pub target_title: Option<String>,   // 被举报内容的标题（JOIN posts/threads 表）
    pub target_content: Option<String>, // 被举报内容的正文
}
