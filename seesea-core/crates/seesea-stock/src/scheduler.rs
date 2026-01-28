//! Stock data rotation scheduler
//!
//! 定时轮询调度器，按照不同的时间间隔刷新股票数据缓存

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Local;
use parking_lot::RwLock;
use tracing::{debug, error, info, warn};

use crate::cached_client::get_cached_stock_client;
use crate::error::StockResult;
use crate::trading_days::TradingDaysManager;
use crate::types::RotationInterval;
use seesea_config::{DateStrategy, SchedulerConfig};

/// 调度任务
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// 任务名称
    pub name: String,

    ask_type: TaskType,
    /// 轮询间隔
    pub interval: RotationInterval,
    /// 是否启用
    pub enabled: bool,
    /// 日期策略
    pub date_strategy: DateStrategy,
    /// 指定日期
    pub specified_date: Option<String>,
}

/// 任务类型
#[derive(Debug, Clone)]
pub enum TaskType {
    /// A股代码名称
    StockInfoACodeName,
    /// 上海主板A股代码名称
    StockInfoShNameCodeA,
    /// 上海主板B股代码名称
    StockInfoShNameCodeB,
    /// 上海科创板代码名称
    StockInfoShNameCodeKcb,
    /// 深圳A股列表代码名称
    StockInfoSzNameCodeA,
    /// 深圳B股列表代码名称
    StockInfoSzNameCodeB,
    /// 深圳CDR列表代码名称
    StockInfoSzNameCodeCdr,
    /// 深圳AB股列表代码名称
    StockInfoSzNameCodeAb,
    /// A股实时行情
    StockZhASpotEm,
    /// B股实时行情
    StockZhBSpotEm,
    /// 美股实时行情
    StockUsSpotEm,
    /// 港股实时行情
    StockHkSpotEm,
    /// 行业板块名称
    StockBoardIndustryNameEm,
    /// 概念板块名称
    StockBoardConceptNameEm,
    /// 指数实时行情
    StockZhIndexSpotEm,
    /// 市场资金流向
    StockMarketFundFlow,
    /// 涨停池
    StockZtPoolEm,
    /// 跌停池
    StockDtPoolEm,
    /// 个股资金流向
    StockIndividualFundFlow,
    /// 个股信息
    StockIndividualInfoEm,
    /// 股票新闻
    StockNewsEm,
}

impl TaskType {
    /// 获取任务的缓存键（包括作用域）
    fn get_cache_key(&self) -> Option<(&'static str, &'static str)> {
        match self {
            TaskType::StockInfoACodeName => Some(("stock.info", "stock_info_a_code_name")),
            TaskType::StockInfoShNameCodeA => Some(("stock.info", "stock_info_sh_name_code_a")),
            TaskType::StockInfoShNameCodeB => Some(("stock.info", "stock_info_sh_name_code_b")),
            TaskType::StockInfoShNameCodeKcb => Some(("stock.info", "stock_info_sh_name_code_kcb")),
            TaskType::StockInfoSzNameCodeA => Some(("stock.info", "stock_info_sz_name_code_a")),
            TaskType::StockInfoSzNameCodeB => Some(("stock.info", "stock_info_sz_name_code_b")),
            TaskType::StockInfoSzNameCodeCdr => Some(("stock.info", "stock_info_sz_name_code_cdr")),
            TaskType::StockInfoSzNameCodeAb => Some(("stock.info", "stock_info_sz_name_code_ab")),
            TaskType::StockZhASpotEm => Some(("stock.quote", "stock_zh_a_spot_em")),
            TaskType::StockZhBSpotEm => Some(("stock.quote", "stock_zh_b_spot_em")),
            TaskType::StockUsSpotEm => Some(("stock.quote", "stock_us_spot_em")),
            TaskType::StockHkSpotEm => Some(("stock.quote", "stock_hk_spot_em")),
            TaskType::StockBoardIndustryNameEm => {
                Some(("stock.industry", "stock_board_industry_name_em"))
            }
            TaskType::StockBoardConceptNameEm => {
                Some(("stock.industry", "stock_board_concept_name_em"))
            }
            TaskType::StockZhIndexSpotEm => Some(("stock.index", "stock_zh_index_spot_em")),
            TaskType::StockMarketFundFlow => Some(("stock.fund_flow", "stock_market_fund_flow")),
            TaskType::StockZtPoolEm => Some(("stock.ranking", "stock_zt_pool_em")),
            TaskType::StockDtPoolEm => Some(("stock.ranking", "stock_zt_pool_dtgc_em")),
            TaskType::StockIndividualFundFlow => None,
            TaskType::StockIndividualInfoEm => None,
            TaskType::StockNewsEm => None,
        }
    }

