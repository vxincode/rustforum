// 邮件发送模块
// 支持两种发送方式：SendFlare API 和 SMTP
// 通过后台设置切换，提供测试接口验证配置是否正确

use sqlx::SqlitePool;

// 从数据库读取邮件相关设置
async fn get_email_settings(pool: &SqlitePool) -> EmailSettings {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM settings WHERE key LIKE 'email_%'"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    let map: std::collections::HashMap<String, String> = rows.into_iter().collect();
    EmailSettings {
        provider: map.get("email_provider").cloned().unwrap_or_else(|| "smtp".to_string()),
        enabled: map.get("email_enabled").cloned().unwrap_or_else(|| "0".to_string()) == "1",
        from_name: map.get("email_from_name").cloned().unwrap_or_default(),
        from_address: map.get("email_from_address").cloned().unwrap_or_default(),
        // SendFlare
        sendflare_api_url: map.get("email_sendflare_api_url").cloned().unwrap_or_else(|| "https://api.sendflare.com".to_string()),
        sendflare_api_key: map.get("email_sendflare_api_key").cloned().unwrap_or_default(),
        // SMTP
        smtp_host: map.get("email_smtp_host").cloned().unwrap_or_default(),
        smtp_port: map.get("email_smtp_port").cloned().unwrap_or_else(|| "465".to_string()),
        smtp_username: map.get("email_smtp_username").cloned().unwrap_or_default(),
        smtp_password: map.get("email_smtp_password").cloned().unwrap_or_default(),
        smtp_encryption: map.get("email_smtp_encryption").cloned().unwrap_or_else(|| "tls".to_string()),
    }
}

struct EmailSettings {
    provider: String,
    enabled: bool,
    from_name: String,
    from_address: String,
    // SendFlare
    sendflare_api_url: String,
    sendflare_api_key: String,
    // SMTP
    smtp_host: String,
    smtp_port: String,
    smtp_username: String,
    smtp_password: String,
    smtp_encryption: String,
}

// 发送邮件（自动根据配置选择 SendFlare 或 SMTP）
pub async fn send_email(
    pool: &SqlitePool,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    let settings = get_email_settings(pool).await;
    if !settings.enabled {
        return Err("邮件服务未启用".to_string());
    }
    if settings.from_address.is_empty() {
        return Err("发件人地址未配置".to_string());
    }
    match settings.provider.as_str() {
        "sendflare" => send_via_sendflare(&settings, to, subject, body).await,
        _ => send_via_smtp(&settings, to, subject, body).await,
    }
}

// 通过 SendFlare API 发送邮件
async fn send_via_sendflare(
    settings: &EmailSettings,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    if settings.sendflare_api_key.is_empty() {
        return Err("SendFlare API Key 未配置".to_string());
    }
    let url = format!("{}/v1/send", settings.sendflare_api_url.trim_end_matches('/'));
    let from = if settings.from_name.is_empty() {
        settings.from_address.clone()
    } else {
        format!("{} <{}>", settings.from_name, settings.from_address)
    };
    let payload = serde_json::json!({
        "from": from,
        "to": to,
        "subject": subject,
        "body": body,
    });
    let client = reqwest::Client::new();
    let resp = client.post(&url)
        .header("Authorization", format!("Bearer {}", settings.sendflare_api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;
    let status = resp.status().as_u16();
    let text = resp.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
        let success = json.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        let code = json.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        let message = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
        if success && code == 0 {
            Ok(())
        } else {
            Err(format!("SendFlare 返回错误 (HTTP {}): {}", status, message))
        }
    } else {
        Err(format!("SendFlare 返回异常响应 (HTTP {}): {}", status, &text[..text.len().min(200)]))
    }
}

// 通过 SMTP 发送邮件
async fn send_via_smtp(
    settings: &EmailSettings,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    use lettre::{SmtpTransport, Transport, message::{header::ContentType, Mailbox}};
    use lettre::transport::smtp::authentication::Credentials;

    if settings.smtp_host.is_empty() {
        return Err("SMTP 主机未配置".to_string());
    }
    let port: u16 = settings.smtp_port.parse().unwrap_or(465);
    let from_mailbox: Mailbox = if settings.from_name.is_empty() {
        settings.from_address.parse().map_err(|e| format!("发件人地址格式错误: {}", e))?
    } else {
        format!("{} <{}>", settings.from_name, settings.from_address)
            .parse().map_err(|e| format!("发件人地址格式错误: {}", e))?
    };
    let to_mailbox: Mailbox = to.parse().map_err(|e| format!("收件人地址格式错误: {}", e))?;

    let email = lettre::Message::builder()
        .from(from_mailbox)
        .to(to_mailbox)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(body.to_string())
        .map_err(|e| format!("构建邮件失败: {}", e))?;

    let creds = Credentials::new(
        settings.smtp_username.clone(),
        settings.smtp_password.clone(),
    );

    let transport = match settings.smtp_encryption.as_str() {
        "starttls" => {
            SmtpTransport::starttls_relay(&settings.smtp_host)
                .map_err(|e| format!("SMTP STARTTLS 连接失败: {}", e))?
                .port(port)
                .credentials(creds)
                .build()
        }
        _ => {
            // 默认使用 TLS（端口 465）
            SmtpTransport::relay(&settings.smtp_host)
                .map_err(|e| format!("SMTP TLS 连接失败: {}", e))?
                .port(port)
                .credentials(creds)
                .build()
        }
    };

    // lettre 的 sync Transport 需要 spawn_blocking
    let result = tokio::task::spawn_blocking(move || transport.send(&email)).await;
    match result {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(format!("SMTP 发送失败: {}", e)),
        Err(e) => Err(format!("SMTP 任务异常: {}", e)),
    }
}

// 测试邮件发送 — 返回 (成功, 消息)
pub async fn test_email_send(
    pool: &SqlitePool,
    to: &str,
) -> (bool, String) {
    let settings = get_email_settings(pool).await;
    if !settings.enabled {
        return (false, "邮件服务未启用，请先在设置中开启".to_string());
    }
    if settings.from_address.is_empty() {
        return (false, "发件人地址未配置".to_string());
    }
    let subject = format!("[{}] 邮件测试", settings.from_name);
    let body = format!(
        "<h2>邮件发送测试</h2>\
         <p>这是一封测试邮件，用于验证邮件服务配置是否正确。</p>\
         <hr>\
         <p style='color:#999;font-size:12px;'>\
         发送方式: {}<br>\
         发件人: {}<br>\
         收件人: {}<br>\
         时间: {}\
         </p>",
        if settings.provider == "sendflare" { "SendFlare API" } else { "SMTP" },
        settings.from_address,
        to,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
    );
    match send_email(pool, to, &subject, &body).await {
        Ok(()) => (true, format!("测试邮件已成功发送至 {}", to)),
        Err(e) => (false, format!("发送失败: {}", e)),
    }
}
