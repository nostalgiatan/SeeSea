//! SeeSea Stock 命令行工具
//!
//! 用法:
//!   stock-cli list              # 获取 A 股实时行情
//!   stock-cli search <关键词>   # 搜索股票
//!   stock-cli get <代码>        # 获取指定股票信息
//!   stock-cli board zt          # 涨停板
//!   stock-cli index             # 获取指数数据
//!   stock-cli codes             # 获取股票代码名称映射

use clap::{Parser, Subcommand};
use seesea_stock::{StockClient, StockProcessor, get_scheduler, start_scheduler, stop_scheduler};
use std::collections::HashMap;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser)]
#[command(name = "stock-cli")]
#[command(about = "SeeSea 股票数据命令行工具", long_about = None)]
struct Cli {
    /// 显示详细日志
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 获取 A 股实时行情列表
    List {
        /// 显示数量限制
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// 搜索股票
    Search {
        /// 搜索关键词
        keyword: String,
        /// 显示数量限制
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// 获取指定股票详情
    Get {
        /// 股票代码
        code: String,
    },
    /// 获取涨停/跌停池
    Board {
        /// 板块类型: zt(涨停), dt(跌停)
        #[arg(value_parser = ["zt", "dt"])]
        board_type: String,
        /// 日期 YYYYMMDD，默认今天
        #[arg(short, long)]
        date: Option<String>,
        /// 显示数量限制
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// 获取指数数据
    Index {
        /// 显示数量限制
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// 获取股票代码名称映射表
    Codes {
        /// 显示数量限制
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// 调用 akshare 任意函数
    Call {
        /// 函数名
        func: String,
        /// 参数 key=value 格式
        #[arg(short, long)]
        args: Vec<String>,
    },
    /// 启动调度器（测试）
    Scheduler {
        /// 运行时长（秒），默认 30 秒
        #[arg(short, long, default_value = "30")]
        duration: u64,
    },
}

fn main() {
    let cli = Cli::parse();

    // 初始化日志
    let filter = if cli.verbose {
        // 只显示 seesea 相关的 debug，其他模块 warn
        EnvFilter::new("warn,seesea_stock=debug")
    } else {
        EnvFilter::new("warn,seesea_stock=info")
    };
    fmt().with_env_filter(filter).init();

    match cli.command {
        Commands::List { limit } => cmd_list(limit),
        Commands::Search { keyword, limit } => cmd_search(&keyword, limit),
        Commands::Get { code } => cmd_get(&code),
        Commands::Board {
            board_type,
            date,
            limit,
        } => cmd_board(&board_type, date, limit),
        Commands::Index { limit } => cmd_index(limit),
        Commands::Codes { limit } => cmd_codes(limit),
        Commands::Call { func, args } => cmd_call(&func, args),
        Commands::Scheduler { duration } => cmd_scheduler(duration),
    }
}

fn get_client() -> StockClient {
    match StockClient::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ 初始化客户端失败: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_list(limit: usize) {
    println!("📊 获取 A 股实时行情...\n");

    let client = get_client();
    match client.stock_zh_a_spot_em() {
        Ok(data) => {
            if let Some(arr) = data.as_array() {
                let count = arr.len().min(limit);
                println!("共 {} 只股票，显示前 {} 只:\n", arr.len(), count);
                print_table_header();
                for item in arr.iter().take(count) {
                    print_stock_row(item);
                }
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).unwrap_or_default()
                );
            }
        }
        Err(e) => eprintln!("❌ 获取失败: {}", e),
    }
}

fn cmd_search(keyword: &str, limit: usize) {
    println!("🔍 搜索: {}\n", keyword);

    // 先获取数据
    let client = get_client();
    let data = match client.stock_info_a_code_name() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("❌ 获取股票列表失败: {}", e);
            return;
        }
    };

    let processor = StockProcessor::new();
    match processor.search(&data, keyword, Some(limit)) {
        Ok(results) => {
            if results.is_empty() {
                println!("未找到匹配的股票");
                return;
            }
            println!("找到 {} 条结果:\n", results.len());
            for (i, item) in results.iter().enumerate() {
                let code = item.get("code").and_then(|v| v.as_str()).unwrap_or("-");
                let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("-");
                println!("{:>3}. {} {}", i + 1, code, name);
            }
        }
        Err(e) => eprintln!("❌ 搜索失败: {}", e),
    }
}

fn cmd_get(code: &str) {
    println!("📈 获取股票 {} 详情...\n", code);

    // 先获取全量数据
    let client = get_client();
    let data = match client.stock_zh_a_spot_em() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("❌ 获取行情失败: {}", e);
            return;
        }
    };

    let processor = StockProcessor::new();
    match processor.get_by_code(&data, code) {
        Ok(Some(stock)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&stock).unwrap_or_default()
            );
        }
        Ok(None) => println!("❌ 未找到股票: {}", code),
        Err(e) => eprintln!("❌ 获取失败: {}", e),
    }
}