    /// 检查缓存是否需要刷新
    fn needs_refresh(&self) -> bool {
        let (scope, cache_key) = match self.get_cache_key() {
            Some((scope, key)) => (scope, key),
            None => return true,
        };

        let client = match get_cached_stock_client() {
            Ok(c) => c,
            Err(_) => return true,
        };

        let interval = self.default_interval();
        let interval_duration = interval.as_duration();

        match client.cache.get_metadata(scope, cache_key) {
            Ok(Some(metadata)) => {
                let now = std::time::SystemTime::now();
                match now.duration_since(metadata.created_at) {
                    Ok(elapsed) => {
                        let needs_refresh = elapsed >= interval_duration;
                        if needs_refresh {
                            info!(
                                "🔄 缓存过期，需要刷新: {} (已过 {:.1} 分钟)",
                                self.display_name(),
                                elapsed.as_secs_f64() / 60.0
                            );
                        } else {
                            info!(
                                "✅ 缓存有效，跳过刷新: {} (剩余 {:.1} 分钟)",
                                self.display_name(),
                                (interval_duration - elapsed).as_secs_f64() / 60.0
                            );
                        }
                        needs_refresh
                    }
                    Err(_) => true,
                }
            }
            Ok(None) => {
                info!("📭 缓存不存在，需要刷新: {}", self.display_name());
                true
            }
            Err(_) => {
                warn!("⚠️ 检查缓存元数据失败: {}", self.display_name());
                true
            }
        }
    }

    /// 执行任务（刷新缓存，不从缓存读取）
    pub fn execute(&self, date_param: Option<&str>) -> StockResult<()> {
        let client = get_cached_stock_client()?;

        match self {
            TaskType::StockInfoACodeName => {
                client.refresh_stock_info_a_code_name()?;
            }
            TaskType::StockInfoShNameCodeA => {
                client.refresh_stock_info_sh_name_code(Some("主板A股"))?;
            }
            TaskType::StockInfoShNameCodeB => {
                client.refresh_stock_info_sh_name_code(Some("主板B股"))?;
            }
            TaskType::StockInfoShNameCodeKcb => {
                client.refresh_stock_info_sh_name_code(Some("科创板"))?;
            }
            TaskType::StockInfoSzNameCodeA => {
                client.refresh_stock_info_sz_name_code(Some("A股列表"))?;
            }
            TaskType::StockInfoSzNameCodeB => {
                client.refresh_stock_info_sz_name_code(Some("B股列表"))?;
            }
            TaskType::StockInfoSzNameCodeCdr => {
                client.refresh_stock_info_sz_name_code(Some("CDR列表"))?;
            }
            TaskType::StockInfoSzNameCodeAb => {
                client.refresh_stock_info_sz_name_code(Some("AB股列表"))?;
            }
            TaskType::StockZhASpotEm => {
                client.refresh_stock_zh_a_spot_em()?;
            }
            TaskType::StockZhBSpotEm => {
                client.refresh_stock_zh_b_spot_em()?;
            }
            TaskType::StockUsSpotEm => {
                client.refresh_stock_us_spot_em()?;
            }
            TaskType::StockHkSpotEm => {
                client.refresh_stock_hk_spot_em()?;
            }
            TaskType::StockBoardIndustryNameEm => {
                client.refresh_stock_board_industry_name_em()?;
            }
            TaskType::StockBoardConceptNameEm => {
                client.refresh_stock_board_concept_name_em()?;
            }
            TaskType::StockZhIndexSpotEm => {
                client.refresh_stock_zh_index_spot_em()?;
            }
            TaskType::StockMarketFundFlow => {
                client.refresh_stock_market_fund_flow()?;
            }
            TaskType::StockZtPoolEm => {
                client.refresh_stock_zt_pool_em(date_param)?;
            }
            TaskType::StockDtPoolEm => {
                client.refresh_stock_dt_pool_em(date_param)?;
            }
            TaskType::StockIndividualFundFlow => {
                info!("⚠️ 个股资金流向需要指定股票代码，暂不支持自动刷新");
            }
            TaskType::StockIndividualInfoEm => {
                info!("⚠️ 个股信息需要指定股票代码，暂不支持自动刷新");
            }
            TaskType::StockNewsEm => {
                info!("⚠️ 股票新闻需要指定股票代码，暂不支持自动刷新");
            }
        }

        Ok(())
    }

