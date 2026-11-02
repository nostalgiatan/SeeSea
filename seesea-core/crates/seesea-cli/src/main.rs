//! `seesea`: search, extract, serve and MCP from one binary.

use clap::{Args, Parser, Subcommand};
use seesea_cli::config::{Settings, build_pipeline};
use seesea_cli::userconfig::{self, FileConfig};
use seesea_cli::models;
use seesea_cli::server::{self, AppState, parse_scope};
use seesea_cli::sources::SourceSettings;
use seesea_mcp::McpServer;
use seesea_pipeline::render::{extract_markdown, search_markdown};
use seesea_pipeline::{Error, Mode, Pipeline, SearchRequest, SearchResponse, TimeRange};
use seesea_pipeline::{eval, replay};
use serde::Serialize;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncWriteExt, BufReader};

#[derive(Parser)]
#[command(
    name = "seesea",
    version,
    about = "Search the web, read the results: one pipeline for the command line, an HTTP server and MCP"
)]
struct Cli {
    #[command(flatten)]
    net: NetArgs,
    #[command(subcommand)]
    command: Command,
}

#[derive(Args)]
struct NetArgs {
    /// SQLite file with users and API keys (default: next to the models).
    #[arg(long, env = "SEESEA_AUTH_DB", global = true)]
    auth_db: Option<PathBuf>,
    /// Config file (default ~/.config/seesea/config.toml). Flags and SEESEA_* variables win over it.
    #[arg(long, env = "SEESEA_CONFIG", global = true)]
    config: Option<PathBuf>,
    /// Address of a SearXNG instance with the json format enabled.
    #[arg(long, env = "SEESEA_SEARXNG_URL", global = true)]
    searxng: Option<String>,
    /// Brave Search API key.
    #[arg(
        long,
        env = "SEESEA_BRAVE_API_KEY",
        global = true,
        hide_env_values = true
    )]
    brave_key: Option<String>,
    /// Do not use the built-in Bing scraper.
    #[arg(long, global = true)]
    no_native: bool,
    /// Proxy for all requests: http://, https://, socks4:// or socks5:// followed by host:port.
    #[arg(long, env = "SEESEA_PROXY", global = true)]
    proxy: Option<String>,
    /// Where to keep the cache between runs.
    #[arg(long, env = "SEESEA_CACHE_DIR", global = true)]
    cache_dir: Option<PathBuf>,
    /// Reuse cached results for queries with this much word overlap (0..1, e.g. 0.8). Off by default.
    #[arg(long, env = "SEESEA_FUZZY_CACHE", global = true)]
    fuzzy_cache: Option<f64>,
    /// Directory of an embedding model (config.json, tokenizer.json, model.safetensors) for semantic reranking.
    #[arg(long, env = "SEESEA_MODEL_DIR", global = true)]
    model_dir: Option<PathBuf>,
    /// Share of content relevance taken from the embedding score, 0..1 (default 0.8).
    #[arg(long, env = "SEESEA_SEMANTIC_WEIGHT", global = true)]
    semantic_weight: Option<f64>,
    /// Passages embedded per page: the opening one plus the best matches (default 4). Lower is faster.
    #[arg(long, env = "SEESEA_SEMANTIC_PASSAGES", global = true)]
    semantic_passages: Option<usize>,
    /// FLOOR,CEIL: cosine similarities mapped to relevance 0 and 1 (default 0.3,0.75 suits bge-m3; BGE-small English ~0.45,0.85).
    #[arg(long, env = "SEESEA_SEMANTIC_RANGE", global = true)]
    semantic_range: Option<String>,
    /// A cross-encoder reranker (.gguf, e.g. bge-reranker-v2-m3): reads query and passage together. Slower, more precise.
    #[arg(long, env = "SEESEA_RERANK_MODEL", global = true)]
    rerank_model: Option<PathBuf>,
    /// Share of content relevance taken from the cross-encoder, 0..1 (default 0.5).
    #[arg(long, env = "SEESEA_RERANK_WEIGHT", global = true)]
    rerank_weight: Option<f64>,
    /// Passages the cross-encoder scores per page (default 3). Lower is faster.
    #[arg(long, env = "SEESEA_RERANK_PASSAGES", global = true)]
    rerank_passages: Option<usize>,
    /// Cross-encoder logit that maps to relevance 0.5 (default -4; the model is strict).
    #[arg(
        long,
        env = "SEESEA_RERANK_CENTER",
        global = true,
        allow_negative_numbers = true
    )]
    rerank_center: Option<f32>,
    /// Logit distance of one sigmoid step (default 3). Larger spreads scores out, smaller sharpens them.
    #[arg(long, env = "SEESEA_RERANK_SCALE", global = true)]
    rerank_scale: Option<f32>,
    /// Choose page sections by shared words only, even when a model is loaded (faster).
    #[arg(long, env = "SEESEA_LEXICAL_SECTIONS", global = true)]
    lexical_sections: bool,
    /// Fetch pages with plain headers instead of browser-like ones (SEESEA_USER_AGENT sets the user agent otherwise).
    #[arg(long, global = true)]
    plain_headers: bool,
    /// Lightpanda binary used to render pages that need JavaScript (default: `lightpanda` on PATH).
    #[arg(long, env = "SEESEA_LIGHTPANDA", global = true)]
    lightpanda: Option<PathBuf>,
    /// Never render pages with a headless browser.
    #[arg(long, global = true)]
    no_render: bool,
    /// Do not adapt stage-1 ranking to what earlier searches found.
    #[arg(long, global = true)]
    no_learn: bool,
    /// Chance (0..1) per search of also reading one lower-ranked result.
    #[arg(long, env = "SEESEA_EXPLORE", global = true)]
    explore: Option<f64>,
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
    /// Replay a search log (from `search --log`) against cache policies: hit ratio, and how much
    /// the cached results differed from what the real search returned.
    Replay {
        log: PathBuf,
        /// How long a cached entry lives, in seconds.
        #[arg(long, default_value_t = 600)]
        ttl: i64,
        /// Fuzzy thresholds to try.
        #[arg(long, value_delimiter = ',', default_values_t = [0.9, 0.8, 0.7])]
        fuzzy: Vec<f64>,
    },
    /// Check an embedding model: load it, embed a few sentences, report width, speed and similarities.
    Model {
        /// The model directory (defaults to --model-dir / SEESEA_MODEL_DIR).
        dir: Option<PathBuf>,
    },
    /// Show where the config file is, or with --init write a commented template there.
    Config {
        #[arg(long)]
        init: bool,
    },
    /// Check the models seesea can use; `models download` fetches the missing ones.
    Models {
        #[command(subcommand)]
        action: Option<ModelsAction>,
    },
    /// Manage API users (accounts) for `seesea serve`.
    Users {
        #[command(subcommand)]
        action: UsersAction,
    },
    /// Manage API keys.
    Keys {
        #[command(subcommand)]
        action: KeysAction,
    },
    /// Credits a user spent per day.
    Usage {
        user: String,
        #[arg(long, default_value_t = 14)]
        days: u32,
    },
    /// Tell stage 1 whether a result was good or bad, so its host ranks higher or lower next time.
    Feedback {
        url: String,
        #[arg(value_parser = ["good", "bad"])]
        verdict: String,
    },
    /// Inspect or clear what stage 1 has learned from earlier searches.
    Learn {
        #[command(subcommand)]
        action: LearnAction,
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
        /// Listen on a non-loopback address although no API key or user exists.
        #[arg(long)]
        allow_open: bool,
    },
    /// Run as an MCP server over standard input and output.
    Mcp,
}

