// 全局站点设置缓存模块
// 职责：将数据库中的站点设置缓存到全局静态变量中，避免每次渲染页面都查询数据库
// 在应用启动时和管理员保存设置时会刷新缓存
// 模板渲染时通过 site_name()、site_description() 等函数获取当前设置

use std::sync::RwLock;
use sqlx::SqlitePool;

// 全局静态变量，使用 RwLock 实现读写锁
// 启动时从数据库加载，管理员修改设置后也会刷新这些值
/// Global site settings cache, refreshed from DB on startup and admin save.
pub static SITE_NAME: RwLock<String> = RwLock::new(String::new());          // 站点名称
pub static SITE_DESCRIPTION: RwLock<String> = RwLock::new(String::new());   // 站点描述（用于 meta 标签）
pub static SITE_KEYWORDS: RwLock<String> = RwLock::new(String::new());      // SEO 关键词
pub static SITE_FOOTER: RwLock<String> = RwLock::new(String::new());        // 页脚文字
pub static SETUP_COMPLETED: RwLock<bool> = RwLock::new(false);              // 安装是否完成

// 从数据库加载站点设置到全局缓存
// 读取 settings 表中所有键值对，更新对应的 RwLock 全局变量
pub async fn load_from_db(pool: &SqlitePool) {
    // 查询所有设置项
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    // 将查询结果转换为 HashMap 方便按 key 查找
    let map: std::collections::HashMap<String, String> = rows.into_iter().collect();

    // 逐个更新全局缓存变量
    {
        let val = map.get("site_name").cloned().unwrap_or_default();
        if let Ok(mut g) = SITE_NAME.write() { *g = val; }
    }
    {
        let val = map.get("site_description").cloned().unwrap_or_default();
        if let Ok(mut g) = SITE_DESCRIPTION.write() { *g = val; }
    }
    {
        let val = map.get("site_keywords").cloned().unwrap_or_default();
        if let Ok(mut g) = SITE_KEYWORDS.write() { *g = val; }
    }
    {
        let val = map.get("site_footer_text").cloned().unwrap_or_default();
        if let Ok(mut g) = SITE_FOOTER.write() { *g = val; }
    }
    {
        let val = map.get("setup_completed").cloned().unwrap_or_default();
        if let Ok(mut g) = SETUP_COMPLETED.write() { *g = val == "1"; }
    }
}

// 获取当前站点名称（供模板渲染使用）
pub fn site_name() -> String {
    SITE_NAME.read().map(|g| g.clone()).unwrap_or_default()
}

// 获取当前站点描述（用于 HTML meta description 标签）
pub fn site_description() -> String {
    SITE_DESCRIPTION.read().map(|g| g.clone()).unwrap_or_default()
}

// 获取当前站点关键词（用于 HTML meta keywords 标签）
pub fn site_keywords() -> String {
    SITE_KEYWORDS.read().map(|g| g.clone()).unwrap_or_default()
}

// 获取当前站点页脚文字（供模板渲染使用）
pub fn site_footer() -> String {
    SITE_FOOTER.read().map(|g| g.clone()).unwrap_or_default()
}

// Check if setup wizard has been completed
pub fn is_setup_completed() -> bool {
    SETUP_COMPLETED.read().map(|g| *g).unwrap_or(false)
}

// Set setup completed state (called by setup handler)
pub fn set_setup_completed(val: bool) {
    if let Ok(mut g) = SETUP_COMPLETED.write() { *g = val; }
}
