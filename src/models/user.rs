// 用户模型模块
// 定义用户相关的数据结构，包括用户实体、注册表单、登录表单，
// 以及用户组名称、等级头衔、徽章等辅助方法。

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// 用户实体 —— 对应数据库 users 表
// 包含用户的完整信息，支持序列化/反序列化和从数据库行直接映射
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,               // 用户唯一 ID
    pub username: String,      // 用户名
    pub email: String,         // 邮箱地址
    pub password_hash: String, // bcrypt 加密后的密码哈希
    pub avatar: String,        // 头像 URL 路径
    pub signature: String,     // 个人签名
    pub group_id: i64,         // 用户组 ID（1=管理员, 2=版主, 3=会员）
    pub post_count: i64,       // 发帖总数
    pub thread_count: i64,     // 主题帖总数
    pub credits: i64,          // 积分（用于计算等级）
    pub status: i64,           // 账号状态（1=正常, 0=禁用）
    pub custom_title: String,  // 自定义头衔（用户可自行设置）
    pub epithet: String,       // 称号/徽章文字
    pub epithet_color: String, // 称号/徽章颜色
    pub last_login_ip: String, // 最近一次登录 IP
    pub last_login_at: String, // 最近一次登录时间
    pub last_login_ua: String, // 最近一次登录的浏览器 User-Agent
    pub created_at: String,    // 注册时间
    pub updated_at: String,    // 信息更新时间
}

impl User {
    // 判断是否为管理员（group_id == 1）
    pub fn is_admin(&self) -> bool {
        self.group_id == 1
    }

    // 根据 group_id 返回用户组名称
    pub fn group_name(&self) -> &str {
        match self.group_id {
            1 => "管理员",
            2 => "版主",
            _ => "会员",
        }
    }

    // 根据积分区间返回用户等级头衔
    pub fn rank_title(&self) -> &str {
        match self.credits {
            0..=49 => "新手上路",
            50..=199 => "初级会员",
            200..=499 => "中级会员",
            500..=999 => "高级会员",
            1000..=1999 => "资深会员",
            2000..=4999 => "钻石会员",
            _ => "传奇会员",
        }
    }

    // 获取显示用的头衔：优先使用自定义头衔，否则使用积分等级头衔
    pub fn display_title(&self) -> &str {
        if !self.custom_title.is_empty() {
            &self.custom_title
        } else {
            self.rank_title()
        }
    }

    // 生成称号/徽章的 HTML 标签
    // 返回带有渐变背景色的圆形徽章 span 元素
    // 若用户没有设置称号则返回空字符串
    pub fn epithet_badge(&self) -> String {
        if self.epithet.is_empty() {
            String::new()
        } else {
            // 使用自定义颜色，未设置则默认为紫色 #8B5CF6
            let color = if self.epithet_color.is_empty() {
                "#8B5CF6".to_string()
            } else {
                self.epithet_color.clone()
            };
            format!(
                r#"<span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-bold shadow-sm" style="background:linear-gradient(135deg,{},{});color:white;letter-spacing:0.05em">{}</span>"#,
                color, color, html_escape_builtin(&self.epithet)
            )
        }
    }
}

// HTML 特殊字符转义工具函数
// 防止 XSS 攻击，将 & < > " 转义为 HTML 实体
fn html_escape_builtin(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// 用户注册表单 —— 从注册页面的 POST 请求中反序列化
#[derive(Debug, Deserialize)]
pub struct RegisterForm {
    pub username: String,           // 用户名
    pub email: String,              // 邮箱
    pub password: String,           // 密码
    pub password_confirm: String,   // 确认密码
    pub invite_code: Option<String>, // 邀请码（可选）
}

// 用户登录表单 —— 从登录页面的 POST 请求中反序列化
#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String, // 用户名
    pub password: String, // 密码
}