#[derive(Subcommand)]
enum LearnAction {
    /// The best and worst hosts and every source, by the value their pages delivered.
    Show {
        #[arg(long, default_value_t = 15)]
        top: usize,
    },
    /// Forget everything.
    Reset,
}

#[derive(Subcommand)]
enum UsersAction {
    /// Create a user. The plan is one defined in the config file (default: `default`).
    Add {
        name: String,
        #[arg(long, default_value = "default")]
        plan: String,
    },
    List,
    /// Refuse every key of this user until `enable`.
    Disable { name: String },
    Enable { name: String },
    /// Move a user to another plan.
    Plan { name: String, plan: String },
}

#[derive(Subcommand)]
enum KeysAction {
    /// Create a key; it is printed once and cannot be shown again.
    Create {
        user: String,
        /// A label to tell keys apart.
        #[arg(long, default_value = "default")]
        name: String,
        #[arg(long)]
        expires_days: Option<u32>,
    },
    List { user: String },
    /// Revoke a key by its public id (`ss_xxxxxxxx`, shown by `keys list`).
    Revoke { id: String },
}

#[derive(Subcommand)]
enum ModelsAction {
    /// Download models (embedding, reranker; all of them when none is named) unless already present.
    Download { names: Vec<String> },
}

/// Fill what flags and environment left empty from the config file, then from models found on disk.
fn apply_file(net: &mut NetArgs, f: &FileConfig, use_model: bool) {
    fn fill<T: Clone>(slot: &mut Option<T>, from: &Option<T>) {
        if slot.is_none() {
            slot.clone_from(from);
        }
    }
    fill(&mut net.searxng, &f.searxng);
    fill(&mut net.brave_key, &f.brave_key);
    fill(&mut net.proxy, &f.proxy);
    fill(&mut net.lightpanda, &f.lightpanda);
    fill(&mut net.auth_db, &f.auth_db);
    net.no_render |= f.no_render.unwrap_or(false);
    net.no_learn |= f.no_learn.unwrap_or(false);
    fill(&mut net.explore, &f.explore);
    fill(&mut net.cache_dir, &f.cache_dir);
    fill(&mut net.fuzzy_cache, &f.fuzzy_cache);
    fill(&mut net.model_dir, &f.models.embedding);
    fill(&mut net.rerank_model, &f.models.reranker);
    fill(&mut net.semantic_weight, &f.semantic.weight);
    fill(&mut net.semantic_passages, &f.semantic.passages);
    fill(&mut net.semantic_range, &f.semantic.range);
    fill(&mut net.rerank_weight, &f.rerank.weight);
    fill(&mut net.rerank_passages, &f.rerank.passages);
    fill(&mut net.rerank_center, &f.rerank.center);
    fill(&mut net.rerank_scale, &f.rerank.scale);
    net.no_native |= f.no_native.unwrap_or(false);
    net.allow_private |= f.allow_private.unwrap_or(false);
    net.plain_headers |= f.plain_headers.unwrap_or(false);
    net.lexical_sections |= f.lexical_sections.unwrap_or(false);
    if use_model {
        let (embedding, reranker) = models::discovered(f);
        fill(&mut net.model_dir, &embedding);
        fill(&mut net.rerank_model, &reranker);
    }
}

