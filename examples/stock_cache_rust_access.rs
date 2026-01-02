/// 股票缓存 Rust 端访问示例
///
/// 演示如何从 Rust 端访问 Python 端写入的股票数据
///
/// 使用方法:
/// ```bash
/// cargo run --example stock_cache_rust_access
/// ```

use seesea_core::{CacheInterface, cache::scope::ScopeCache};
use seesea_cache::cache::types::CacheImplConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 股票行情数据结构
#[derive(Debug, Serialize, Deserialize)]
struct StockQuote {
    code: String,
    name: String,
    price: f64,
    change_pct: f64,
    timestamp: String,
}

/// 股票信息数据结构
#[derive(Debug, Serialize, Deserialize)]
struct StockInfo {
    code: String,
    name: String,
    #[serde(rename = "type")]
    info_type: String,
}

/// 从缓存获取股票行情
async fn get_stock_quote(cache: &CacheInterface, code: &str) -> anyhow::Result<Option<StockQuote>> {
    // 获取股票行情作用域
    let quote_cache = cache.scope("stock.quote");

    // 读取数据
    match quote_cache.get(code)? {
        Some(data) => {
            // 反序列化 JSON 数据
            let quote: StockQuote = serde_json::from_slice(&data)?;
            println!("✅ 从作用域 'stock.quote' 读取到股票 {}: {:?}", code, quote);
            Ok(Some(quote))
        }
        None => {
            println!("⚠️  股票 {} 在作用域 'stock.quote' 中不存在", code);
            Ok(None)
        }
    }
}

/// 从缓存获取股票信息
async fn get_stock_info(cache: &CacheInterface, code: &str) -> anyhow::Result<Option<StockInfo>> {
    let info_cache = cache.scope("stock.info");

    match info_cache.get(code)? {
        Some(data) => {
            let info: StockInfo = serde_json::from_slice(&data)?;
            println!("✅ 从作用域 'stock.info' 读取到股票 {}: {:?}", code, info);
            Ok(Some(info))
        }
        None => {
            println!("⚠️  股票 {} 在作用域 'stock.info' 中不存在", code);
            Ok(None)
        }
    }
}

/// 列出指定作用域的所有键
async fn list_scope_keys(cache: &CacheInterface, scope: &str) -> anyhow::Result<()> {
    let scope_cache = cache.scope(scope);
    let keys = scope_cache.keys()?;

    println!("\n📋 作用域 '{}' 中的所有键:", scope);
    for key in &keys {
        println!("   - {}", key);
    }
    println!("   总计: {} 个键", keys.len());

    Ok(())
}

/// 获取缓存统计信息
async fn show_cache_stats(cache: &CacheInterface) -> anyhow::Result<()> {
    let manager = cache.manager()?;
    let stats = manager.stats();

    println!("\n📊 缓存统计信息:");
    println!("   总命中数: {}", stats.total_hits);
    println!("   总未命中数: {}", stats.total_misses);
    println!("   总写入数: {}", stats.total_inserts);
    println!("   总删除数: {}", stats.total_deletes);
    println!("   当前条目数: {}", stats.current_size);

    let total = stats.total_hits + stats.total_misses;
    if total > 0 {
        let hit_rate = stats.total_hits as f64 / total as f64;
        println!("   命中率: {:.2}%", hit_rate * 100.0);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=" .repeat(60));
    println!("🦀 股票缓存 Rust 端访问示例");
    println!("=" .repeat(60));

    // 1. 创建缓存接口（自动使用全局缓存实例）
    println!("\n📦 步骤1: 创建缓存接口...");

    // 获取系统默认缓存路径
    let cache_dir = if cfg!(target_os = "windows") {
        std::path::PathBuf::from("D:\\seesea\\cache")
    } else if cfg!(target_os = "linux") {
        std::path::PathBuf::from("/etc/seesea/cache")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home).join("Library/Caches/seesea")
    };

    let config = CacheImplConfig {
        db_path: cache_dir.to_string_lossy().to_string(),
        ..Default::default()
    };

    let cache = CacheInterface::new(config)?;
    println!("✅ 缓存接口创建成功");

    // 2. 读取 Python 端写入的股票数据
    println!("\n🔍 步骤2: 读取 Python 端写入的股票数据...");

    // 读取股票行情
    if let Some(_quote) = get_stock_quote(&cache, "000001").await? {
        println!("✅ 成功读取股票行情数据");
    }

    // 读取股票信息
    if let Some(_info) = get_stock_info(&cache, "000001").await? {
        println!("✅ 成功读取股票信息数据");
    }

    // 3. 列出各个作用域的键
    println!("\n📋 步骤3: 列出各个作用域的键...");
    list_scope_keys(&cache, "stock.quote").await?;
    list_scope_keys(&cache, "stock.info").await?;
    list_scope_keys(&cache, "stock.financial").await?;

    // 4. 显示缓存统计
    println!("\n📊 步骤4: 显示缓存统计...");
    show_cache_stats(&cache).await?;

    // 5. 总结
    println!("\n" + &"=".repeat(60));
    println!("✅ 演示完成！");
    println!("\n💡 要点总结:");
    println!("   1. Rust 端可以直接读取 Python 端写入的数据");
    println!("   2. 通过作用域 (scope) 隔离不同类型的数据");
    println!("   3. 使用相同的缓存数据库，无需额外配置");
    println!("   4. 支持序列化/反序列化 JSON 数据");
    println!("=" .repeat(60));

    Ok(())
}
