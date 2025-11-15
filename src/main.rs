//! SeeSea 主程序入口

use seesea_core::config::{ConfigManager, ConfigLoader, ConfigValidator, SeeSeaConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🌊 SeeSea - 看海看得远，看得广");
    println!("🦀 隐私保护型元搜索引擎");
    println!();

    // 测试配置加载器
    println!("📁 测试配置加载器...");
    let loader = ConfigLoader::new()
        .add_search_path("./config");

    println!("  🔍 自动发现配置文件...");
    match loader.auto_load().await {
        Ok(load_result) => {
            println!("  ✅ 配置加载成功");
            println!("  📄 文件路径: {:?}", load_result.file_path);
            println!("  ⚠️ 警告数量: {}", load_result.warnings.len());

            if !load_result.warnings.is_empty() {
                println!("  警告列表:");
                for warning in &load_result.warnings {
                    println!("    - {}", warning);
                }
            }
        }
        Err(e) => {
            println!("  ❌ 配置加载失败: {}", e);
            // 使用默认配置继续
            println!("  🔄 使用默认配置继续");
        }
    }

    // 测试配置验证器
    println!("\n🔍 测试配置验证器...");
    let validator = ConfigValidator::new();
    let config = SeeSeaConfig::development();
    let validation_result = validator.validate(&config);

    if validation_result.is_valid {
        println!("  ✅ 配置验证通过");
    } else {
        println!("  ❌ 配置验证失败");
        println!("  错误数量: {}", validation_result.errors.len());
        for error in &validation_result.errors {
            println!("    - {}", error);
        }
    }

    if !validation_result.warnings.is_empty() {
        println!("  ⚠️ 警告数量: {}", validation_result.warnings.len());
        for warning in &validation_result.warnings {
            println!("    - {}", warning);
        }
    }

    // 测试环境特定配置
    println!("\n🏗️ 测试环境特定配置...");

    let environments = vec!["development", "testing", "staging", "production"];
    for env in environments {
        let env_config = match env {
            "development" => SeeSeaConfig::development(),
            "testing" => SeeSeaConfig::testing(),
            "staging" => SeeSeaConfig::default(),
            "production" => SeeSeaConfig::production(),
            _ => SeeSeaConfig::default(),
        };

        let env_validation = validator.validate(&env_config);

        println!("  {} 环境:", env);
        println!("    验证结果: {}", if env_validation.is_valid { "✅ 通过" } else { "❌ 失败" });
        println!("    调试模式: {}", if env_config.general.debug { "启用" } else { "禁用" });
        println!("    HTTPS: {}", if env_config.server.is_https() { "启用" } else { "禁用" });
        println!("    缓存: {}", if env_config.cache.enable_result_cache { "启用" } else { "禁用" });
    }

    // 测试配置管理器
    println!("\n🛠️ 测试配置管理器...");
    let manager = ConfigManager::with_environment(None, "development").await?;

    let current_config = manager.get_config().await;
    println!("  当前配置实例: {}", current_config.general.instance_name);
    println!("  当前环境: {:?}", current_config.general.environment);
    println!("  服务器端口: {}", current_config.server.port);
    println!("  调试模式: {}", current_config.general.debug);
    println!("  生产就绪: {}", manager.is_production_ready().await);

    // 测试配置摘要
    let summary = current_config.get_summary();
    println!("\n📊 配置摘要:");
    println!("  配置文件路径: {}", summary.config_path);
    println!("  环境: {}", summary.environment);
    println!("  启用引擎: {}/{}", summary.enabled_engines, summary.total_engines);
    println!("  启用代理: {}", summary.enabled_proxies);
    println!("  缓存: {}", summary.cache_enabled);

    // 测试配置建议
    println!("\n💡 配置建议:");
    let recommendations = current_config.get_config_recommendations();
    for recommendation in recommendations {
        println!("  • {}", recommendation);
    }

    println!("\n🚀 SeeSea 配置系统测试完成！");
    Ok(())
}