fn settings(net: &NetArgs, memory_cache: bool, use_model: bool) -> Settings {
    Settings {
        sources: SourceSettings {
            searxng_url: net.searxng.clone(),
            brave_key: net.brave_key.clone(),
            native_engines: !net.no_native,
        },
        proxy: net.proxy.clone(),
        cache_dir: net.cache_dir.clone(),
        no_cache: net.no_cache,
        memory_cache,
        allow_private: net.allow_private,
        fuzzy_cache: net.fuzzy_cache,
        model_dir: net.model_dir.clone().filter(|_| use_model),
        semantic_weight: net.semantic_weight,
        semantic_passages: net.semantic_passages,
        semantic_range: net.semantic_range.clone(),
        rerank_model: net.rerank_model.clone().filter(|_| use_model),
        rerank_weight: net.rerank_weight,
        rerank_passages: net.rerank_passages,
        rerank_center: net.rerank_center,
        rerank_scale: net.rerank_scale,
        lexical_sections: net.lexical_sections,
        plain_headers: net.plain_headers,
        lightpanda: net.lightpanda.clone(),
        no_render: net.no_render,
        learn_file: (!net.no_learn).then(userconfig::default_learn_file),
        explore: net.explore.unwrap_or(0.0),
    }
}

fn open_auth(net: &NetArgs) -> Result<seesea_auth::Store, Error> {
    let path = net.auth_db.clone().unwrap_or_else(userconfig::default_auth_db);
    seesea_auth::Store::open(&path)
        .map_err(|e| Error::Config(format!("{}: {e}", path.display())))
}

fn account_error(e: seesea_auth::AuthError) -> Error {
    match e {
        seesea_auth::AuthError::NotFound(_) | seesea_auth::AuthError::Exists(_) => Error::InvalidRequest(e.to_string()),
        other => Error::Config(other.to_string()),
    }
}

