//! SeeSea 命令行界面
//!
//! 提供命令行交互式搜索功能

use clap::{Parser, Subcommand};
use colored::*;
use std::io::{self, Write};

use seesea_core::derive::{SearchQuery, SearchResultItem};
use seesea_core::search::{EngineManager, EngineMode};

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

        /// 使用中国模式（优化的中国友好引擎）
        #[arg(short = 'c', long)]
        china: bool,

        /// 指定使用的引擎（逗号分隔）
        #[arg(short, long)]
        engines: Option<String>,

        /// 显示详细输出
        #[arg(short, long)]
        verbose: bool,

        /// 调试模式 - 显示详细的引擎响应信息
        #[arg(long)]
        debug: bool,
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

        /// 使用中国模式
        #[arg(short = 'c', long)]
        china: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    match cli.command {
        Some(Commands::Search { query, global, china, engines, verbose, debug }) => {
            execute_search(query, global, china, engines, verbose, debug).await?;
        }
        Some(Commands::ListEngines { stats }) => {
            list_engines(stats).await?;
        }
        Some(Commands::Interactive { global, china }) => {
            interactive_mode(global, china).await?;
        }
        None => {
            // 默认进入交互模式
            interactive_mode(false, false).await?;
        }
    }
    
    Ok(())
}

