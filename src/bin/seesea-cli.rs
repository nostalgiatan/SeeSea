// Copyright 2025 nostalgiatan
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! SeeSea 命令行界面
//!
//! 提供命令行交互式搜索功能

use clap::{Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};
use std::time::Duration;

use seesea_core::derive::{SearchQuery, SearchResultItem};
use seesea_core::search::{SearchInterface, SearchConfig, SearchRequest};
use seesea_core::search::engine_config::EngineMode;

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
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    match cli.command {
        Some(Commands::Search { query, global, engines, verbose, debug }) => {
            execute_search(query, global, engines, verbose, debug).await?;
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
    debug: bool,
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
        (EngineMode::Custom(engine_list.clone()), engine_list)
    } else {
        // 默认使用全局模式
        (EngineMode::Global, vec![])
    };

    println!("📌 查询: {}", query_str.bright_white().bold());
    println!("⚙️  模式: {}",
        match mode {
            EngineMode::Global => "全局模式（所有引擎）".bright_green(),
            EngineMode::Custom(_) => "配置模式".bright_yellow(),
        }
    );

    // 创建搜索接口
    let search_config = SearchConfig::default();
    let search_interface = std::sync::Arc::new(
        SearchInterface::new(search_config)
            .map_err(|e| format!("Failed to create search interface: {}", e))?
    );

    // 显示要使用的引擎
    println!("🔍 使用引擎: {}",
        if configured_engines.is_empty() {
            match mode {
                EngineMode::Global => search_interface.list_global_engines().join(", "),
                _ => "默认引擎".to_string(),
            }
        } else {
            configured_engines.join(", ")
        }.bright_blue()
    );

    // 检查是否使用了缓存
    println!("🗄️  缓存: {}", "已启用".bright_green());
    println!();

    // 创建搜索查询
    let mut query = SearchQuery::default();
    query.query = query_str;

    // 创建搜索请求
    let search_request = SearchRequest {
        query: query.clone(),
        engines: configured_engines.clone(),
        timeout: Some(std::time::Duration::from_secs(30)),
        max_results: Some(100),
        force: false,
        cache_timeline: Some(3600),
    };

    // 执行搜索
    println!("{}", "正在搜索...".bright_yellow());

    // 创建进度条
    let progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap()
            .progress_chars("=>-")
    );
    progress_bar.set_message("正在搜索...");
    progress_bar.enable_steady_tick(Duration::from_millis(120));

    let search_result = if let EngineMode::Custom(_) = mode {
        // 配置模式，使用指定引擎
        search_interface.search(&search_request).await
    } else {
        // 全局或中国模式，使用模式搜索
        search_interface.search_with_mode(&search_request, mode).await
    };

    // 完成进度条
    progress_bar.finish_with_message("搜索完成！");

    println!();

    // 处理搜索结果
    match search_result {
        Ok(response) => {
            println!("{}", "🔍 搜索结果".bright_cyan().bold());
            println!("{}", "━".repeat(60).bright_black());

            if response.cached {
                println!("🗄️  {} 从缓存获取", "结果".bright_green());
                println!();
            }

            // 显示使用的引擎
            println!("🔧 实际使用的引擎: {}", response.engines_used.join(", ").bright_blue());
            println!("📊 总结果数: {}", response.total_count.to_string().bright_white().bold());
            println!("⏱️  查询时间: {} ms", response.query_time_ms.to_string().bright_yellow());
            println!();

            // 收集所有结果
            let mut all_results: Vec<(String, SearchResultItem)> = Vec::new();

            for search_result in &response.results {
                for item in &search_result.items {
                    all_results.push((search_result.engine_name.clone(), item.clone()));
                }
            }

            // 注意：结果已经在SearchInterface中通过BM25评分排序，这里不需要重复排序
            // SearchInterface的aggregate_with_scoring已经处理了评分和排序

            // 显示前20个结果
            let results_to_show = if verbose {
                all_results.len().min(50)
            } else {
                all_results.len().min(20)
            };

            if all_results.is_empty() {
                println!("❌ {}", "没有找到结果".bright_red());
            } else {
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
            println!("📊 搜索完成: {} 个引擎, {} 个结果",
                response.engines_used.len().to_string().bright_green(),
                response.total_count.to_string().bright_white().bold()
            );
        }
        Err(e) => {
            println!("❌ 搜索失败: {}", format!("{}", e).bright_red());
            if debug {
                println!("🔍 详细错误: {:?}", e);
            }
        }
    }

    // 显示统计信息
    if verbose {
        println!();
        print_search_stats(&search_interface).await;
    }

    Ok(())
}

