//! `seesea`: search, extract, serve and MCP from one binary.

use clap::{Args, Parser, Subcommand};
use seesea_cli::config::{Settings, build_pipeline};
use seesea_cli::server::{self, AppState, parse_scope};
use seesea_cli::sources::SourceSettings;
use seesea_mcp::McpServer;
use seesea_pipeline::eval;
use seesea_pipeline::render::{extract_markdown, search_markdown};
use seesea_pipeline::{Error, Mode, Pipeline, SearchRequest, SearchResponse, TimeRange};
use serde::Serialize;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncWriteExt, BufReader};

#[derive(Parser)]
#[command(name = "seesea", version, about = "Search the web, read the results: one pipeline for the command line, an HTTP server and MCP")]
struct Cli {
    #[command(flatten)]
    net: NetArgs,
    #[command(subcommand)]
    command: Command,
}

#[derive(Args)]
struct NetArgs {
    /// Address of a SearXNG instance with the json format enabled.
    #[arg(long, env = "SEESEA_SEARXNG_URL", global = true)]
    searxng: Option<String>,
    /// Brave Search API key.
    #[arg(long, env = "SEESEA_BRAVE_API_KEY", global = true, hide_env_values = true)]
    brave_key: Option<String>,
    /// Do not use the built-in scraping engines (Bing, Baidu, Sogou).
    #[arg(long, global = true)]
    no_native: bool,
    /// Proxy for all requests: http://, https://, socks4:// or socks5:// followed by host:port.
    #[arg(long, env = "SEESEA_PROXY", global = true)]
    proxy: Option<String>,
    /// Where to keep the cache between runs.
    #[arg(long, env = "SEESEA_CACHE_DIR", global = true)]
    cache_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    no_cache: bool,
    /// Allow fetching loopback and private addresses.
    #[arg(long, global = true)]
    allow_private: bool,
    /// Print JSON instead of Markdown.
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Search and read the best results.
    Search {
        query: Vec<String>,
        /// fast (snippets), standard (read the top results) or deep (read more of them).
        #[arg(short, long, default_value = "standard")]
        mode: String,
        #[arg(short = 'n', long, default_value_t = 10)]
        limit: usize,
        /// day, week, month or year.
        #[arg(long)]
        time_range: Option<String>,
        #[arg(long)]
        lang: Option<String>,
        /// Only these sources (comma separated).
        #[arg(short, long, value_delimiter = ',')]
        sources: Vec<String>,
        /// Cap on page text returned per result.
        #[arg(long, default_value_t = 3000)]
        max_chars: usize,
        /// Return whole pages instead of the relevant sections.
        #[arg(long)]
        full: bool,
        /// Append a JSON line per search to this file.
        #[arg(long)]
        log: Option<PathBuf>,
    },
    /// Read one page: a URL, a local HTML file, or `-` for standard input.
    Extract {
        target: String,
        /// Return only the sections that answer this.
        #[arg(short, long)]
        query: Option<String>,
        /// core or extended (also keeps comments and similar material).
        #[arg(long, default_value = "core")]
        scope: String,
        #[arg(long, default_value_t = 6000)]
        max_chars: usize,
    },
    /// List the sources, or with --check ask each one a question and report how it did.
    Sources {
        #[arg(long)]
        check: bool,
        #[arg(short, long, default_value = "rust programming language")]
        query: String,
    },
    /// Score the pipeline against judged queries (JSON lines: query and relevant URLs with grades).
    Eval {
        judgments: PathBuf,
        #[arg(short, default_value_t = 10)]
        k: usize,
        #[arg(short, long, default_value = "fast")]
        mode: String,
    },
    /// Run the HTTP server (JSON API and MCP on /mcp).
    Serve {
        #[arg(long, default_value = "127.0.0.1:8080")]
        addr: std::net::SocketAddr,
        /// Require this bearer token on every endpoint except /health.
        #[arg(long, env = "SEESEA_API_KEY", hide_env_values = true)]
        api_key: Option<String>,
        #[arg(long, default_value_t = 64)]
        max_concurrent: usize,
    },
    /// Run as an MCP server over standard input and output.
    Mcp,
}

fn settings(net: &NetArgs, memory_cache: bool) -> Settings {
    Settings {
        sources: SourceSettings { searxng_url: net.searxng.clone(), brave_key: net.brave_key.clone(), native_engines: !net.no_native },
        proxy: net.proxy.clone(),
        cache_dir: net.cache_dir.clone(),
        no_cache: net.no_cache,
        memory_cache,
        allow_private: net.allow_private,
    }
}

fn print_json<T: Serialize>(value: &T) -> Result<(), Error> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

async fn append_log(path: &PathBuf, resp: &SearchResponse) -> Result<(), Error> {
    let line = serde_json::json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "query": resp.query,
        "mode": resp.mode,
        "took_ms": resp.took_ms,
        "cached": resp.cached,
        "hits": resp.hits.iter().map(|h| serde_json::json!({"url": h.url, "score": h.score, "found_by": h.found_by})).collect::<Vec<_>>(),
        "failures": resp.failures,
    });
    let mut file = tokio::fs::OpenOptions::new().create(true).append(true).open(path).await?;
    file.write_all(format!("{line}\n").as_bytes()).await?;
    Ok(())
}

