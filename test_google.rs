use reqwest;
use std::time::SystemTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    // 生成 async 参数
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_millis();
    let async_id = format!("{}:{}", timestamp, timestamp % 1000000 + 1000000);

    // 构建 URL
    let url = format!(
        "https://www.google.com/search?q=Rust&start=0&ie=utf8&oe=utf8&filter=0&asearch=arc&async={}",
        async_id
    );

    println!("Testing URL: {}", url);

    let response = client.get(&url).send().await?;
    let status = response.status();
    println!("Status: {}", status);

    let headers = response.headers();
    println!("Response headers:");
    for (name, value) in headers.iter() {
        println!("  {}: {:?}", name, value);
    }

    let content = response.text().await?;
    println!("Response length: {} bytes", content.len());

    // 打印前500个字符
    println!("First 500 chars:");
    println!("{}", &content[..content.len().min(500)]);

    // 检查是否包含特定的Google错误页面内容
    if content.contains("captcha") || content.contains("CAPTCHA") {
        println!("🚨 CAPTCHA detected!");
    }
    if content.contains("unusual traffic") {
        println!("🚨 Unusual traffic page detected!");
    }
    if content.contains("robot") {
        println!("🚨 Robot detection page detected!");
    }
    if content.contains("sorry.google.com") {
        println!("🚨 Google sorry page detected!");
    }

    Ok(())
}