fn cmd_board(board_type: &str, date: Option<String>, limit: usize) {
    let board_name = match board_type {
        "zt" => "涨停板",
        "dt" => "跌停板",
        _ => board_type,
    };

    let date_str = date.unwrap_or_else(|| chrono::Local::now().format("%Y%m%d").to_string());

    println!("📋 获取 {} 数据 ({})...\n", board_name, date_str);

    let client = get_client();
    let result = match board_type {
        "zt" => client.stock_zt_pool_em(&date_str),
        "dt" => client.stock_dt_pool_em(&date_str),
        _ => {
            eprintln!("❌ 未知板块类型: {}", board_type);
            return;
        }
    };

    match result {
        Ok(data) => {
            if let Some(arr) = data.as_array() {
                let count = arr.len().min(limit);
                println!("共 {} 只股票，显示前 {} 只:\n", arr.len(), count);
                for (i, item) in arr.iter().take(count).enumerate() {
                    let code = item.get("代码").and_then(|v| v.as_str()).unwrap_or("-");
                    let name = item.get("名称").and_then(|v| v.as_str()).unwrap_or("-");
                    let price = item.get("最新价").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let pct = item.get("涨跌幅").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    println!(
                        "{:>3}. {} {} {:.2} ({:+.2}%)",
                        i + 1,
                        code,
                        name,
                        price,
                        pct
                    );
                }
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).unwrap_or_default()
                );
            }
        }
        Err(e) => eprintln!("❌ 获取失败: {}", e),
    }
}

fn cmd_index(limit: usize) {
    println!("📊 获取指数数据...\n");

    let client = get_client();
    match client.stock_zh_index_spot_em() {
        Ok(data) => {
            if let Some(arr) = data.as_array() {
                let count = arr.len().min(limit);
                println!("共 {} 个指数，显示前 {} 个:\n", arr.len(), count);
                for (i, item) in arr.iter().take(count).enumerate() {
                    let code = item.get("代码").and_then(|v| v.as_str()).unwrap_or("-");
                    let name = item.get("名称").and_then(|v| v.as_str()).unwrap_or("-");
                    let price = item.get("最新价").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let pct = item.get("涨跌幅").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    println!(
                        "{:>3}. {} {} {:.2} ({:+.2}%)",
                        i + 1,
                        code,
                        name,
                        price,
                        pct
                    );
                }
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).unwrap_or_default()
                );
            }
        }
        Err(e) => eprintln!("❌ 获取失败: {}", e),
    }
}

fn cmd_codes(limit: usize) {
    println!("📑 获取股票代码名称映射...\n");

    let client = get_client();
    match client.stock_info_a_code_name() {
        Ok(data) => {
            if let Some(arr) = data.as_array() {
                let count = arr.len().min(limit);
                println!("共 {} 只股票，显示前 {} 只:\n", arr.len(), count);
                for (i, item) in arr.iter().take(count).enumerate() {
                    let code = item.get("code").and_then(|v| v.as_str()).unwrap_or("-");
                    let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("-");
                    println!("{:>4}. {} {}", i + 1, code, name);
                }
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).unwrap_or_default()
                );
            }
        }
        Err(e) => eprintln!("❌ 获取失败: {}", e),
    }
}

fn cmd_call(func: &str, args: Vec<String>) {
    println!("📞 调用 akshare.{}...\n", func);

    let client = get_client();

    // 解析参数
    let mut kwargs = HashMap::new();
    for arg in args {
        if let Some((key, value)) = arg.split_once('=') {
            kwargs.insert(key.to_string(), value.to_string());
        }
    }

    match client.call(func, kwargs) {
        Ok(data) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
        }
        Err(e) => eprintln!("❌ 调用失败: {}", e),
    }
}

fn cmd_scheduler(duration: u64) {
    println!("🚀 启动调度器测试 ({}秒)...\n", duration);
    println!("按 Ctrl+C 可提前停止\n");

    // 设置 Ctrl+C 处理
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("\n⚠️  收到停止信号...");
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("设置 Ctrl+C 处理器失败");

    // 启动调度器
    start_scheduler();

    println!(
        "✅ 调度器已启动，状态: {}",
        if get_scheduler().is_running() {
            "运行中"
        } else {
            "已停止"
        }
    );
    println!("⏰ 等待 {} 秒...\n", duration);

    // 等待指定时间或 Ctrl+C
    let start = std::time::Instant::now();
    while running.load(std::sync::atomic::Ordering::SeqCst) && start.elapsed().as_secs() < duration
    {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // 停止调度器
    stop_scheduler();
    println!("\n✅ 调度器测试完成");
}

fn print_table_header() {
    println!(
        "{:<8} {:<10} {:>10} {:>8} {:>12} {:>12}",
        "代码", "名称", "最新价", "涨跌幅", "成交量", "成交额"
    );
    println!("{}", "-".repeat(70));
}

fn print_stock_row(item: &serde_json::Value) {
    let code = item.get("代码").and_then(|v| v.as_str()).unwrap_or("-");
    let name = item.get("名称").and_then(|v| v.as_str()).unwrap_or("-");
    let price = item.get("最新价").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let pct = item.get("涨跌幅").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let volume = item.get("成交量").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let amount = item.get("成交额").and_then(|v| v.as_f64()).unwrap_or(0.0);

    println!(
        "{:<8} {:<10} {:>10.2} {:>+7.2}% {:>12.0} {:>12.0}",
        code, name, price, pct, volume, amount
    );
}
