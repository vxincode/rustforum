// 请求处理器模块声明
// 按功能划分各个子模块，每个模块包含对应的路由处理函数
// 所有处理器通过 main.rs 中的路由注册与 URL 路径绑定

pub mod auth;          // 用户认证：登录、注册、退出
pub mod forum;         // 版块管理：版块列表、版块详情、发帖
pub mod thread;        // 主题帖：查看帖子、回复、编辑、删除
pub mod admin;         // 管理后台：仪表盘、版块/用户/帖子管理、系统设置
pub mod index;         // 首页处理器
pub mod api;           // JSON API 接口：供前端 JS 调用的数据接口
pub mod profile;       // 用户资料：个人资料查看与编辑、密码修改
pub mod avatar;        // 头像管理：上传和删除头像
pub mod message;       // 私信功能：收件箱、发送私信、对话查看
pub mod notification;  // 通知系统：通知列表、标记已读、行内编辑帖子
pub mod checkin;       // 签到功能：每日签到、签到状态、排行榜
pub mod report;        // 举报功能：提交内容举报
pub mod about;         // 静态页面：关于、条款、隐私、联系
pub mod backup;        // 备份恢复：数据库备份、下载、恢复、删除
pub mod ai_share;      // AI 共享：Prompt/Skill 分享与兑换
pub mod setup;         // 安装向导：首次运行时的引导设置