    /// 获取默认轮询间隔
    pub fn default_interval(&self) -> RotationInterval {
        match self {
            // 基础信息：长期轮询（11.5小时）
            TaskType::StockInfoACodeName
            | TaskType::StockInfoShNameCodeA
            | TaskType::StockInfoShNameCodeB
            | TaskType::StockInfoShNameCodeKcb
            | TaskType::StockInfoSzNameCodeA
            | TaskType::StockInfoSzNameCodeB
            | TaskType::StockInfoSzNameCodeCdr
            | TaskType::StockInfoSzNameCodeAb
            | TaskType::StockBoardIndustryNameEm
            | TaskType::StockBoardConceptNameEm => RotationInterval::LongTerm,

            // 实时行情：实时轮询（5分钟）
            TaskType::StockZhASpotEm
            | TaskType::StockZhBSpotEm
            | TaskType::StockUsSpotEm
            | TaskType::StockHkSpotEm
            | TaskType::StockZhIndexSpotEm
            | TaskType::StockZtPoolEm
            | TaskType::StockDtPoolEm => RotationInterval::Realtime,

            // 资金流向：短期轮询（15分钟）
            TaskType::StockMarketFundFlow => RotationInterval::ShortTerm,

            // 个股数据：不自动刷新
            TaskType::StockIndividualFundFlow
            | TaskType::StockIndividualInfoEm
            | TaskType::StockNewsEm => RotationInterval::LongTerm,
        }
    }

    /// 获取任务显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            TaskType::StockInfoACodeName => "A股代码名称",
            TaskType::StockInfoShNameCodeA => "上海主板A股",
            TaskType::StockInfoShNameCodeB => "上海主板B股",
            TaskType::StockInfoShNameCodeKcb => "上海科创板",
            TaskType::StockInfoSzNameCodeA => "深圳A股列表",
            TaskType::StockInfoSzNameCodeB => "深圳B股列表",
            TaskType::StockInfoSzNameCodeCdr => "深圳CDR列表",
            TaskType::StockInfoSzNameCodeAb => "深圳AB股列表",
            TaskType::StockZhASpotEm => "A股实时行情",
            TaskType::StockZhBSpotEm => "B股实时行情",
            TaskType::StockUsSpotEm => "美股实时行情",
            TaskType::StockHkSpotEm => "港股实时行情",
            TaskType::StockBoardIndustryNameEm => "行业板块",
            TaskType::StockBoardConceptNameEm => "概念板块",
            TaskType::StockZhIndexSpotEm => "指数行情",
            TaskType::StockMarketFundFlow => "市场资金流向",
            TaskType::StockZtPoolEm => "涨停池",
            TaskType::StockDtPoolEm => "跌停池",
            TaskType::StockIndividualFundFlow => "个股资金流向",
            TaskType::StockIndividualInfoEm => "个股信息",
            TaskType::StockNewsEm => "股票新闻",
        }
    }
}