fn when(t: i64) -> String {
    chrono::DateTime::from_timestamp(t, 0).map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default()
}

fn manage_accounts(command: &Command, store: &seesea_auth::Store, file: &FileConfig, json: bool) -> Result<(), Error> {
    let known_plans = {
        let mut p = seesea_auth::default_plans();
        p.extend(file.plans.clone());
        p
    };
    match command {
        Command::Users { action } => match action {
            UsersAction::Add { name, plan } => {
                if !known_plans.contains_key(plan) {
                    let mut names: Vec<_> = known_plans.keys().cloned().collect();
                    names.sort();
                    return Err(Error::InvalidRequest(format!("unknown plan '{plan}' (defined: {})", names.join(", "))));
                }
                let u = store.create_user(name, plan).map_err(account_error)?;
                println!("created user {} (plan {}); `seesea keys create {}` makes a key", u.name, u.plan, u.name);
            }
            UsersAction::List => {
                let users = store.users().map_err(account_error)?;
                if json {
                    let rows: Vec<_> = users.iter().map(|u| serde_json::json!({"name": u.name, "plan": u.plan, "disabled": u.disabled})).collect();
                    return print_json(&rows);
                }
                for u in users {
                    println!("{:<20} plan {:<12} {}  created {}", u.name, u.plan, if u.disabled { "DISABLED" } else { "active  " }, when(u.created));
                }
            }
            UsersAction::Disable { name } => {
                store.set_disabled(name, true).map_err(account_error)?;
                println!("{name} is disabled");
            }
            UsersAction::Enable { name } => {
                store.set_disabled(name, false).map_err(account_error)?;
                println!("{name} is enabled");
            }
            UsersAction::Plan { name, plan } => {
                if !known_plans.contains_key(plan) {
                    return Err(Error::InvalidRequest(format!("unknown plan '{plan}'")));
                }
                store.set_plan(name, plan).map_err(account_error)?;
                println!("{name} is on plan {plan}");
            }
        },
        Command::Keys { action } => match action {
            KeysAction::Create { user, name, expires_days } => {
                let (info, key) = store.create_key(user, name, *expires_days).map_err(account_error)?;
                if json {
                    return print_json(&serde_json::json!({"id": info.public_id, "key": key}));
                }
                println!("{key}\n\nkey {} for {user}: this is the only time it is shown. Send it as `Authorization: Bearer <key>`.", info.public_id);
            }
            KeysAction::List { user } => {
                for k in store.keys(user).map_err(account_error)? {
                    println!(
                        "{}  {:<16} {}  created {}  last used {}",
                        k.public_id,
                        k.name,
                        if k.revoked { "REVOKED" } else if k.expires.is_some_and(|e| e <= chrono::Utc::now().timestamp()) { "EXPIRED" } else { "active " },
                        when(k.created),
                        k.last_used.map(when).unwrap_or_else(|| "never".into())
                    );
                }
            }
            KeysAction::Revoke { id } => {
                store.revoke_key(id).map_err(account_error)?;
                println!("{id} is revoked");
            }
        },
        Command::Usage { user, days } => {
            let history = store.usage_history(user, *days).map_err(account_error)?;
            if json {
                let rows: Vec<_> = history.iter().map(|(d, c)| serde_json::json!({"day": d, "credits": c})).collect();
                return print_json(&rows);
            }
            for (day, credits) in history {
                let date = chrono::DateTime::from_timestamp(day as i64 * 86400, 0).map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
                println!("{date}  {credits} credits");
            }
        }
        _ => unreachable!(),
    }
    Ok(())
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
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    file.write_all(format!("{line}\n").as_bytes()).await?;
    Ok(())
}

