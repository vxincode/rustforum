// Redis 缓存辅助模块
// 提供 get_cached / set_cached / invalidate 三个核心函数
// 所有缓存键统一使用 "cache:" 前缀，支持 TTL 过期

use redis::AsyncCommands;

/// 从 Redis 读取缓存，返回 None 表示未命中或 Redis 不可用
pub async fn get_cached(redis: &Option<redis::aio::ConnectionManager>, key: &str) -> Option<String> {
    let cm = redis.as_ref()?;
    let full_key = format!("cache:{}", key);
    match cm.clone().get::<_, String>(&full_key).await {
        Ok(val) => {
            tracing::debug!("cache HIT: {}", full_key);
            Some(val)
        }
        Err(_) => {
            tracing::debug!("cache MISS: {}", full_key);
            None
        }
    }
}

/// 写入 Redis 缓存并设置 TTL（秒）
pub async fn set_cached(redis: &Option<redis::aio::ConnectionManager>, key: &str, value: &str, ttl_secs: u64) {
    let Some(cm) = redis.as_ref() else { return };
    let full_key = format!("cache:{}", key);
    if let Err(e) = cm.clone().set_ex::<_, _, ()>(&full_key, value, ttl_secs).await {
        tracing::warn!("cache SET failed for {}: {}", full_key, e);
    }
}

/// 删除缓存键（可传多个 key 一次性清除）
pub async fn invalidate(redis: &Option<redis::aio::ConnectionManager>, keys: &[&str]) {
    let Some(cm) = redis.as_ref() else { return };
    let full_keys: Vec<String> = keys.iter().map(|k| format!("cache:{}", k)).collect();
    if let Err(e) = cm.clone().del::<_, ()>(&full_keys).await {
        tracing::warn!("cache DEL failed: {}", e);
    }
}