async fn run(cli: Cli) -> Result<(), Error> {
    let json = cli.net.json;
    let long_lived = matches!(cli.command, Command::Serve { .. } | Command::Mcp);
    let pipeline = Arc::new(build_pipeline(&settings(&cli.net, long_lived))?);

    match cli.command {
        Command::Search { query, mode, limit, time_range, lang, sources, max_chars, full, log } => {
            let mut req = SearchRequest::new(query.join(" "));
            req.mode = mode.parse::<Mode>()?;
            req.limit = limit;
            req.time_range = time_range.map(|t| t.parse::<TimeRange>()).transpose()?;
            req.language = lang;
            req.sources = sources;
            req.max_chars_per_result = max_chars;
            req.full_content = full;
            let resp = pipeline.search(&req).await?;
            if let Some(path) = &log {
                append_log(path, &resp).await?;
            }
            if json { print_json(&resp) } else { Ok(print!("{}", search_markdown(&resp))) }
        }
        Command::Extract { target, query, scope, max_chars } => {
            let scope = parse_scope(Some(&scope))?;
            let page = if target.starts_with("http://") || target.starts_with("https://") {
                pipeline.extract(&target, scope, query.as_deref(), max_chars).await?
            } else if target == "-" {
                let mut html = String::new();
                std::io::stdin().read_to_string(&mut html)?;
                pipeline.extract_html(html, "stdin://", scope, query.as_deref(), max_chars).await?
            } else {
                let html = std::fs::read(&target).map_err(|e| Error::InvalidRequest(format!("cannot read '{target}': {e}")))?;
                let abs = std::fs::canonicalize(&target).unwrap_or_else(|_| PathBuf::from(&target));
                pipeline.extract_html(String::from_utf8_lossy(&html).into_owned(), &format!("file://{}", abs.display()), scope, query.as_deref(), max_chars).await?
            };
            if json { print_json(&page) } else { Ok(print!("{}", extract_markdown(&page))) }
        }
        Command::Sources { check, query } => {
            if !check {
                let names = pipeline.source_names();
                return if json { print_json(&serde_json::json!({"sources": names})) } else { Ok(println!("{}", names.join("\n"))) };
            }
            let probes = pipeline.probe(&query).await;
            if json {
                print_json(&serde_json::json!({"sources": probes}))?;
            } else {
                for p in &probes {
                    match &p.error {
                        None => println!("ok    {:<10} {:>2} results  {:>5} ms", p.source, p.hits, p.ms),
                        Some(e) => println!("FAIL  {:<10} {:>5} ms  {:?}: {}", p.source, p.ms, e.kind, e.message),
                    }
                }
            }
            match probes.iter().any(|p| p.ok) {
                true => Ok(()),
                false if probes.is_empty() => Err(Error::NoSources),
                false => Err(Error::AllSourcesFailed(probes.into_iter().filter_map(|p| p.error).collect())),
            }
        }
        Command::Eval { judgments, k, mode } => {
            let text = std::fs::read_to_string(&judgments).map_err(|e| Error::InvalidRequest(format!("cannot read '{}': {e}", judgments.display())))?;
            let list = eval::parse_judgments(&text)?;
            let mut template = SearchRequest::new("x");
            template.mode = mode.parse::<Mode>()?;
            let report = eval::run(&pipeline, &list, &template, k).await;
            if json {
                print_json(&report)
            } else {
                for q in &report.queries {
                    println!("ndcg@{k} {:.3}  recall {:.3}  mrr {:.3}  {:>5} ms  {}{}", q.ndcg, q.recall, q.mrr, q.took_ms, q.query, q.error.as_deref().map(|e| format!("  [{e}]")).unwrap_or_default());
                }
                println!("\nmean ndcg@{k} {:.3}  recall {:.3}  mrr {:.3}  ({} queries, {} failed)", report.mean_ndcg, report.mean_recall, report.mean_mrr, report.queries.len(), report.failed_queries);
                Ok(())
            }
        }
        Command::Serve { addr, api_key, max_concurrent } => server::serve(addr, AppState::new(pipeline, api_key), max_concurrent).await,
        Command::Mcp => {
            // Standard output belongs to the protocol; anything else goes to standard error.
            eprintln!("seesea MCP server ready on stdio");
            Arc::new(McpServer::new(pipeline)).serve(BufReader::new(tokio::io::stdin()), tokio::io::stdout()).await?;
            Ok(())
        }
    }
}

fn report_error(e: &Error, json: bool) {
    if json {
        eprintln!("{}", serde_json::to_string(&e.body()).unwrap_or_else(|_| e.to_string()));
    } else {
        eprintln!("error[{}]: {e}", e.code());
        if let Error::AllSourcesFailed(fs) = e {
            for f in fs {
                eprintln!("  {} ({:?}): {}", f.source, f.kind, f.message);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let json = cli.net.json;
    if let Err(e) = run(cli).await {
        report_error(&e, json);
        std::process::exit(e.exit_code());
    }
}

#[allow(dead_code)]
fn _assert_pipeline_is_send_sync(p: Pipeline) -> impl Send + Sync {
    p
}