/// 打印搜索统计信息
async fn print_search_stats(search_interface: &SearchInterface) {
    println!("{}", "📊 搜索统计信息".bright_cyan().bold());
    println!("{}", "━".repeat(60).bright_black());

    let stats = search_interface.get_stats().await;

    println!("  {} {}",
        format!("{:20}", "总搜索次数").bright_white().bold(),
        stats.total_searches.to_string().bright_white()
    );
    println!("  {} {}",
        format!("{:20}", "缓存命中").bright_white().bold(),
        stats.cache_hits.to_string().bright_green()
    );
    println!("  {} {}",
        format!("{:20}", "缓存未命中").bright_white().bold(),
        stats.cache_misses.to_string().bright_yellow()
    );
    println!("  {} {}",
        format!("{:20}", "引擎失败").bright_white().bold(),
        stats.engine_failures.to_string().bright_red()
    );
    println!("  {} {}",
        format!("{:20}", "超时次数").bright_white().bold(),
        stats.timeouts.to_string().bright_red()
    );

    let total_requests = stats.cache_hits + stats.cache_misses;
    if total_requests > 0 {
        let cache_hit_rate = (stats.cache_hits as f64 / total_requests as f64 * 100.0) as u32;
        println!("  {} {}",
            format!("{:20}", "缓存命中率").bright_white().bold(),
            format!("{}%", cache_hit_rate).bright_green()
        );
    }
}

/// 列出所有引擎
async fn list_engines(show_stats: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🔍 可用的搜索引擎".bright_cyan().bold());
    println!("{}", "━".repeat(60).bright_black());

    // 创建搜索接口
    let search_config = SearchConfig::default();
    let search_interface = SearchInterface::new(search_config)
        .map_err(|e| format!("Failed to create search interface: {}", e))?;

    // 列出所有可用引擎
    println!("\n🌍 {} 可用引擎", "━━━━━━━━━━━━━━━━━━".bright_green());
    let global_engines = search_interface.list_global_engines();
    for (i, engine) in global_engines.iter().enumerate() {
        println!("  {}. {}", (i + 1).to_string().bright_white().bold(), engine.bright_blue());
    }

    // 显示统计信息
    if show_stats {
        println!();
        print_search_stats(&search_interface).await;
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

    // 创建搜索接口（用于交互模式）
    let search_config = SearchConfig::default();
    let search_interface = std::sync::Arc::new(
        SearchInterface::new(search_config)
            .map_err(|e| format!("Failed to create search interface: {}", e))?
    );

    let mut mode = if use_global {
        EngineMode::Global
    } else {
        EngineMode::Global
    };

    let mut configured_engines = if use_global {
        vec![]
    } else {
        vec![]
    };

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
                print_search_stats(&search_interface).await;
            }
            "mode" => {
                println!("{}", "\n🔄 选择运行模式:".bright_cyan().bold());
                println!("1. 全局模式 (所有引擎)");
                println!("2. 配置模式 (自定义引擎)");
                print!("请选择 (1-2): ");
                io::stdout().flush()?;

                let mut choice = String::new();
                io::stdin().read_line(&mut choice)?;

                match choice.trim() {
                    "1" => {
                        println!("{}", "✅ 切换到全局模式".bright_green());
                        mode = EngineMode::Global;
                        configured_engines = vec![];
                    }
                    "2" => {
                        println!("{}", "✅ 切换到配置模式".bright_yellow());
                        mode = EngineMode::Custom(configured_engines.clone());
                    }
                    _ => {
                        println!("{}", "❌ 无效选择，保持当前模式".bright_red());
                    }
                };
            }
            _ => {
                // 根据当前模式执行搜索
                match mode {
                    EngineMode::Global => {
                        execute_search(input.to_string(), true, None, false, false).await?;
                    }
                    EngineMode::Custom(ref engines) => {
                        execute_search(input.to_string(), false, Some(engines.join(",")), false, false).await?;
                    }
                }
            }
        }

        println!();
    }

    Ok(())
}