/// 执行搜索
async fn execute_search(
    query_str: String,
    use_global: bool,
    use_china: bool,
    engines_str: Option<String>,
    verbose: bool,
    debug: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🌊 SeeSea 搜索".bright_cyan().bold());
    println!("{}", "━".repeat(60).bright_black());
    
    // 确定运行模式和引擎列表
    let (mode, configured_engines) = if use_china {
        // 中国模式：使用在国内能快速访问的引擎，基于全局测试成功的8个引擎
        let china_engines = vec![
            "yandex".to_string(),       // ✅ 在全局模式成功 - 1527ms响应
            "wikidata".to_string(),     // ✅ 在全局模式成功 - 1067ms响应
            "search360".to_string(),    // ✅ 在全局模式成功 - 1551ms响应
            "bing".to_string(),         // ✅ 在全局模式成功 - 946ms响应
            "baidu".to_string(),        // ✅ 在全局模式成功 - 1108ms响应
            "github".to_string(),       // ✅ 在全局模式成功 - 2040ms响应
            "stackoverflow".to_string(), // ✅ 在全局模式成功 - 1496ms响应
            "unsplash".to_string(),     // ✅ 在全局模式成功 - 1497ms响应
        ];
        (EngineMode::Configured, china_engines)
    } else if use_global {
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
        if use_china {
            "中国模式（优化的中国友好引擎）".bright_red()
        } else {
            match mode {
                EngineMode::Global => "全局模式（所有引擎）".bright_green(),
                EngineMode::Configured => "配置模式".bright_yellow(),
            }
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
    let mut all_results: Vec<(String, SearchResultItem)> = Vec::new();

    // 收集所有成功的结果
    for (engine_name, result) in &results {
        match result {
            Ok(search_result) => {
                success_count += 1;
                println!("✅ {} - {} 个结果",
                    engine_name.bright_green(),
                    search_result.items.len().to_string().bright_white().bold()
                );

                if debug {
                    println!("   ⏱️  响应时间: {} ms", search_result.elapsed_ms);
                    println!("   📊 总结果数: {:?}", search_result.total_results);
                    if !search_result.items.is_empty() {
                        println!("   📋 前3个结果:");
                        for (i, item) in search_result.items.iter().take(3).enumerate() {
                            println!("     {}. {}", i + 1, item.title);
                            println!("        URL: {}", item.url);
                        }
                    }
                }

                // 收集结果用于统一显示
                for item in &search_result.items {
                    all_results.push((engine_name.clone(), item.clone()));
                }
            }
            Err(e) => {
                failure_count += 1;
                println!("❌ {} - {}",
                    engine_name.bright_red(),
                    e.bright_red()
                );

                if debug {
                    println!("   🔍 详细错误: {:?}", e);
                }
            }
        }
    }

    // 如果有结果，显示统一的搜索结果列表
    if !all_results.is_empty() {
        println!();
        println!("{}", "🔍 搜索结果".bright_cyan().bold());
        println!("{}", "━".repeat(60).bright_black());

        // 按评分排序结果
        all_results.sort_by(|a, b| b.1.score.partial_cmp(&a.1.score).unwrap_or_else(|| std::cmp::Ordering::Equal));

        // 显示前20个结果
        let results_to_show = if verbose {
            all_results.len().min(50)
        } else {
            all_results.len().min(20)
        };

        for (i, (engine_name, item)) in all_results.iter().take(results_to_show).enumerate() {
            println!("{}", format!("{}. {}", i + 1, item.title.bright_white().bold()));
            println!("   {}", item.url.bright_blue());

            // 显示内容摘要
            if !item.content.is_empty() {
                let content = if item.content.len() > 200 {
                    // 安全地截断文本，避免在UTF-8字符中间截断
                    let mut end = 200;
                    while !item.content.is_char_boundary(end) {
                        end -= 1;
                    }
                    format!("{}...", &item.content[..end])
                } else {
                    item.content.clone()
                };
                println!("   {}", content.bright_black());
            }

            // 显示显示URL（如果与URL不同）
            if let Some(display_url) = &item.display_url {
                if display_url != &item.url {
                    println!("   {}", format!("🔗 {}", display_url).bright_black());
                }
            }

            // 显示来源引擎
            println!("   {}", format!("📌 来源: {}", engine_name.bright_green()));

            // 显示发布时间（如果有）
            if let Some(published_date) = &item.published_date {
                println!("   {}", format!("📅 {}", published_date.format("%Y-%m-%d")).bright_black());
            }

            // 显示评分（如果大于0）
            if item.score > 0.0 {
                println!("   {}", format!("⭐ 评分: {:.2}", item.score).bright_black());
            }

            println!();
        }

        if all_results.len() > results_to_show {
            println!("{}", format!("... 还有 {} 个结果（使用 --verbose 查看更多）",
                all_results.len() - results_to_show).bright_yellow());
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
async fn interactive_mode(use_global: bool, use_china: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🌊 SeeSea 交互式搜索".bright_cyan().bold());
    println!("{}", "━".repeat(60).bright_black());
    println!("输入查询来搜索，输入 'quit' 或 'exit' 退出");
    println!("输入 'engines' 列出所有引擎");
    println!("输入 'stats' 查看引擎统计信息");
    println!("输入 'mode' 切换运行模式");
    println!("{}", "━".repeat(60).bright_black());
    println!();
    
    let (mode, configured_engines) = if use_china {
        // 中国模式：使用在国内能快速访问的引擎，基于全局测试成功的8个引擎
        let china_engines = vec![
            "yandex".to_string(),       // ✅ 在全局模式成功 - 1527ms响应
            "wikidata".to_string(),     // ✅ 在全局模式成功 - 1067ms响应
            "search360".to_string(),    // ✅ 在全局模式成功 - 1551ms响应
            "bing".to_string(),         // ✅ 在全局模式成功 - 946ms响应
            "baidu".to_string(),        // ✅ 在全局模式成功 - 1108ms响应
            "github".to_string(),       // ✅ 在全局模式成功 - 2040ms响应
            "stackoverflow".to_string(), // ✅ 在全局模式成功 - 1496ms响应
            "unsplash".to_string(),     // ✅ 在全局模式成功 - 1497ms响应
        ];
        (EngineMode::Configured, china_engines)
    } else if use_global {
        (EngineMode::Global, vec![])
    } else {
        // 默认使用一些常用引擎
        (
            EngineMode::Configured,
            vec!["google".to_string(), "bing".to_string(), "duckduckgo".to_string()],
        )
    };

    let mut mode = mode;
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
                execute_search(input.to_string(), mode == EngineMode::Global, false, None, false, false).await?;
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
