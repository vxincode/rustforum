// 登录频率限制模块
// 基于 IP 地址限制登录失败次数，防止暴力破解密码。
// 规则：同一 IP 在 5 分钟内最多允许 5 次登录失败尝试。
// 使用内存中的 HashMap 存储失败记录（进程重启后清空）。

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

// 全局登录失败记录表
// Key: IP 地址, Value: (失败次数, 首次失败时间)
static LOGIN_ATTEMPTS: LazyLock<Mutex<HashMap<String, (u32, Instant)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// 每个时间窗口内允许的最大失败次数
const MAX_ATTEMPTS: u32 = 5;
// 时间窗口长度（秒），即 5 分钟
const WINDOW_SECS: u64 = 300;

// 记录一次登录失败
// 如果距离首次失败已超过时间窗口，则重新计数；否则累加失败次数
pub fn record_failed_login(ip: &str) {
    let mut map = LOGIN_ATTEMPTS.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    let entry = map.entry(ip.to_string()).or_insert((0, now));
    // 超过时间窗口，重置计数器
    if now.duration_since(entry.1).as_secs() > WINDOW_SECS {
        *entry = (1, now);
    } else {
        // 在时间窗口内，累加失败次数
        entry.0 += 1;
    }
}

// 清除指定 IP 的登录失败记录（登录成功时调用）
pub fn clear_login_attempts(ip: &str) {
    let mut map = LOGIN_ATTEMPTS.lock().unwrap_or_else(|e| e.into_inner());
    map.remove(ip);
}

// 检查指定 IP 是否被限制登录
// 返回 Ok(剩余尝试次数) 表示未被限制
// 返回 Err(已尝试次数) 表示已被限制
pub fn check_login_rate(ip: &str) -> Result<u32, u32> {
    let map = LOGIN_ATTEMPTS.lock().unwrap_or_else(|e| e.into_inner());
    match map.get(ip) {
        Some((count, first)) => {
            // 时间窗口已过，重置为允许最大次数
            if Instant::now().duration_since(*first).as_secs() > WINDOW_SECS {
                Ok(MAX_ATTEMPTS)
            // 失败次数达到上限，拒绝登录
            } else if *count >= MAX_ATTEMPTS {
                Err(*count)
            // 未达上限，返回剩余可用次数
            } else {
                Ok(MAX_ATTEMPTS - *count)
            }
        }
        // 该 IP 没有失败记录，返回最大可用次数
        None => Ok(MAX_ATTEMPTS),
    }
}

// 从 HTTP 请求头中提取客户端真实 IP 地址
// 支持反向代理场景下的 X-Forwarded-For 和 X-Real-IP 头
pub fn extract_ip_from_headers(headers: &axum::http::HeaderMap) -> String {
    // 优先读取 X-Forwarded-For 头（取第一个 IP，即最原始的客户端 IP）
    if let Some(xff) = headers.get("x-forwarded-for") {
        if let Ok(val) = xff.to_str() {
            if let Some(ip) = val.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }
    // 其次读取 X-Real-IP 头（Nginx 等代理常用）
    if let Some(xri) = headers.get("x-real-ip") {
        if let Ok(val) = xri.to_str() {
            return val.trim().to_string();
        }
    }
    // 无法识别 IP 时返回 "unknown"
    "unknown".to_string()
}