async fn run(mut cli: Cli) -> Result<(), Error> {
    let json = cli.net.json;
    let config_path = cli.net.config.clone().unwrap_or_else(userconfig::default_path);
    let file = userconfig::load(&config_path)?.unwrap_or_default();
    let long_lived = matches!(cli.command, Command::Serve { .. } | Command::Mcp);
    // The model is only loaded (in the background) for commands that rerank page content.
    let use_model = match &cli.command {
        Command::Search { mode, .. } | Command::Eval { mode, .. } => {
            !mode.eq_ignore_ascii_case("fast")
        }
        Command::Serve { .. } | Command::Mcp => true,
        // Models only help `extract` choose sections for a query.
        Command::Extract { query, .. } => query.is_some(),
        _ => false,
    };
    apply_file(&mut cli.net, &file, use_model);

    match &cli.command {
        Command::Config { init } => {
            let exists = config_path.exists();
            if *init && !exists {
                if let Some(dir) = config_path.parent() {
                    std::fs::create_dir_all(dir)?;
                }
                std::fs::write(&config_path, userconfig::TEMPLATE)?;
                println!("wrote {}", config_path.display());
            } else if *init {
                println!("{} already exists; not touching it", config_path.display());
            } else {
                println!(
                    "{} ({})",
                    config_path.display(),
                    if exists { "in use" } else { "not there; `seesea config --init` writes a template" }
                );
            }
            return Ok(());
        }
        Command::Models { action } => {
            let wanted: Vec<String> = match action {
                Some(ModelsAction::Download { names }) => names.clone(),
                None => {
                    let st = models::status(&file);
                    if json {
                        return print_json(&st);
                    }
                    println!("models directory: {}", models::models_dir(&file).display());
                    match seesea_fetch::render::Renderer::discover(cli.net.lightpanda.as_deref()) {
                        Some(r) => println!("renderer   ok       {}  Lightpanda renders pages that need JavaScript", r.binary().display()),
                        None => println!("renderer   missing  Lightpanda not found: put `lightpanda` on PATH or set `lightpanda` in the config (optional)"),
                    }
                    for s in &st {
                        println!(
                            "{:<10} {:<8} {}  {}",
                            s.model,
                            match (s.present, s.valid) {
                                (true, true) => "ok",
                                (true, false) => "BROKEN",
                                _ => "missing",
                            },
                            s.path.as_deref().map(|p| p.display().to_string()).unwrap_or_default(),
                            s.note
                        );
                    }
                    return Ok(());
                }
            };
            for name in &wanted {
                if !models::CATALOG.iter().any(|s| s.key == name) {
                    return Err(Error::InvalidRequest(format!(
                        "unknown model '{name}' (embedding, reranker)"
                    )));
                }
            }
            let st = models::status(&file);
            for (spec, state) in models::CATALOG.iter().zip(&st) {
                if !wanted.is_empty() && !wanted.iter().any(|n| n == spec.key) {
                    continue;
                }
                if state.valid {
                    println!("{}: already at {}", spec.key, state.path.as_deref().map(|p| p.display().to_string()).unwrap_or_default());
                    continue;
                }
                let path = models::download(spec, &file, cli.net.proxy.as_deref()).await?;
                println!("{}: saved {}", spec.key, path.display());
            }
            return Ok(());
        }
        _ => {}
    }
    match &cli.command {
        Command::Feedback { .. } | Command::Learn { .. } => {
            let path = userconfig::default_learn_file();
            let learner = seesea_pipeline::Learner::open(Default::default(), path.clone());
            match &cli.command {
                Command::Feedback { url, verdict } => {
                    let host = seesea_pipeline::host_of(url);
                    if host.is_empty() {
                        return Err(Error::InvalidRequest(format!("not a URL: {url}")));
                    }
                    learner.feedback(&host, verdict == "good");
                    println!("{host}: marked {verdict}");
                }
                Command::Learn { action: LearnAction::Reset } => {
                    learner.reset();
                    println!("forgot everything ({})", path.display());
                }
                Command::Learn { action: LearnAction::Show { top } } => {
                    let r = learner.report(*top);
                    if json {
                        return print_json(&r);
                    }
                    println!("{} pages judged, average value {:.2}, {} hosts ({})", r.observations.round(), r.average_value, r.hosts, path.display());
                    let show = |title: &str, rows: &[seesea_pipeline::learn::Standing]| {
                        println!("\n{title}");
                        for h in rows {
                            println!("  {:.2}  {:>6.1}  {}", h.value, h.observations, h.name);
                        }
                    };
                    show("best hosts", &r.best_hosts);
                    show("worst hosts", &r.worst_hosts);
                    show("sources", &r.sources);
                }
                _ => unreachable!(),
            }
            return Ok(());
        }
        _ => {}
    }
    match &cli.command {
        Command::Users { .. } | Command::Keys { .. } | Command::Usage { .. } => {
            let store = open_auth(&cli.net)?;
            return manage_accounts(&cli.command, &store, &file, json);
        }
        _ => {}
    }
    let mut pipeline_settings = settings(&cli.net, long_lived, use_model);
    // An evaluation must not change the ranking it is measuring.
    if matches!(cli.command, Command::Eval { .. }) {
        pipeline_settings.learn_file = None;
    }
    let pipeline = Arc::new(build_pipeline(&pipeline_settings)?);

    match cli.command {
        Command::Search {
            query,
            mode,
            limit,
            time_range,
            lang,
            sources,
            max_chars,
            full,
            log,
        } => {
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
            if json {
                print_json(&resp)
            } else {
                Ok(print!("{}", search_markdown(&resp)))
            }
        }
        Command::Extract {
            target,
            query,
            scope,
            max_chars,
        } => {
            let scope = parse_scope(Some(&scope))?;
            let page = if target.starts_with("http://") || target.starts_with("https://") {
                pipeline
                    .extract(&target, scope, query.as_deref(), max_chars)
                    .await?
            } else if target == "-" {
                let mut html = String::new();
                std::io::stdin().read_to_string(&mut html)?;
                pipeline
                    .extract_html(html, "stdin://", scope, query.as_deref(), max_chars)
                    .await?
            } else {
                let html = std::fs::read(&target)
                    .map_err(|e| Error::InvalidRequest(format!("cannot read '{target}': {e}")))?;
                let abs = std::fs::canonicalize(&target).unwrap_or_else(|_| PathBuf::from(&target));
                pipeline
                    .extract_html(
                        String::from_utf8_lossy(&html).into_owned(),
                        &format!("file://{}", abs.display()),
                        scope,
                        query.as_deref(),
                        max_chars,
                    )
                    .await?
            };
            if json {
                print_json(&page)
            } else {
                Ok(print!("{}", extract_markdown(&page)))
            }
        }
        Command::Sources { check, query } => {
            if !check {
                let names = pipeline.source_names();
                return if json {
                    print_json(&serde_json::json!({"sources": names}))
                } else {
                    Ok(println!("{}", names.join("\n")))
                };
            }
            let probes = pipeline.probe(&query).await;
            if json {
                print_json(&serde_json::json!({"sources": probes}))?;
            } else {
                for p in &probes {
                    match &p.error {
                        None => println!(
                            "ok    {:<10} {:>2} results  {:>5} ms",
                            p.source, p.hits, p.ms
                        ),
                        Some(e) => println!(
                            "FAIL  {:<10} {:>5} ms  {:?}: {}",
                            p.source, p.ms, e.kind, e.message
                        ),
                    }
                }
            }
            match probes.iter().any(|p| p.ok) {
                true => Ok(()),
                false if probes.is_empty() => Err(Error::NoSources),
                false => Err(Error::AllSourcesFailed(
                    probes.into_iter().filter_map(|p| p.error).collect(),
                )),
            }
        }
        Command::Model { dir } => {
            let dir = dir
                .or_else(|| cli.net.model_dir.clone())
                .ok_or_else(|| Error::InvalidRequest("give a model directory".into()))?;
            let started = std::time::Instant::now();
            let embedder = seesea_embed::Embedder::load(&dir).map_err(|e| {
                Error::Config(format!("cannot load the model in '{}': {e}", dir.display()))
            })?;
            let load_ms = started.elapsed().as_millis();
            let query = "how does the tokio scheduler work";
            let passages = [
                "Tokio schedules async tasks on a work-stealing thread pool, moving tasks between worker threads.",
                "Cheap watches and handbags at discount prices with free shipping on every order.",
            ];
            let t = std::time::Instant::now();
            let q = embedder
                .embed(&[query], seesea_embed::Kind::Query)
                .map_err(|e| Error::Config(e.to_string()))?;
            let p = embedder
                .embed(&passages, seesea_embed::Kind::Passage)
                .map_err(|e| Error::Config(e.to_string()))?;
            let embed_ms = t.elapsed().as_millis();
            let sims: Vec<f32> = p.iter().map(|v| seesea_embed::dot(&q[0], v)).collect();
            let report = serde_json::json!({
                "model": embedder.name(), "dim": embedder.dim(), "device": embedder.device_name(), "load_ms": load_ms,
                "embed_ms_for_3_texts": embed_ms,
                "related_similarity": sims[0], "unrelated_similarity": sims[1],
                "separates": sims[0] > sims[1] + 0.05,
            });
            if json {
                print_json(&report)
            } else {
                println!(
                    "{}: {} dimensions on the {}, loaded in {load_ms} ms, 3 texts in {embed_ms} ms\nrelated {:.3}, unrelated {:.3} -> {}",
                    embedder.name(),
                    embedder.dim(),
                    embedder.device_name(),
                    sims[0],
                    sims[1],
                    if sims[0] > sims[1] + 0.05 {
                        "ok"
                    } else {
                        "the model does not separate them: wrong pooling or prefixes?"
                    }
                );
                Ok(())
            }
        }
        Command::Replay { log, ttl, fuzzy } => {
            let text = std::fs::read_to_string(&log).map_err(|e| {
                Error::InvalidRequest(format!("cannot read '{}': {e}", log.display()))
            })?;
            let mut entries = replay::parse_log(&text)?;
            entries.sort_by_key(|e| e.ts);
            let mut policies = vec![replay::Policy::Exact, replay::Policy::Normalized];
            policies.extend(fuzzy.into_iter().map(replay::Policy::Fuzzy));
            let reports: Vec<_> = policies
                .into_iter()
                .map(|p| replay::replay(&entries, p, ttl))
                .collect();
            if json {
                print_json(&reports)
            } else {
                println!("{} searches, entries live {ttl} s", entries.len());
                for r in &reports {
                    println!(
                        "{:<14} hit ratio {:>5.1}%  ({} hits)  overlap {}  poor hits {}",
                        r.policy,
                        r.hit_ratio * 100.0,
                        r.hits,
                        r.mean_overlap
                            .map(|o| format!("{o:.2}"))
                            .unwrap_or_else(|| "-".into()),
                        r.poor_hits
                    );
                }
                Ok(())
            }
        }
        Command::Eval { judgments, k, mode } => {
            let text = std::fs::read_to_string(&judgments).map_err(|e| {
                Error::InvalidRequest(format!("cannot read '{}': {e}", judgments.display()))
            })?;
            let list = eval::parse_judgments(&text)?;
            let mut template = SearchRequest::new("x");
            template.mode = mode.parse::<Mode>()?;
            let report = eval::run(&pipeline, &list, &template, k).await;
            if json {
                print_json(&report)
            } else {
                for q in &report.queries {
                    println!(
                        "ndcg@{k} {:.3}  recall {:.3}  mrr {:.3}  {:>5} ms  {}{}",
                        q.ndcg,
                        q.recall,
                        q.mrr,
                        q.took_ms,
                        q.query,
                        q.error
                            .as_deref()
                            .map(|e| format!("  [{e}]"))
                            .unwrap_or_default()
                    );
                }
                println!(
                    "\nmean ndcg@{k} {:.3}  recall {:.3}  mrr {:.3}  ({} queries, {} failed)",
                    report.mean_ndcg,
                    report.mean_recall,
                    report.mean_mrr,
                    report.queries.len(),
                    report.failed_queries
                );
                Ok(())
            }
        }
        Command::Serve {
            addr,
            api_key,
            max_concurrent,
            allow_open,
        } => {
            let store = open_auth(&cli.net)?;
            let access = seesea_cli::access::Access::new(Some(store), file.plans.clone(), api_key);
            server::serve(addr, AppState::with_access(pipeline, access), max_concurrent, allow_open).await
        }
        Command::Config { .. }
        | Command::Models { .. }
        | Command::Users { .. }
        | Command::Keys { .. }
        | Command::Usage { .. }
        | Command::Feedback { .. }
        | Command::Learn { .. } => unreachable!("handled above"),
        Command::Mcp => {
            // Standard output belongs to the protocol; anything else goes to standard error.
            eprintln!("seesea MCP server ready on stdio");
            Arc::new(McpServer::new(pipeline))
                .serve(BufReader::new(tokio::io::stdin()), tokio::io::stdout())
                .await?;
            Ok(())
        }
    }
}

fn report_error(e: &Error, json: bool) {
    if json {
        eprintln!(
            "{}",
            serde_json::to_string(&e.body()).unwrap_or_else(|_| e.to_string())
        );
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
