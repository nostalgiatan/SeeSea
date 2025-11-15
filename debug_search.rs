//! Debug tool for search engines
use SeeSea::search::{EngineManager, EngineMode};
use SeeSea::derive::SearchQuery;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run --bin debug_search <query> [engine]");
        return Ok(());
    }

    let query_str = args[1].clone();
    let engine_name = if args.len() > 2 {
        Some(args[2].clone())
    } else {
        None
    };

    println!("🔍 Debug search for: {}", query_str);

    let manager = EngineManager::new(EngineMode::Global, vec![]);
    let engines = if let Some(engine) = engine_name {
        vec![engine]
    } else {
        manager.get_active_engines().await
    };

    let mut query = SearchQuery::default();
    query.query = query_str.clone();

    for engine_name in engines {
        println!("\n🚀 Testing engine: {}", engine_name);
        println!("{}", "=".repeat(50));

        let results = manager.search_single_engine(&engine_name, &query).await;

        match results {
            Ok(search_result) => {
                println!("✅ Success - {} results", search_result.items.len());
                println!("⏱️  Time: {} ms", search_result.elapsed_ms);
                println!("📊 Total results: {:?}", search_result.total_results);

                if !search_result.items.is_empty() {
                    println!("\n📋 Results:");
                    for (i, item) in search_result.items.iter().take(3).enumerate() {
                        println!("  {}. {}", i + 1, item.title);
                        println!("     URL: {}", item.url);
                        println!("     Content: {}...",
                            if item.content.len() > 100 {
                                &item.content[..100]
                            } else {
                                &item.content
                            });
                        println!("     Score: {:.2}", item.score);
                        println!();
                    }
                }
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }

    Ok(())
}