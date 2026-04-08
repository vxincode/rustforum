// 数据模型模块声明
// 定义与数据库表对应的结构体，用于 SQLx 查询结果的类型映射
// 每个模块包含对应表的模型定义和相关的查询辅助方法

pub mod user;          // 用户模型：用户基本信息、组、状态
pub mod forum;         // 版块模型：版块名称、描述、帖子统计
pub mod thread;        // 主题帖模型：标题、置顶、精华、回复数
pub mod post;          // 回复模型：内容、楼层、是否首帖
pub mod message;       // 私信模型：发送者、接收者、内容、已读状态
pub mod notification;  // 通知模型：通知类型（回复/引用/私信）、已读状态
pub mod report;        // 举报模型：举报原因、处理状态
pub mod blacklist;     // 黑名单模型：IP/用户封禁记录
pub mod forum_moderator; // 版块版主模型：版主分配记录
pub mod ai_share;      // AI 共享模型：Prompt/Skill 分享
