// CSRF（跨站请求伪造）防护模块
// 基于 bcrypt 算法为每个会话生成唯一的 CSRF 令牌。
// 工作原理：
//   1. 令牌由 CSRF_SECRET + session_id 经 bcrypt 哈希生成
//   2. 令牌通过 <meta name="csrf-token"> 嵌入每个页面
//   3. 前端 JS 自动将令牌注入表单和 fetch 请求中
//   4. 服务端验证令牌时使用常量时间比较，防止时序攻击

use axum::http::HeaderMap;

// CSRF 令牌生成所使用的密钥前缀
const CSRF_SECRET: &str = "rustforum-csrf-secret";

// 根据 session_id 生成 CSRF 令牌
// 使用 bcrypt 算法（cost=4，较低成本以换取速度）对 "密钥:会话ID" 进行哈希
// 若 bcrypt 失败则回退为随机 UUID
pub fn generate_token(session_id: &str) -> String {
    // 将密钥和会话 ID 拼接后进行哈希
    let input = format!("{}:{}", CSRF_SECRET, session_id);
    match bcrypt::hash(&input, 4) {
        Ok(hash) => hash,
        // bcrypt 失败时使用 UUID 作为兜底令牌
        Err(_) => uuid::Uuid::new_v4().to_string(),
    }
}

// 验证 CSRF 令牌是否有效
// 使用常量时间比较（constant-time comparison）防止时序攻击
#[allow(dead_code)]
pub fn validate_token(token: &str, session_id: &str) -> bool {
    // 空令牌或空会话 ID 直接拒绝
    if token.is_empty() || session_id.is_empty() {
        return false;
    }
    // 重新生成期望的令牌
    let expected = generate_token(session_id);
    // 长度不一致直接返回 false（避免后续逐字节比较时泄露长度信息）
    if token.len() != expected.len() {
        return false;
    }
    // 常量时间比较：对所有字节进行异或，结果为 0 则表示完全匹配
    let mut result = 0u8;
    for (a, b) in token.bytes().zip(expected.bytes()) {
        result |= a ^ b;
    }
    result == 0
}

// 从 HTTP 请求的 cookie 头中提取 session_id
#[allow(dead_code)]
pub fn get_session_id(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get("cookie")?.to_str().ok()?;
    // 遍历所有 cookie，查找名为 session_id 的条目
    cookie_header.split(';').find_map(|c| {
        let c = c.trim();
        c.strip_prefix("session_id=").map(|s| s.to_string())
    })
}
