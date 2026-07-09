use crate::config::EMAIL_CONF;
use lettre::message::{Mailbox, Message, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use std::fs;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub enable_notify: bool,
    pub smtp_server: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub retry_interval: u64,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            enable_notify: false,
            smtp_server: String::new(),
            smtp_port: 587,
            username: String::new(),
            password: String::new(),
            from: String::new(),
            to: String::new(),
            subject: "TaskMod 通知".to_string(),
            body: "脚本已执行完成".to_string(),
            timeout_secs: 30,
            max_retries: 3,
            retry_interval: 1,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum EmailError {
    AuthError(String),
    NetworkError(String),
    TimeoutError(String),
    ConnectionRefusedError(String),
    FormatError(String),
    AttachmentError(String, String),
    Other(String),
}

impl std::fmt::Display for EmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailError::AuthError(e) => write!(f, "SMTP认证失败: {}", e),
            EmailError::NetworkError(e) => write!(f, "网络错误: {}", e),
            EmailError::TimeoutError(e) => write!(f, "超时错误: {}", e),
            EmailError::ConnectionRefusedError(e) => write!(f, "连接被拒绝: {}", e),
            EmailError::FormatError(e) => write!(f, "格式错误: {}", e),
            EmailError::AttachmentError(e, filename) => write!(f, "附件错误 {}: {}", filename, e),
            EmailError::Other(e) => write!(f, "其他错误: {}", e),
        }
    }
}

pub fn parse_email_conf() -> EmailConfig {
    let mut config = EmailConfig::default();
    
    if let Ok(content) = fs::read_to_string(EMAIL_CONF) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_lowercase();
                let value = value.trim().to_string();
                match key.as_str() {
                    "enable_notify" => config.enable_notify = value == "true" || value == "1",
                    "smtp_server" => config.smtp_server = value,
                    "smtp_port" => config.smtp_port = value.parse().unwrap_or(587),
                    "username" => config.username = value,
                    "password" => config.password = value,
                    "from" => config.from = value,
                    "to" => config.to = value,
                    "subject" => config.subject = value,
                    "body" => config.body = value,
                    "timeout_secs" => config.timeout_secs = value.parse().unwrap_or(30),
                    "max_retries" => config.max_retries = value.parse().unwrap_or(3),
                    "retry_interval" => config.retry_interval = value.parse().unwrap_or(1),
                    _ => {}
                }
            }
        }
    }
    
    config
}

pub fn save_email_conf(config: &EmailConfig) -> Result<(), std::io::Error> {
    let content = format!(
        "# TaskMod 邮件配置\n# enable_notify=true 启用邮件通知（热加载）\n# 不配置或enabled=false则不加载邮件功能\n\nenable_notify={}\nsmtp_server={}\nsmtp_port={}\nusername={}\npassword={}\nfrom={}\nto={}\nsubject={}\nbody={}\ntimeout_secs={}\nmax_retries={}\nretry_interval={}",
        if config.enable_notify { "true" } else { "false" },
        config.smtp_server,
        config.smtp_port,
        config.username,
        config.password,
        config.from,
        config.to,
        config.subject,
        config.body,
        config.timeout_secs,
        config.max_retries,
        config.retry_interval
    );
    if let Some(parent) = std::path::Path::new(EMAIL_CONF).parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(EMAIL_CONF, content)
}

pub fn get_email_config() -> EmailConfig {
    parse_email_conf()
}