/// 调度器
pub struct StockScheduler {
    /// 是否正在运行
    running: Arc<AtomicBool>,
    /// 任务列表
    tasks: Arc<RwLock<Vec<ScheduledTask>>>,
    /// 活跃任务计数
    active_tasks: Arc<AtomicUsize>,
}

impl StockScheduler {
    /// 创建新的调度器
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            tasks: Arc::new(RwLock::new(Vec::new())),
            active_tasks: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// 添加默认任务
    pub fn add_default_tasks(&self) {
        self.add_tasks_from_config(&SchedulerConfig::default());
    }

    /// 从配置添加任务
    pub fn add_tasks_from_config(&self, config: &SchedulerConfig) {
        let mut tasks = self.tasks.write();

        for task_config in &config.tasks {
            if !task_config.enabled {
                continue;
            }

            let task_type = match task_config.task_type.as_str() {
                "stock_info_a_code_name" => TaskType::StockInfoACodeName,
                "stock_info_sh_name_code_a" => TaskType::StockInfoShNameCodeA,
                "stock_info_sh_name_code_b" => TaskType::StockInfoShNameCodeB,
                "stock_info_sh_name_code_kcb" => TaskType::StockInfoShNameCodeKcb,
                "stock_info_sz_name_code_a" => TaskType::StockInfoSzNameCodeA,
                "stock_info_sz_name_code_b" => TaskType::StockInfoSzNameCodeB,
                "stock_info_sz_name_code_cdr" => TaskType::StockInfoSzNameCodeCdr,
                "stock_info_sz_name_code_ab" => TaskType::StockInfoSzNameCodeAb,
                "stock_zh_a_spot_em" => TaskType::StockZhASpotEm,
                "stock_zh_b_spot_em" => TaskType::StockZhBSpotEm,
                "stock_us_spot_em" => TaskType::StockUsSpotEm,
                "stock_hk_spot_em" => TaskType::StockHkSpotEm,
                "stock_board_industry_name_em" => TaskType::StockBoardIndustryNameEm,
                "stock_board_concept_name_em" => TaskType::StockBoardConceptNameEm,
                "stock_zh_index_spot_em" => TaskType::StockZhIndexSpotEm,
                "stock_market_fund_flow" => TaskType::StockMarketFundFlow,
                "stock_zt_pool_em" => TaskType::StockZtPoolEm,
                "stock_dt_pool_em" => TaskType::StockDtPoolEm,
                _ => {
                    warn!("未知的任务类型: {}", task_config.task_type);
                    continue;
                }
            };

            let interval = task_config
                .custom_interval
                .map(RotationInterval::Custom)
                .unwrap_or_else(|| task_type.default_interval());

            tasks.push(ScheduledTask {
                name: task_config.name.clone(),
                ask_type: task_type,
                interval,
                enabled: task_config.enabled,
                date_strategy: task_config.date_strategy,
                specified_date: task_config.specified_date.clone(),
            });
        }

        info!("从配置加载了 {} 个任务", tasks.len());
    }

    /// 添加自定义任务
    pub fn add_task(&self, task: ScheduledTask) {
        self.tasks.write().push(task);
    }

