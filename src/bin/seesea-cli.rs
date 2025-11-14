//! SeeSea 命令行界面
//!
//! 提供命令行交互式搜索功能

use clap::{Parser, Subcommand};
use colored::*;
use std::io::{self, Write};

use SeeSea::derive::SearchQuery;
use SeeSea::search::{EngineManager, EngineMode};

/// SeeSea 命令行应用
#[derive(Parser)]
#[command(name = "seesea")]
#[command(about = "🌊 SeeSea - 隐私保护型元搜索引擎", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 执行搜索
    Search {
        /// 搜索查询
        query: String,
        
        /// 使用全局模式（所有引擎）
        #[arg(short, long)]
        global: bool,
        
        /// 指定使用的引擎（逗号分隔）
        #[arg(short, long)]
        engines: Option<String>,
        
        /// 显示详细输出
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// 列出所有可用的搜索引擎
    ListEngines {
        /// 显示引擎统计信息
        #[arg(short, long)]
        stats: bool,
    },
    
    /// 交互式搜索模式
    Interactive {
        /// 使用全局模式
        #[arg(short, long)]
        global: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    match cli.command {
        Some(Commands::Search { query, global, engines, verbose }) => {
            execute_search(query, global, engines, verbose).await?;
        }
        Some(Commands::ListEngines { stats }) => {
            list_engines(stats).await?;
        }
        Some(Commands::Interactive { global }) => {
            interactive_mode(global).await?;
        }
        None => {
            // 默认进入交互模式
            interactive_mode(false).await?;
        }
    }
    
    Ok(())
}

/// 执行搜索
async fn execute_search(
    query_str: String,
    use_global: bool,
    engines_str: Option<String>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🌊 SeeSea 搜索".bright_cyan().bold());
    println!("{}", "━".repeat(60).bright_black());
    
    // 确定运行模式和引擎列表
    let (mode, configured_engines) = if use_global {
        (EngineMode::Global, vec![])
    } else if let Some(engines) = engines_str {
        let engine_list: Vec<String> = engines
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        (EngineMode::Configured, engine_list)
    } else {
        // 默认使用一些常用引擎
        (
            EngineMode::Configured,
            vec!["google".to_string(), "bing".to_string(), "duckduckgo".to_string()],
        )
    };
    
    println!("📌 查询: {}", query_str.bright_white().bold());
    println!("⚙️  模式: {}", 
        match mode {
            EngineMode::Global => "全局模式（所有引擎）".bright_green(),
            EngineMode::Configured => "配置模式".bright_yellow(),
        }
    );
    
    // 创建引擎管理器
    let manager = EngineManager::new(mode, configured_engines);
    
    // 显示活跃的引擎
    let active_engines = manager.get_active_engines().await;
    println!("🔍 使用引擎: {}", active_engines.join(", ").bright_blue());
    println!();
    
    // 创建搜索查询
    let mut query = SearchQuery::default();
    query.query = query_str;
    
    // 执行并发搜索
    println!("{}", "正在搜索...".bright_yellow());
    let results = manager.search_concurrent(&query).await;
    
    println!();
    println!("{}", "搜索结果".bright_cyan().bold());
    println!("{}", "━".repeat(60).bright_black());
    
    // 显示结果
    let mut success_count = 0;
    let mut failure_count = 0;
    
    for (engine_name, result) in &results {
        match result {
            Ok(search_result) => {
                success_count += 1;
                println!("✅ {} - {} 个结果", 
                    engine_name.bright_green(),
                    search_result.items.len().to_string().bright_white().bold()
                );
                
                if verbose && !search_result.items.is_empty() {
                    for (i, item) in search_result.items.iter().take(3).enumerate() {
                        println!("   {}. {}", i + 1, item.title.bright_white());
                        println!("      {}", item.url.bright_black());
                    }
                }
            }
            Err(e) => {
                failure_count += 1;
                println!("❌ {} - {}", 
                    engine_name.bright_red(),
                    e.bright_red()
                );
            }
        }
    }
    
    println!();
    println!("{}", "━".repeat(60).bright_black());
    println!("📊 统计: {} 成功, {} 失败", 
        success_count.to_string().bright_green(),
        failure_count.to_string().bright_red()
    );
    
    // 显示引擎统计信息
    if verbose {
        println!();
        print_engine_stats(&manager).await;
    }
    
    Ok(())
}

/// 列出所有引擎
async fn list_engines(show_stats: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🔍 可用的搜索引擎".bright_cyan().bold());
    println!("{}", "━".repeat(60).bright_black());
    
    let engines = vec![
        ("duckduckgo", "DuckDuckGo", "隐私保护搜索"),
        ("google", "Google", "全球最流行"),
        ("bing", "Bing", "微软搜索"),
        ("yahoo", "Yahoo", "多地区支持"),
        ("baidu", "Baidu", "中文搜索"),
        ("yandex", "Yandex", "俄罗斯搜索"),
        ("brave", "Brave", "隐私优先"),
        ("qwant", "Qwant", "欧洲隐私"),
        ("startpage", "Startpage", "Google代理"),
        ("mojeek", "Mojeek", "独立索引"),
    ];
    
    for (id, name, desc) in engines {
        println!("  {} {}", 
            format!("{:15}", name).bright_white().bold(),
            format!("({}) - {}", id, desc).bright_black()
        );
    }
    
    if show_stats {
        println!();
        let manager = EngineManager::new(EngineMode::Global, vec![]);
        print_engine_stats(&manager).await;
    }
    
    Ok(())
}

/// 交互式搜索模式
async fn interactive_mode(use_global: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🌊 SeeSea 交互式搜索".bright_cyan().bold());
    println!("{}", "━".repeat(60).bright_black());
    println!("输入查询来搜索，输入 'quit' 或 'exit' 退出");
    println!("输入 'engines' 列出所有引擎");
    println!("输入 'stats' 查看引擎统计信息");
    println!("输入 'mode' 切换运行模式");
    println!("{}", "━".repeat(60).bright_black());
    println!();
    
    let mut mode = if use_global {
        EngineMode::Global
    } else {
        EngineMode::Configured
    };
    
    let configured_engines = vec!["google".to_string(), "bing".to_string(), "duckduckgo".to_string()];
    let mut manager = EngineManager::new(mode, configured_engines.clone());
    
    loop {
        print!("🔍 > ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        match input.to_lowercase().as_str() {
            "quit" | "exit" => {
                println!("{}", "👋 再见！".bright_cyan());
                break;
            }
            "engines" => {
                list_engines(false).await?;
            }
            "stats" => {
                print_engine_stats(&manager).await;
            }
            "mode" => {
                mode = match mode {
                    EngineMode::Global => {
                        println!("{}", "切换到配置模式".bright_yellow());
                        EngineMode::Configured
                    }
                    EngineMode::Configured => {
                        println!("{}", "切换到全局模式".bright_green());
                        EngineMode::Global
                    }
                };
                manager = EngineManager::new(mode, configured_engines.clone());
            }
            _ => {
                execute_search(input.to_string(), mode == EngineMode::Global, None, false).await?;
            }
        }
        
        println!();
    }
    
    Ok(())
}

/// 打印引擎统计信息
async fn print_engine_stats(manager: &EngineManager) {
    println!("{}", "📊 引擎统计信息".bright_cyan().bold());
    println!("{}", "━".repeat(60).bright_black());
    
    let stats = manager.get_engine_stats().await;
    
    if stats.is_empty() {
        println!("  暂无统计数据");
        return;
    }
    
    for (name, state) in stats {
        let status = if !state.enabled {
            "❌ 禁用".bright_red()
        } else if state.temporarily_disabled {
            "⏸️  临时禁用".bright_yellow()
        } else {
            "✅ 启用".bright_green()
        };
        
        println!("  {} {}", 
            format!("{:15}", name).bright_white().bold(),
            status
        );
        
        if state.total_requests > 0 {
            let success_rate = (state.successful_requests as f64 / state.total_requests as f64 * 100.0) as u32;
            println!("      请求: {} | 成功: {} | 失败: {} | 成功率: {}%",
                state.total_requests,
                state.successful_requests,
                state.failed_requests,
                success_rate
            );
            println!("      平均响应时间: {} ms", state.avg_response_time_ms);
        }
    }
}