async fn send_email_inner(
    config: &EmailConfig,
    subject: &str,
    body: &str,
    attachments: Option<&Vec<(String, Vec<u8>)>>,
) -> Result<String, EmailError> {
    let from: Mailbox = config.from.parse::<Mailbox>().map_err(|e| {
        log("Email", &format!("发件人地址无效: {}", e));
        EmailError::FormatError(e.to_string())
    })?;

    let to_addrs: Vec<Mailbox> = config.to.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<Mailbox>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            log("Email", &format!("收件人地址无效: {}", e));
            EmailError::FormatError(e.to_string())
        })?;

    let mut builder = Message::builder().from(from);
    for addr in to_addrs {
        builder = builder.to(addr);
    }
    let builder = builder.subject(subject);

    let email = match attachments {
        Some(attachments) if !attachments.is_empty() => {
            log("Email", &format!("创建带附件的邮件，共 {} 个附件", attachments.len()));
            let mut multipart = MultiPart::mixed().singlepart(SinglePart::plain(body.to_string()));
            
            for (filename, content) in attachments {
                log("Email", &format!("添加附件: {} ({} bytes)", filename, content.len()));
                multipart = multipart.singlepart(
                    SinglePart::builder()
                        .header(lettre::message::header::ContentType::parse("application/octet-stream").unwrap())
                        .header(lettre::message::header::ContentDisposition::attachment(filename))
                        .body(content.clone())
                );
            }
            
            builder.multipart(multipart).map_err(|e| {
                log("Email", &format!("创建多部分邮件失败: {}", e));
                EmailError::Other(e.to_string())
            })?
        }
        _ => {
            builder.body(body.to_string()).map_err(|e| {
                log("Email", &format!("邮件构建失败: {}", e));
                EmailError::Other(e.to_string())
            })?
        }
    };

    let creds = Credentials::new(config.username.clone(), config.password.clone());
    
    log("Email", &format!("连接SMTP服务器: {}:{}", config.smtp_server, config.smtp_port));
    
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_server)
        .port(config.smtp_port)
        .credentials(creds)
        .timeout(Some(Duration::from_secs(config.timeout_secs)))
        .build();

    match mailer.send(email).await {
        Ok(_) => {
            log("Email", &format!("邮件发送成功: {}", subject));
            Ok("Sent".to_string())
        }
        Err(e) => {
            let error_str = e.to_string();
            log("Email", &format!("邮件发送失败: {}", error_str));
            
            if error_str.contains("authentication") {
                Err(EmailError::AuthError(error_str))
            } else if error_str.contains("timeout") {
                Err(EmailError::TimeoutError(error_str))
            } else if error_str.contains("connection refused") {
                Err(EmailError::ConnectionRefusedError(error_str))
            } else if error_str.contains("connection") {
                Err(EmailError::NetworkError(error_str))
            } else {
                Err(EmailError::Other(error_str))
            }
        }
    }
}

async fn retry_email_sending<F, Fut>(
    max_retries: u32,
    retry_interval: u64,
    send_fn: F,
) -> Result<String, EmailError>
where
    F: Fn() -> Fut + Send,
    Fut: std::future::Future<Output = Result<String, EmailError>> + Send,
{
    let mut last_error: Option<EmailError> = None;
    let mut backoff = retry_interval.max(1);
    
    for attempt in 0..=max_retries {
        log("Email", &format!("第 {} 次尝试发送邮件", attempt + 1));
        
        match send_fn().await {
            Ok(result) => {
                log("Email", "邮件发送成功");
                return Ok(result);
            }
            Err(e) => {
                log("Email", &format!("第 {} 次尝试失败: {}", attempt + 1, e));
                
                let is_retryable = matches!(
                    &e,
                    EmailError::NetworkError(_) | EmailError::TimeoutError(_) | EmailError::ConnectionRefusedError(_)
                );
                
                if is_retryable {
                    last_error = Some(e);
                    if attempt < max_retries {
                        log("Email", &format!("{} 秒后重试（指数退避）...", backoff));
                        tokio::time::sleep(Duration::from_secs(backoff)).await;
                        backoff = (backoff * 2).min(60); // 最大60秒
                    }
                } else {
                    log("Email", &format!("非重试错误: {}", e));
                    return Err(e);
                }
            }
        }
    }
    
    log("Email", &format!("所有 {} 次尝试都失败", max_retries + 1));
    Err(last_error.unwrap_or_else(|| EmailError::Other("邮件发送失败".to_string())))
}

pub async fn send_email(
    config: &EmailConfig,
    subject: Option<&str>,
    body: Option<&str>,
    attachments: Option<Vec<(String, Vec<u8>)>>,
) -> Result<String, EmailError> {
    if !config.enable_notify && subject.is_none() {
        log("Email", "邮件通知未启用，跳过发送");
        return Ok("邮件通知未启用".to_string());
    }

    if config.smtp_server.is_empty() {
        return Err(EmailError::Other("SMTP服务器未配置".to_string()));
    }

    let subject = subject.unwrap_or(&config.subject);
    let body = body.unwrap_or(&config.body);
    
    retry_email_sending(
        config.max_retries,
        config.retry_interval,
        || async {
            send_email_inner(config, subject, body, attachments.as_ref()).await
        },
    ).await
}

fn log(module: &str, msg: &str) {
    use crate::config::LOG_FILE;
    let now = chrono::Local::now();
    let log_msg = format!("[{}] [{}] {}", now.format("%Y-%m-%d %H:%M:%S"), module, msg);
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)
        .and_then(|mut f| {
            use std::io::Write;
            writeln!(f, "{}", log_msg)
        });
}