    /// 获取运行状态的 Arc 引用
    pub fn running_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.running)
    }

    /// 启动调度器（后台线程，不阻塞）
    pub fn start(&self) {
        if self.running.load(Ordering::SeqCst) {
            return;
        }

        self.running.store(true, Ordering::SeqCst);

        // 启动单个工作线程，串行执行所有任务
        let running = Arc::clone(&self.running);
        let tasks = Arc::clone(&self.tasks);
        let active_tasks = Arc::clone(&self.active_tasks);

        thread::spawn(move || {
            info!("📈 调度器工作线程已启动");

            // 首次执行所有任务
            Self::execute_all_tasks(&tasks, &running, &active_tasks);

            // 循环检查各间隔
            let mut last_realtime = Instant::now();
            let mut last_short = Instant::now();
            let mut last_long = Instant::now();

            while running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(1));

                if !running.load(Ordering::SeqCst) {
                    break;
                }

                // 检查实时间隔（5分钟）
                if last_realtime.elapsed() >= RotationInterval::Realtime.as_duration() {
                    Self::execute_interval_tasks(
                        &tasks,
                        RotationInterval::Realtime,
                        &running,
                        &active_tasks,
                    );
                    last_realtime = Instant::now();
                }

                // 检查短期间隔（15分钟）
                if last_short.elapsed() >= RotationInterval::ShortTerm.as_duration() {
                    Self::execute_interval_tasks(
                        &tasks,
                        RotationInterval::ShortTerm,
                        &running,
                        &active_tasks,
                    );
                    last_short = Instant::now();
                }

                // 检查长期间隔（11.5小时）
                if last_long.elapsed() >= RotationInterval::LongTerm.as_duration() {
                    Self::execute_interval_tasks(
                        &tasks,
                        RotationInterval::LongTerm,
                        &running,
                        &active_tasks,
                    );
                    last_long = Instant::now();
                }
            }

            info!("📈 调度器工作线程已退出");
        });
    }

    /// 执行所有任务（首次启动，顺序执行，智能跳过有效缓存）
    fn execute_all_tasks(
        tasks: &Arc<RwLock<Vec<ScheduledTask>>>,
        running: &Arc<AtomicBool>,
        active_tasks: &Arc<AtomicUsize>,
    ) {
        let tasks_to_run: Vec<_> = {
            let guard = tasks.read();
            guard.iter().filter(|t| t.enabled).cloned().collect()
        };

        let total = tasks_to_run.len();
        info!("🚀 开始检查 {} 个任务的缓存状态", total);

        let mut executed_count = 0;
        let mut skipped_count = 0;

        for (i, task) in tasks_to_run.iter().enumerate() {
            if !running.load(Ordering::SeqCst) {
                warn!("⚠️ 收到停止信号，中断任务执行");
                return;
            }

            if task.ask_type.needs_refresh() {
                Self::execute_single_task(task, i + 1, total, active_tasks);
                executed_count += 1;
            } else {
                skipped_count += 1;
            }
        }

        info!(
            "✅ 初始化任务检查完成: 执行 {} 个，跳过 {} 个",
            executed_count, skipped_count
        );
    }

    /// 执行指定间隔的任务（顺序执行）
    fn execute_interval_tasks(
        tasks: &Arc<RwLock<Vec<ScheduledTask>>>,
        interval: RotationInterval,
        running: &Arc<AtomicBool>,
        active_tasks: &Arc<AtomicUsize>,
    ) {
        let tasks_to_run: Vec<_> = {
            let guard = tasks.read();
            guard
                .iter()
                .filter(|t| t.enabled && t.interval == interval)
                .cloned()
                .collect()
        };

        if tasks_to_run.is_empty() {
            return;
        }

        let total = tasks_to_run.len();
        debug!("执行 {:?} 间隔的 {} 个任务", interval, total);

        for (i, task) in tasks_to_run.iter().enumerate() {
            if !running.load(Ordering::SeqCst) {
                return;
            }
            Self::execute_single_task(task, i + 1, total, active_tasks);
        }
    }

    /// 执行单个任务（带进度和超时）
    fn execute_single_task(
        task: &ScheduledTask,
        current: usize,
        total: usize,
        active_tasks: &Arc<AtomicUsize>,
    ) {
        let name = task.ask_type.display_name();
        info!("[{}/{}] ⏳ 开始执行任务: {}", current, total, name);

        active_tasks.fetch_add(1, Ordering::SeqCst);
        let start = Instant::now();

        const TASK_TIMEOUT_SECS: u64 = 180;

        let task_type = task.ask_type.clone();

        // 根据日期策略获取日期参数
        let date_param = Self::get_date_param_for_task(&task.date_strategy, &task.specified_date);

        // 直接执行任务，不创建新线程（因为已经在 Rayon 并行上下文中）
        info!("[{}/{}] 📞 调用 akshare 接口...", current, total);
        let result = task_type.execute(date_param.as_deref());

        let elapsed = start.elapsed();
        active_tasks.fetch_sub(1, Ordering::SeqCst);

        if elapsed.as_secs() > TASK_TIMEOUT_SECS {
            warn!(
                "[{}/{}] ⚠️ {} 执行超时 ({:.1}s)",
                current,
                total,
                name,
                elapsed.as_secs_f32()
            );
        }

        match result {
            Ok(()) => {
                info!(
                    "[{}/{}] ✅ {} 完成 ({:.1}s)",
                    current,
                    total,
                    name,
                    elapsed.as_secs_f32()
                );
            }
            Err(e) => {
                error!(
                    "[{}/{}] ❌ {} 失败: {} ({:.1}s)",
                    current,
                    total,
                    name,
                    e,
                    elapsed.as_secs_f32()
                );
            }
        }
    }

    /// 获取任务的日期参数
    fn get_date_param_for_task(
        date_strategy: &DateStrategy,
        specified_date: &Option<String>,
    ) -> Option<String> {
        let trading_manager = TradingDaysManager::new(24);

        match date_strategy {
            DateStrategy::Current => Some(Local::now().format("%Y%m%d").to_string()),
            DateStrategy::LastTradingDay => {
                let date = trading_manager.get_last_trading_day(None);
                Some(date.format("%Y%m%d").to_string())
            }
            DateStrategy::Specified => specified_date.clone(),
            DateStrategy::LastWorkday => {
                let date = trading_manager.get_last_workday(None);
                Some(date.format("%Y%m%d").to_string())
            }
        }
    }

    /// 停止调度器（立即返回，不等待任务完成）
    pub fn stop(&self) {
        info!("⏹️ 正在停止调度器...");
        self.running.store(false, Ordering::SeqCst);

        // 给一点时间让工作线程检测到停止信号
        thread::sleep(Duration::from_millis(200));

        let active = self.active_tasks.load(Ordering::SeqCst);
        if active > 0 {
            warn!("⚠️ 还有 {} 个任务正在执行中（后台继续）", active);
        }

        info!("⏹️ 调度器已标记停止");
    }

    /// 检查调度器是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 获取当前活跃任务数
    pub fn active_task_count(&self) -> usize {
        self.active_tasks.load(Ordering::SeqCst)
    }
}

impl Default for StockScheduler {
    fn default() -> Self {
        Self::new()
    }
}

// 全局调度器实例
static SCHEDULER: once_cell::sync::Lazy<StockScheduler> =
    once_cell::sync::Lazy::new(StockScheduler::new);

/// 获取全局调度器实例
pub fn get_scheduler() -> &'static StockScheduler {
    &SCHEDULER
}

/// 启动全局调度器（带默认任务）
pub fn start_scheduler() {
    let scheduler = get_scheduler();
    if scheduler.is_running() {
        debug!("调度器已在运行中");
        info!("⚠️ 调度器已在运行中");
        return;
    }
    info!("📋 添加默认调度任务...");
    scheduler.add_default_tasks();
    info!("🚀 启动调度器工作线程...");
    scheduler.start();
    info!("✅ 调度器启动完成，后台任务开始执行");
}

/// 停止全局调度器
pub fn stop_scheduler() {
    get_scheduler().stop();
}
