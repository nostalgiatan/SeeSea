# Copyright (C) 2025 nostalgiatan
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

"""
股票数据服务

提供统一的股票数据访问接口，整合多个数据源和缓存系统。

主要功能:
- 实时行情获取和推送
- 历史数据查询
- 财务数据分析
- 自动数据更新
- 多数据源融合
"""

import asyncio
from datetime import date, datetime
from typing import Optional, List, Dict, Any, Callable
import logging
import time

from .client import StockDataClient
from .cache import StockCacheManager, CacheScope, CacheTTL
from .scheduler import StockScheduler, UpdateFrequency
from .preload_service import StockPreloadService
from .task_queue import StockTaskQueue, TaskPriority
from .event_handler import StockDataRequestHandler

from seesea.stock.models import (
    Stock,
    StockQuote,
    StockKLine,
    StockFinancial,
    StockShareholder,
    StockAnnouncement,
    StockFundFlow,
    PeriodType,
    AdjustType,
)
from seesea.stock.exchange import IndexQuote, MarketStatus

logger = logging.getLogger(__name__)


# 默认缓存目录（当 Rust 未传递配置时使用）
def _get_default_cache_path() -> str:
    """获取默认缓存路径（仅作为后备方案）"""
    import platform
    from pathlib import Path

    system = platform.system().lower()
    if system == "windows":
        base = Path("D:/seesea") if Path("D:/").exists() else Path("C:/seesea")
    elif system == "darwin":
        base = Path.home() / "Library" / "Application Support" / "seesea"
    else:
        base = Path.home() / ".local" / "share" / "seesea"

    cache_dir = base / "cache"
    cache_dir.mkdir(parents=True, exist_ok=True)
    return str(cache_dir / "stock_cache.db")


class StockService:
    """
    股票数据服务

    整合多个数据源，提供统一的数据访问接口。
    支持缓存、自动更新和数据融合。

    使用示例:
        >>> service = StockService()
        >>> await service.initialize()
        >>>
        >>> # 获取实时行情
        >>> quote = await service.get_quote("000001")
        >>> print(f"{quote.name}: {quote.price}")
        >>>
        >>> # 获取K线数据
        >>> klines = await service.get_klines("000001", period=PeriodType.DAILY)
        >>>
        >>> # 订阅实时更新
        >>> await service.subscribe_realtime(["000001", "600519"])
        >>> await service.start_realtime()
    """

    def __init__(
        self,
        enable_cache: bool = True,
        enable_scheduler: bool = True,
    ):
        """
        初始化股票服务

        Args:
            enable_cache: 是否启用缓存
            enable_scheduler: 是否启用调度器
        """
        self._enable_cache = enable_cache
        self._enable_scheduler = enable_scheduler

        # 数据客户端
        self._client: Optional[StockDataClient] = None

        # 缓存管理器（自动使用全局缓存实例）
        self._cache: Optional[StockCacheManager] = None
        if enable_cache:
            self._cache = StockCacheManager()

        # 预加载服务
        self._preload_service: Optional[StockPreloadService] = None

        # 任务队列系统
        self._task_queue: Optional[StockTaskQueue] = None

        # 调度器
        self._scheduler: Optional[StockScheduler] = None
        if enable_scheduler:
            self._scheduler = StockScheduler()

        # 事件处理器
        self._event_handler: Optional[StockDataRequestHandler] = None

        # 缓存优先模式 - 不再使用回调函数
        # 所有数据更新都通过缓存机制实现

        # 初始化状态
        self._initialized = False
        self._event_loop = None  # 存储事件循环用于后台任务

    def initialize_sync(self) -> bool:
        """同步初始化服务（用于避免在tokio任务中使用asyncio）"""
        if self._initialized:
            return True

        try:
            # 初始化数据客户端
            try:
                self._client = StockDataClient()
                logger.info("股票数据客户端初始化成功")
            except Exception as e:
                logger.warning(f"股票数据客户端初始化失败: {e}")

            # 初始化缓存（同步方式）
            if self._cache:
                # 使用同步方式初始化缓存，避免在tokio任务中使用asyncio
                success = self._cache.initialize_sync()
                if success:
                    logger.info("缓存同步初始化成功")
                else:
                    logger.warning("缓存同步初始化失败，服务将继续运行")

            # 初始化任务队列系统
            if self._cache and self._client:
                try:
                    logger.info("初始化任务队列系统...")
                    self._task_queue = StockTaskQueue(max_concurrent=10)
                    self._task_queue.set_cache(self._cache)
                    
                    # 注册任务处理器
                    self._register_task_handlers()
                    
                    logger.info("任务队列系统初始化成功")
                except Exception as e:
                    logger.error(f"任务队列系统初始化失败: {e}")

            # 初始化事件处理器
            if self._task_queue and self._cache and self._client:
                try:
                    logger.info("初始化事件处理器...")
                    self._event_handler = StockDataRequestHandler(
                        task_queue=self._task_queue,
                        cache_manager=self._cache,
                        client=self._client
                    )
                    logger.info("事件处理器初始化成功")
                except Exception as e:
                    logger.error(f"事件处理器初始化失败: {e}")

            # 初始化预加载服务（仅用于加载股票列表）
            if self._cache and self._client:
                try:
                    logger.info("初始化预加载服务...")
                    self._preload_service = StockPreloadService(self._cache, self._client)
                    
                    # 阻塞加载股票列表（这是唯一阻塞的操作）
                    logger.info("开始同步加载股票列表数据（阻塞）...")
                    
                    # 使用同步预加载确保股票列表在服务器启动前缓存
                    import asyncio
                    import threading
                    
                    # 检测是否已有事件循环在运行
                    try:
                        loop = asyncio.get_running_loop()
                        logger.info("检测到已有事件循环在运行，使用现有循环")
                        own_loop = False
                    except RuntimeError:
                        # 没有运行中的循环，创建新的
                        loop = asyncio.new_event_loop()
                        asyncio.set_event_loop(loop)
                        own_loop = True
                        logger.info("创建新的事件循环")
                    
                    self._event_loop = loop
                    
                    try:
                        # 只加载股票列表，阻塞直到完成
                        # 如果已有事件循环在运行，使用线程执行预加载
                        if own_loop:
                            # 自己创建的循环，直接使用
                            success = self._preload_service.preload_market_data_priority_sync(loop)
                        else:
                            # 使用现有循环，在线程中执行预加载
                            logger.info("在线程中执行股票列表预加载...")
                            preload_result = {'success': False, 'error': None}
                            
                            def preload_in_thread():
                                try:
                                    # 创建新的事件循环用于预加载
                                    preload_loop = asyncio.new_event_loop()
                                    asyncio.set_event_loop(preload_loop)
                                    try:
                                        preload_result['success'] = self._preload_service.preload_market_data_priority_sync(preload_loop)
                                    finally:
                                        preload_loop.close()
                                except Exception as e:
                                    preload_result['error'] = str(e)
                                    logger.error(f"线程中预加载失败: {e}")
                            
                            # 创建并启动预加载线程
                            preload_thread = threading.Thread(target=preload_in_thread)
                            preload_thread.start()
                            preload_thread.join()  # 等待预加载完成
                            
                            success = preload_result['success']
                            if preload_result['error']:
                                logger.error(f"预加载线程出错: {preload_result['error']}")
                        
                        if success:
                            logger.info("股票列表加载成功，搜索功能立即可用")
                            # 验证股票列表确实在缓存中
                            try:
                                # 根据是否拥有事件循环，使用不同的验证方式
                                if own_loop:
                                    # 自己创建的循环，直接使用
                                    stock_list_data = loop.run_until_complete(
                                        self._cache.get(CacheScope.STOCK_LIST, "list:cn_a")
                                    )
                                else:
                                    # 使用现有循环，在线程中执行验证
                                    verify_result = {'data': None, 'error': None}
                                    
                                    def verify_in_thread():
                                        try:
                                            verify_loop = asyncio.new_event_loop()
                                            asyncio.set_event_loop(verify_loop)
                                            try:
                                                verify_result['data'] = verify_loop.run_until_complete(
                                                    self._cache.get(CacheScope.STOCK_LIST, "list:cn_a")
                                                )
                                            finally:
                                                verify_loop.close()
                                        except Exception as e:
                                            verify_result['error'] = str(e)
                                    
                                    verify_thread = threading.Thread(target=verify_in_thread)
                                    verify_thread.start()
                                    verify_thread.join()
                                    
                                    if verify_result['error']:
                                        raise Exception(verify_result['error'])
                                    
                                    stock_list_data = verify_result['data']
                                
                                if stock_list_data:
                                    logger.info(f"✅ 验证成功：股票列表已在缓存中，包含 {len(stock_list_data)} 条数据")
                                    
                                    # 只在自己创建循环时启动后台线程
                                    if own_loop:
                                        # 在后台线程中运行事件循环以执行后台任务
                                        def run_event_loop():
                                            try:
                                                # 启动后台任务队列
                                                loop.run_until_complete(self._start_background_tasks(stock_list_data))
                                                # 保持循环运行以处理后续任务
                                                loop.run_forever()
                                            except Exception as e:
                                                logger.error(f"后台任务循环错误: {e}")
                                            finally:
                                                loop.close()
                                        
                                        # 启动后台线程
                                        background_thread = threading.Thread(target=run_event_loop, daemon=True)
                                        background_thread.start()
                                        logger.info("后台任务线程已启动")
                                    else:
                                        logger.info("使用现有事件循环，在线程中启动后台任务...")
                                        # 使用现有事件循环，在新线程中启动后台任务
                                        def run_background_in_thread():
                                            try:
                                                # 创建新的事件循环用于后台任务
                                                bg_loop = asyncio.new_event_loop()
                                                asyncio.set_event_loop(bg_loop)
                                                try:
                                                    bg_loop.run_until_complete(self._start_background_tasks(stock_list_data))
                                                    bg_loop.run_forever()
                                                except Exception as e:
                                                    logger.error(f"后台任务循环错误: {e}")
                                                finally:
                                                    bg_loop.close()
                                            except Exception as e:
                                                logger.error(f"后台线程启动失败: {e}")
                                        
                                        background_thread = threading.Thread(target=run_background_in_thread, daemon=True)
                                        background_thread.start()
                                        logger.info("后台任务线程已启动（使用新的事件循环）")
                                else:
                                    logger.warning("⚠️ 验证失败：股票列表未在缓存中找到")
                            except Exception as verify_error:
                                logger.error(f"验证缓存失败: {verify_error}")
                        else:
                            logger.warning("股票列表加载失败，搜索功能可能不可用")
                        
                        logger.info("初始化完成，后台任务已启动...")
                        
                    except Exception as e:
                        logger.error(f"事件循环初始化失败: {e}")
                        
                except Exception as e:
                    logger.error(f"预加载服务初始化失败: {e}")
                    # 预加载失败不影响服务启动
        
        except Exception as e:
            logger.error(f"股票服务初始化失败: {e}")
            return False
        
        # 标记服务初始化完成
        self._initialized = True
        logger.info("股票服务同步初始化完成")
        
        return True
    
    def _register_task_handlers(self):
        """注册任务队列的处理器"""
        if not self._task_queue:
            return

        # 股票基本信息处理器
        async def handle_stock_info(code: str) -> bool:
            """处理股票基本信息获取任务"""
            try:
                if not self._client or not self._cache:
                    return False
                    
                info = await self._client.get_stock_info(code)
                if info:
                    await self._cache.set(CacheScope.STOCK_INFO, code, info.to_dict(), CacheTTL.STOCK_INFO)
                    logger.debug(f"股票信息缓存成功: {code}")
                    await self._send_done_event(code, "info")
                    return True
                return False
            except Exception as e:
                logger.warning(f"获取股票信息失败 {code}: {e}")
                return False

        # 财务数据处理器
        async def handle_financial_data(code: str) -> bool:
            """处理财务数据获取任务"""
            try:
                if not self._client or not self._cache:
                    return False
                    
                financial = await self._client.get_financial_data(code)
                if financial:
                    await self._cache.set(CacheScope.STOCK_FINANCIAL, code, financial.to_dict(), CacheTTL.FINANCIAL)
                    logger.debug(f"财务数据缓存成功: {code}")
                    await self._send_done_event(code, "financial")
                    return True
                return False
            except Exception as e:
                logger.warning(f"获取财务数据失败 {code}: {e}")
                return False

        # 行业数据处理器
        async def handle_industry_data(code: str) -> bool:
            """处理行业数据获取任务"""
            try:
                if not self._client or not self._cache:
                    return False
                    
                industry = await self._client.get_industry_data(code)
                if industry:
                    await self._cache.set(CacheScope.STOCK_INDUSTRY, code, industry.to_dict(), CacheTTL.INDUSTRY)
                    logger.debug(f"行业数据缓存成功: {code}")
                    await self._send_done_event(code, "industry")
                    return True
                return False
            except Exception as e:
                logger.warning(f"获取行业数据失败 {code}: {e}")
                return False

        # 行情数据处理器
        async def handle_quote_data(code: str) -> bool:
            """处理行情数据获取任务"""
            try:
                if not self._client or not self._cache:
                    return False
                    
                quote = await self._client.get_realtime_quote(code)
                if quote:
                    await self._cache.set(CacheScope.STOCK_QUOTE, code, quote.to_dict(), CacheTTL.QUOTE)
                    logger.debug(f"行情数据缓存成功: {code}")
                    await self._send_done_event(code, "quote")
                    return True
                return False
            except Exception as e:
                logger.warning(f"获取行情数据失败 {code}: {e}")
                return False

        # K线数据处理器
        async def handle_kline_data(code: str) -> bool:
            """处理K线数据获取任务"""
            try:
                if not self._client or not self._cache:
                    return False
                    
                kline = await self._client.get_kline_data(code, period="daily", limit=100)
                if kline:
                    await self._cache.set(CacheScope.STOCK_KLINE, f"{code}:daily", [k.to_dict() for k in kline], CacheTTL.KLINE)
                    logger.debug(f"K线数据缓存成功: {code}")
                    await self._send_done_event(code, "kline")
                    return True
                return False
            except Exception as e:
                logger.warning(f"获取K线数据失败 {code}: {e}")
                return False

        # 股东数据处理器
        async def handle_holders_data(code: str) -> bool:
            """处理股东数据获取任务"""
            try:
                if not self._client or not self._cache:
                    return False
                    
                holder = await self._client.get_shareholder_data(code)
                if holder:
                    await self._cache.set(CacheScope.STOCK_HOLDERS, f"{code}:top10", holder.to_dict(), CacheTTL.HOLDERS)
                    logger.debug(f"股东数据缓存成功: {code}")
                    await self._send_done_event(code, "holders")
                    return True
                return False
            except Exception as e:
                logger.warning(f"获取股东数据失败 {code}: {e}")
                return False

        # 公告数据处理器
        async def handle_announcements_data(code: str) -> bool:
            """处理公告数据获取任务"""
            try:
                if not self._client or not self._cache:
                    return False
                    
                announcements = await self._client.get_announcements(code, date_str="")
                if announcements:
                    await self._cache.set(CacheScope.STOCK_ANNOUNCEMENTS, f"{code}:1:10", [a.to_dict() for a in announcements], CacheTTL.ANNOUNCEMENTS)
                    logger.debug(f"公告数据缓存成功: {code}")
                    await self._send_done_event(code, "announcements")
                    return True
                return False
            except Exception as e:
                logger.warning(f"获取公告数据失败 {code}: {e}")
                return False

        # 注册处理器
        self._task_queue.register_handler("stock_info", handle_stock_info)
        self._task_queue.register_handler("financial_data", handle_financial_data)
        self._task_queue.register_handler("industry_data", handle_industry_data)
        self._task_queue.register_handler("quote_data", handle_quote_data)
        self._task_queue.register_handler("kline_data", handle_kline_data)
        self._task_queue.register_handler("holders_data", handle_holders_data)
        self._task_queue.register_handler("announcements_data", handle_announcements_data)
        
        logger.info("任务处理器注册完成")

    async def _send_done_event(self, code: str, data_type: str):
        """发送数据完成事件"""
        try:
            from seesea_core import send_string_notification_event
            import json
            
            event_data = {
                "code": code,
                "data_type": data_type,
                "status": "done",
                "timestamp": datetime.now().isoformat()
            }
            
            await send_string_notification_event(
                "stock.data.done",
                json.dumps(event_data)
            )
            
            logger.debug(f"已发送股票 {code} 的 {data_type} 数据完成事件")
            
        except ImportError:
            logger.warning("未找到事件系统，无法发送done事件")
        except Exception as e:
            logger.error(f"发送done事件时发生错误: {e}")

    async def _start_background_tasks(self, stock_list_data):
        """启动后台任务，并发获取股票详细信息"""
        try:
            if not self._task_queue:
                logger.warning("任务队列未初始化，无法启动后台任务")
                return

            logger.info("启动后台任务队列...")
            
            # 启动任务队列
            await self._task_queue.start()
            
            # 启动事件处理器
            if self._event_handler:
                try:
                    await self._event_handler.start()
                    logger.info("事件处理器已启动")
                except Exception as e:
                    logger.error(f"事件处理器启动失败: {e}")
            
            # 提取股票代码列表
            stock_codes = [stock.get("code") for stock in stock_list_data if stock.get("code")]
            logger.info(f"准备为 {len(stock_codes)} 只股票添加后台任务...")
            
            # 批量添加股票信息任务（高优先级）
            await self._task_queue.add_bulk_tasks(
                task_type="stock_info",
                codes=stock_codes,
                priority=TaskPriority.HIGH,
                check_expiration=True,
                ttl=CacheTTL.STOCK_INFO,
                scope=CacheScope.STOCK_INFO
            )
            
            # 批量添加财务数据任务（中优先级）
            await self._task_queue.add_bulk_tasks(
                task_type="financial_data",
                codes=stock_codes,
                priority=TaskPriority.MEDIUM,
                check_expiration=True,
                ttl=CacheTTL.FINANCIAL,
                scope=CacheScope.STOCK_FINANCIAL
            )
            
            # 批量添加行业数据任务（中优先级）
            await self._task_queue.add_bulk_tasks(
                task_type="industry_data",
                codes=stock_codes,
                priority=TaskPriority.MEDIUM,
                check_expiration=True,
                ttl=CacheTTL.INDUSTRY,
                scope=CacheScope.STOCK_INDUSTRY
            )
            
            logger.info("后台任务添加完成，开始并发执行...")
            
            # 定期打印进度
            async def print_progress():
                while self._task_queue._is_running:
                    await asyncio.sleep(30)  # 每30秒打印一次
                    progress = self._task_queue.get_progress()
                    stats = self._task_queue.get_stats()
                    logger.info(
                        f"后台任务进度: {progress['completed']}/{progress['total']} "
                        f"({progress['progress']:.1%}), "
                        f"运行中: {stats['running_tasks']}, "
                        f"待处理: {stats['queue_size']}"
                    )
            
            asyncio.create_task(print_progress())
            
        except Exception as e:
            logger.error(f"启动后台任务失败: {e}")

    async def _run_async_preload(self) -> None:
        """运行异步预加载（后台任务）"""
        try:
            logger.info("开始后台异步预加载股票数据...")
            
            # 使用异步版本的预加载方法
            preload_status = await self._preload_service.preload_all()
            
            logger.info(f"后台预加载完成，状态: {preload_status}")
            
            # 检查关键数据是否加载成功
            success_count = sum(1 for v in preload_status.values() if v)
            total_count = len(preload_status)
            if success_count >= total_count * 0.5:  # 50%成功率认为成功
                logger.info(f"后台预加载成功：{success_count}/{total_count}")
            else:
                logger.warning(f"后台预加载成功率较低：{success_count}/{total_count}")
                
        except Exception as e:
            logger.error(f"后台异步预加载失败: {e}")
        
        # 异步预加载完成后，注册调度任务
        try:
            if self._scheduler:
                self._register_scheduled_tasks()
                logger.info("调度任务已注册")
        except Exception as e:
            logger.error(f"注册调度任务失败: {e}")
        
        # 标记初始化完成
        self._initialized = True
        logger.info("股票服务异步初始化完成")

    def _register_scheduled_tasks(self):
        """注册调度任务"""
        if not self._scheduler:
            return

        # 实时行情更新 (5秒)
        self._scheduler.register_task(
            name="realtime_quotes",
            callback=self._update_realtime_quotes,
            frequency=UpdateFrequency.REALTIME,
        )

        # 指数行情更新 (30秒)
        self._scheduler.register_task(
            name="index_quotes",
            callback=self._update_index_quotes,
            frequency=UpdateFrequency.EVERY_MINUTE,
        )

        # 资金流向更新 (5分钟)
        self._scheduler.register_task(
            name="fund_flow",
            callback=self._update_fund_flow,
            frequency=UpdateFrequency.SHORT,
        )

        # 公告更新 (30分钟)
        self._scheduler.register_task(
            name="announcements",
            callback=self._update_announcements,
            frequency=UpdateFrequency.MEDIUM,
        )

        # 股票基础信息更新 (每日)
        self._scheduler.register_task(
            name="stock_info",
            callback=self._update_stock_info,
            frequency=UpdateFrequency.DAILY,
        )

    async def _update_realtime_quotes(self) -> List[StockQuote]:
        """更新实时行情"""
        codes = self._scheduler.get_subscribed_codes() if self._scheduler else []

        if not codes:
            return []

        quotes = await self.get_realtime_quotes(codes, use_cache=False)

        # 缓存更新 - 缓存优先模式的核心
        if self._cache and quotes:
            for quote in quotes:
                await self._cache.set_realtime_quote(quote.code, quote.to_dict())

        return quotes

    def _get_index_name(self, symbol: str) -> str:
        """获取指数名称"""
        index_names = {
            "000001": "上证指数",
            "000300": "沪深300",
            "399001": "深证成指",
            "399006": "创业板指"
        }
        return index_names.get(symbol, f"指数{symbol}")

    async def _update_index_quotes(self) -> List[IndexQuote]:
        """更新指数行情"""
        if not self._client:
            return []

        # 获取主要指数数据
        index_codes = ["000001", "000300", "399001", "399006"]  # 上证指数, 沪深300, 深证成指, 创业板指
        indices = []
        
        for symbol in index_codes:
            try:
                klines = await self._client.get_index_data(symbol, period=PeriodType.DAILY)
                if klines:
                    latest = klines[-1]  # 获取最新数据
                    index_quote = IndexQuote(
                        code=symbol,
                        name=self._get_index_name(symbol),
                        price=latest.close,
                        change=latest.change,
                        change_pct=latest.change_pct,
                        volume=latest.volume,
                        amount=latest.amount,
                        open=latest.open,
                        high=latest.high,
                        low=latest.low,
                        prev_close=latest.close - latest.change,
                        time=datetime.now()
                    )
                    indices.append(index_quote)
            except Exception as e:
                logger.warning(f"获取指数 {symbol} 数据失败: {e}")

        # 缓存
        if self._cache and indices:
            for idx in indices:
                await self._cache.set(
                    CacheScope.STOCK_INDEX,
                    idx.code,
                    idx.to_dict(),
                )

        return indices

    async def _update_fund_flow(self) -> List[StockFundFlow]:
        """更新资金流向"""
        codes = self._scheduler.get_subscribed_codes() if self._scheduler else []

        if not codes or not self._client:
            return []

        all_flows = []
        for code in codes[:10]:  # 限制数量避免过多请求
            try:
                fund_flow = await self._client.get_fund_flow(code)
                if fund_flow:
                    all_flows.append(fund_flow)

                    # 缓存
                    if self._cache:
                        await self._cache.set(
                            CacheScope.STOCK_FUND_FLOW,
                            code,
                            fund_flow.to_dict(),
                        )
            except Exception as e:
                logger.warning(f"获取股票 {code} 资金流向失败: {e}")

        return all_flows

    async def _update_announcements(self) -> List[StockAnnouncement]:
        """更新公告"""
        if not self._client:
            return []

        try:
            # 获取今日公告
            today = date.today().strftime("%Y%m%d")
            announcements = await self._client.get_announcements(code=None, date_str=today)

            # 缓存
            if self._cache and announcements:
                await self._cache.set(
                    CacheScope.STOCK_ANNOUNCEMENTS,
                    "latest",
                    [a.to_dict() for a in announcements],
                )

            # 缓存优先模式 - 数据已写入缓存，无需回调

            return announcements
        except Exception as e:
            logger.warning(f"获取公告失败: {e}")
            return []

    async def _update_stock_info(self) -> List[Stock]:
        """更新股票基础信息"""
        if not self._client:
            return []

        try:
            # 获取股票列表
            stock_basics = await self._client.get_stock_list()
            
            # 转换为Stock模型
            stocks = []
            for basic in stock_basics:
                stock = Stock(
                    code=basic.code,
                    name=basic.name,
                    market="cn_a",  # 默认A股市场
                    industry=basic.industry,
                    area=basic.area,
                    listing_date=basic.listing_date
                )
                stocks.append(stock)

            # 缓存
            if self._cache and stocks:
                for stock in stocks:
                    await self._cache.set_stock_info(stock.code, stock.to_dict())

            return stocks
        except Exception as e:
            logger.warning(f"更新股票基础信息失败: {e}")
            return []

    # ==================== 公开接口 ====================

    async def get_stock_list(self, market: str = "cn_a") -> List[Stock]:
        """
        获取股票列表

        Args:
            market: 市场类型

        Returns:
            股票列表
        """
        logger.info(f"开始获取股票列表，市场: {market}")
        
        # 检查缓存
        if self._cache:
            logger.info("检查缓存中的股票列表")
            cached = await self._cache.get(CacheScope.STOCK_LIST, market)
            if cached:
                logger.info(f"从缓存获取到 {len(cached)} 条股票数据")
                return [Stock.from_dict(s) for s in cached]
            else:
                logger.info("缓存中没有股票列表数据")
        else:
            logger.info("没有启用缓存")

        # 从客户端获取
        if self._client:
            logger.info("从客户端获取股票列表")
            try:
                stock_basics = await self._client.get_stock_list()
                logger.info(f"客户端返回 {len(stock_basics)} 条股票基础数据")
                
                # 转换为Stock模型
                stocks = []
                for basic in stock_basics:
                    logger.debug(f"处理股票: {basic.code} - {basic.name}")
                    stock = Stock(
                        code=basic.code,
                        name=basic.name,
                        market=market,
                        industry=basic.industry,
                        area=basic.area,
                        listing_date=basic.listing_date
                    )
                    stocks.append(stock)
                
                logger.info(f"成功转换 {len(stocks)} 条股票数据")

                # 缓存
                if self._cache and stocks:
                    logger.info("将股票列表写入缓存")
                    await self._cache.set(
                        CacheScope.STOCK_LIST,
                        market,
                        [s.to_dict() for s in stocks],
                    )

                return stocks
            except Exception as e:
                logger.error(f"获取股票列表失败: {e}", exc_info=True)
        else:
            logger.warning("客户端未初始化")

        logger.warning("返回空股票列表")
        return []

    def get_code_by_name(self, name: str) -> Optional[str]:
        """
        根据股票名称获取代码

        Args:
            name: 股票名称，如 "平安银行"

        Returns:
            股票代码，如 "000001"，未找到返回 None

        注意:
            需要先调用 get_stock_list() 初始化映射
        """
        # 这里可以实现基于缓存的查找逻辑
        # 暂时返回None，后续可以优化
        return None

    def get_name_by_code(self, code: str) -> Optional[str]:
        """
        根据股票代码获取名称

        Args:
            code: 股票代码，如 "000001"

        Returns:
            股票名称，如 "平安银行"，未找到返回 None
        """
        # 这里可以实现基于缓存的查找逻辑
        # 暂时返回None，后续可以优化
        return None

    async def get_stock_info(self, code: str) -> Optional[Stock]:
        """
        获取股票基础信息

        Args:
            code: 股票代码

        Returns:
            股票信息
        """
        # 检查缓存
        if self._cache:
            cached = await self._cache.get_stock_info(code)
            if cached:
                return Stock.from_dict(cached)

        # 从客户端获取
        if self._client:
            try:
                stock_info = await self._client.get_stock_info(code)
                if stock_info:
                    # 转换为Stock模型
                    stock = Stock(
                        code=stock_info.code,
                        name=stock_info.name,
                        market="cn_a",
                        industry=stock_info.industry,
                        area="",  # StockInfo没有area字段，使用默认值
                        listing_date=None  # StockInfo没有上市日期字段
                    )

                    # 缓存
                    if self._cache:
                        await self._cache.set_stock_info(code, stock.to_dict())

                    return stock
            except Exception as e:
                logger.warning(f"获取股票基础信息失败 {code}: {e}")

        return None

    async def get_quote(
        self, code: str, use_cache: bool = True
    ) -> Optional[StockQuote]:
        """
        获取实时行情

        Args:
            code: 股票代码
            use_cache: 是否使用缓存

        Returns:
            实时行情
        """
        # 检查缓存
        if use_cache and self._cache:
            cached = await self._cache.get_realtime_quote(code)
            if cached:
                return StockQuote.from_dict(cached)

        # 从客户端获取
        if self._client:
            try:
                quote = await self._client.get_realtime_quote(code)

                # 缓存
                if self._cache and quote:
                    await self._cache.set_realtime_quote(code, quote.to_dict())

                return quote
            except Exception as e:
                logger.warning(f"获取实时行情失败 {code}: {e}")

        return None

    async def get_realtime_quotes(
        self,
        codes: List[str] | None = None,
        use_cache: bool = True,
    ) -> List[StockQuote]:
        """
        批量获取实时行情

        Args:
            codes: 股票代码列表
            use_cache: 是否使用缓存

        Returns:
            实时行情列表
        """
        if self._client:
            try:
                if codes:
                    return await self._client.get_realtime_quotes_batch(codes)
                else:
                    # 如果没有指定codes，获取所有股票的行情
                    stock_list = await self.get_stock_list()
                    if stock_list:
                        codes = [stock.code for stock in stock_list[:100]]  # 限制数量
                        return await self._client.get_realtime_quotes_batch(codes)
            except Exception as e:
                logger.warning(f"批量获取实时行情失败: {e}")
        return []

    async def get_klines(
        self,
        code: str,
        period: PeriodType = PeriodType.DAILY,
        adjust: AdjustType = AdjustType.QFQ,
        start_date: Optional[date] = None,
        end_date: Optional[date] = None,
        limit: int = 1000,
        use_cache: bool = True,
    ) -> List[StockKLine]:
        """
        获取K线数据

        Args:
            code: 股票代码
            period: K线周期
            adjust: 复权类型
            start_date: 开始日期
            end_date: 结束日期
            limit: 数据条数
            use_cache: 是否使用缓存

        Returns:
            K线数据列表
        """
        # 检查缓存
        if use_cache and self._cache:
            cached = await self._cache.get_klines(code, period.value)
            if cached:
                return [StockKLine(**k) for k in cached]

        # 从客户端获取
        if self._client:
            try:
                # 转换日期格式
                start_str = start_date.strftime("%Y%m%d") if start_date else None
                end_str = end_date.strftime("%Y%m%d") if end_date else None
                
                # 转换复权类型
                adjust_str = ""
                if adjust == AdjustType.QFQ:
                    adjust_str = "qfq"
                elif adjust == AdjustType.HFQ:
                    adjust_str = "hfq"
                
                klines = await self._client.get_kline_data(
                    code=code,
                    period=period,
                    start_date=start_str,
                    end_date=end_str,
                    adjust=adjust_str,
                    limit=limit
                )

                # 缓存
                if self._cache and klines:
                    await self._cache.set_klines(
                        code,
                        period.value,
                        [k.to_dict() for k in klines],
                    )

                return klines
            except Exception as e:
                logger.warning(f"获取K线数据失败 {code}: {e}")

        return []

    async def get_financial(self, code: str) -> List[StockFinancial]:
        """
        获取财务数据

        Args:
            code: 股票代码

        Returns:
            财务数据列表
        """
        # 检查缓存
        if self._cache:
            cached = await self._cache.get(CacheScope.STOCK_FINANCIAL, code)
            if cached:
                return [StockFinancial(**f) for f in cached]

        # 从客户端获取
        financials = []
        if self._client:
            try:
                # 获取资产负债表
                balance_sheet = await self._client.get_balance_sheet(code)
                # 获取利润表
                income_statement = await self._client.get_income_statement(code)
                # 获取现金流量表
                cash_flow = await self._client.get_cash_flow(code)
                
                # 这里可以根据需要构建StockFinancial对象
                # 暂时返回空列表，后续可以完善
                
            except Exception as e:
                logger.warning(f"获取财务数据失败 {code}: {e}")

        # 缓存
        if self._cache and financials:
            await self._cache.set(
                CacheScope.STOCK_FINANCIAL,
                code,
                [f.to_dict() for f in financials],
            )

        return financials

    async def get_holders(
        self, code: str, holder_type: str = "top10"
    ) -> List[StockShareholder]:
        """
        获取股东信息

        Args:
            code: 股票代码
            holder_type: 股东类型

        Returns:
            股东信息列表
        """
        # 检查缓存
        cache_key = f"{code}:{holder_type}"
        if self._cache:
            cached = await self._cache.get(CacheScope.STOCK_HOLDER, cache_key)
            if cached:
                return [StockShareholder(**h) for h in cached]

        # 从客户端获取
        if self._client:
            try:
                # 获取股东数据
                shareholder_data = await self._client.get_shareholder_data(code)
                
                # 这里可以根据需要构建StockShareholder对象
                # 暂时返回空列表，后续可以完善
                holders = []
                
                # 缓存
                if self._cache and holders:
                    await self._cache.set(
                        CacheScope.STOCK_HOLDER,
                        cache_key,
                        [h.to_dict() for h in holders],
                    )

                return holders
            except Exception as e:
                logger.warning(f"获取股东信息失败 {code}: {e}")

        return []

        return []

    async def get_announcements(
        self,
        code: str | None = None,
        limit: int = 100,
    ) -> List[StockAnnouncement]:
        """
        获取公告信息

        Args:
            code: 股票代码 (可选)
            limit: 数据条数

        Returns:
            公告列表
        """
        cache_key = code or "latest"

        # 检查缓存
        if self._cache:
            cached = await self._cache.get_announcements(cache_key)
            if cached:
                return [StockAnnouncement(**a) for a in cached]

        # 从客户端获取
        if self._client:
            try:
                # 获取今日公告
                today = date.today().strftime("%Y%m%d")
                announcements = await self._client.get_announcements(
                    code=code,
                    date_str=today
                )

                # 缓存
                if self._cache and announcements:
                    await self._cache.set_announcements(
                        cache_key,
                        [a.to_dict() for a in announcements],
                    )

                return announcements
            except Exception as e:
                logger.warning(f"获取公告失败: {e}")

        return []

    async def get_fund_flow(self, code: str) -> List[StockFundFlow]:
        """
        获取资金流向

        Args:
            code: 股票代码

        Returns:
            资金流向数据
        """
        # 检查缓存
        if self._cache:
            cached = await self._cache.get(CacheScope.STOCK_FUND_FLOW, code)
            if cached:
                return [StockFundFlow(**f) for f in cached]

        # 从客户端获取
        if self._client:
            try:
                fund_flow = await self._client.get_fund_flow(code)
                if fund_flow:
                    # 缓存
                    if self._cache:
                        await self._cache.set(
                            CacheScope.STOCK_FUND_FLOW,
                            code,
                            fund_flow.to_dict(),
                        )
                    return [fund_flow]
            except Exception as e:
                logger.warning(f"获取资金流向失败 {code}: {e}")

        return []

    async def get_index_quotes(self) -> List[IndexQuote]:
        """
        获取指数行情

        Returns:
            指数行情列表
        """
        if self._client:
            try:
                # 获取主要指数数据
                index_codes = ["000001", "000300", "399001", "399006"]  # 上证指数, 沪深300, 深证成指, 创业板指
                indices = []
                
                for symbol in index_codes:
                    try:
                        klines = await self._client.get_index_data(symbol, period=PeriodType.DAILY)
                        if klines:
                            latest = klines[-1]  # 获取最新数据
                            index_quote = IndexQuote(
                                code=symbol,
                                name=self._get_index_name(symbol),
                                price=latest.close,
                                change=latest.change,
                                change_pct=latest.change_pct,
                                volume=latest.volume,
                                amount=latest.amount,
                                open=latest.open,
                                high=latest.high,
                                low=latest.low,
                                prev_close=latest.close - latest.change,
                                time=datetime.now()
                            )
                            indices.append(index_quote)
                    except Exception as e:
                        logger.warning(f"获取指数 {symbol} 数据失败: {e}")
                
                return indices
            except Exception as e:
                logger.warning(f"获取指数行情失败: {e}")
        return []

    async def get_index_quotes_from_cache(self) -> List[IndexQuote]:
        """
        从缓存获取指数行情（缓存优先）
        
        Returns:
            指数行情列表，如果缓存未就绪则返回空列表
        """
        try:
            # 首先检查预加载是否完成
            if not self.is_preload_complete():
                logger.warning("预加载尚未完成，无法获取指数行情")
                return []
                
            if not self.is_cache_ready():
                logger.warning("缓存未就绪，无法获取指数行情")
                return []
            
            # 从缓存获取指数数据
            import asyncio
            try:
                # 尝试获取当前事件循环
                loop = asyncio.get_running_loop()
                # 如果有运行中的循环，使用run_coroutine_threadsafe
                future = asyncio.run_coroutine_threadsafe(
                    self._cache.get(CacheScope.STOCK_INDEX, "main"), 
                    loop
                )
                cached_data = future.result(timeout=30)  # 30秒超时
            except RuntimeError:
                # 没有运行中的事件循环，可以安全使用run_until_complete
                loop = asyncio.new_event_loop()
                asyncio.set_event_loop(loop)
                try:
                    cached_data = loop.run_until_complete(self._cache.get(CacheScope.STOCK_INDEX, "main"))
                finally:
                    loop.close()
            
            if not cached_data:
                logger.warning(f"缓存中未找到指数行情数据")
                return []
            
            # 转换为IndexQuote对象列表
            indices = []
            for index_data in cached_data:
                index_quote = IndexQuote(
                    code=index_data.get('code', ''),
                    name=index_data.get('name', ''),
                    price=index_data.get('price', 0.0),
                    change=index_data.get('change', 0.0),
                    change_pct=index_data.get('change_pct', 0.0),
                    volume=index_data.get('volume', 0),
                    amount=index_data.get('amount', 0.0),
                    open=index_data.get('open', 0.0),
                    high=index_data.get('high', 0.0),
                    low=index_data.get('low', 0.0),
                    prev_close=index_data.get('prev_close', 0.0),
                    time=datetime.fromisoformat(index_data.get('time', datetime.now().isoformat()))
                )
                indices.append(index_quote)
            
            logger.info(f"从缓存获取指数行情：{len(indices)} 个指数")
            return indices
            
        except Exception as e:
            logger.error(f"从缓存获取指数行情失败: {e}")
            return []

    async def get_market_status(self, exchange: str = "sse") -> Optional[MarketStatus]:
        """
        获取市场状态

        Args:
            exchange: 交易所代码

        Returns:
            市场状态
        """
        from datetime import datetime, time
        
        # 获取当前时间
        now = datetime.now()
        current_time = now.time()
        
        # 判断是否为交易日 (周一到周五)
        is_trading_day = now.weekday() < 5
        
        # 定义交易时间段 (A股市场: 9:30-11:30, 13:00-15:00)
        morning_start = time(9, 30)
        morning_end = time(11, 30)
        afternoon_start = time(13, 0)
        afternoon_end = time(15, 0)
        
        # 判断市场状态
        if not is_trading_day:
            status = "closed"
            status_name = "休市"
        elif morning_start <= current_time <= morning_end:
            status = "open"
            status_name = "交易中"
        elif afternoon_start <= current_time <= afternoon_end:
            status = "open" 
            status_name = "交易中"
        elif time(0, 0) <= current_time < morning_start:
            status = "pre_open"
            status_name = "开盘前"
        elif afternoon_end < current_time <= time(23, 59):
            status = "closed"
            status_name = "已收盘"
        else:
            status = "closed"
            status_name = "休市"
        
        return MarketStatus(
            market=exchange,
            is_open=status == "open",
            open_time=datetime.combine(now.date(), morning_start) if is_trading_day else None,
            close_time=datetime.combine(now.date(), afternoon_end) if is_trading_day else None,
            status=status,
            description=status_name
        )

    # ==================== 订阅接口 ====================

    def subscribe_realtime(self, codes: List[str]):
        """
        订阅实时行情

        Args:
            codes: 股票代码列表
        """
        if self._scheduler:
            self._scheduler.subscribe(codes)

    def unsubscribe_realtime(self, codes: List[str]):
        """
        取消订阅

        Args:
            codes: 股票代码列表
        """
        if self._scheduler:
            self._scheduler.unsubscribe(codes)

    # 缓存优先模式 - 不再提供回调函数设置
    # 所有数据更新都通过缓存机制实现，无需回调

    async def start_realtime(self):
        """启动实时数据更新"""
        if self._scheduler:
            await self._scheduler.start()

    async def stop_realtime(self):
        """停止实时数据更新"""
        if self._scheduler:
            await self._scheduler.stop()

    # ==================== 同步方法（供Rust调用） ====================
    
    def _get_stock_list_sync(self) -> List[Stock]:
        """
        同步获取股票列表（避免使用asyncio.get_event_loop()）
        
        Returns:
            股票列表
        """
        try:
            # 优先从缓存获取
            if self._cache and self._cache._initialized:
                # 使用同步方式访问缓存
                cache_key = "list:cn_a"
                logger.info(f"搜索股票列表 - scope: '{CacheScope.STOCK_LIST}', key: '{cache_key}'")
                if hasattr(self._cache, '_memory_cache') and cache_key in self._cache._memory_cache:
                    cached_data = self._cache._memory_cache[cache_key]
                    if cached_data:
                        print(f"💾 [CACHE] 从内存缓存获取到 {len(cached_data)} 条股票数据")
                        logger.info(f"从内存缓存获取到 {len(cached_data)} 条股票数据")
                        return [Stock.from_dict(s) for s in cached_data]
            
            # 如果缓存中没有，从客户端获取（使用同步方式）
            if self._client:
                print(f"🌐 [CLIENT] 从客户端同步获取股票列表")
                logger.info("从客户端同步获取股票列表")
                # 使用客户端的同步方法（如果有的话）或者创建临时事件循环
                import asyncio
                try:
                    # 尝试获取当前事件循环
                    loop = asyncio.get_running_loop()
                    # 如果有运行中的循环，使用run_coroutine_threadsafe
                    future = asyncio.run_coroutine_threadsafe(
                        self._client.get_stock_list(), 
                        loop
                    )
                    stock_basics = future.result(timeout=30)  # 30秒超时
                except RuntimeError:
                    # 没有运行中的事件循环，可以安全使用run_until_complete
                    loop = asyncio.new_event_loop()
                    asyncio.set_event_loop(loop)
                    try:
                        stock_basics = loop.run_until_complete(self._client.get_stock_list())
                    finally:
                        loop.close()
                
                logger.info(f"客户端返回 {len(stock_basics)} 条股票基础数据")
                
                # 转换为Stock模型
                stocks = []
                for basic in stock_basics:
                    stock = Stock(
                        code=basic.code,
                        name=basic.name,
                        market="cn_a",
                        industry=basic.industry,
                        area=basic.area,
                        listing_date=basic.listing_date
                    )
                    stocks.append(stock)
                
                logger.info(f"成功转换 {len(stocks)} 条股票数据")
                
                # 将数据设置到缓存中
                if self._cache and self._cache._initialized:
                    cache_key = "list:cn_a"
                    logger.info(f"缓存股票列表 - scope: '{CacheScope.STOCK_LIST}', key: '{cache_key}', 数据量: {len(stocks)} 条")
                    stocks_dict = [s.to_dict() for s in stocks]
                    self._cache._memory_cache[cache_key] = stocks_dict
                    self._cache._memory_cache_time[cache_key] = datetime.now()
                    self._cache._memory_cache_ttl[cache_key] = CacheTTL.STOCK_LIST
                    print(f"💾 [CACHE] 已将 {len(stocks)} 条股票数据设置到缓存")
                    logger.info(f"已将 {len(stocks)} 条股票数据设置到缓存")
                
                return stocks
            else:
                logger.warning("没有可用的客户端获取股票列表")
                return []
                
        except Exception as e:
            logger.error(f"同步获取股票列表失败: {e}", exc_info=True)
            return []

    def get_quotes_sync(self, codes: List[str]) -> List[Dict[str, Any]]:
        """同步获取行情"""
        loop = asyncio.get_event_loop()
        quotes = loop.run_until_complete(self.get_realtime_quotes(codes))
        return [q.to_dict() for q in quotes]

    def get_all_quotes_sync(self) -> List[Dict[str, Any]]:
        """同步获取所有行情"""
        loop = asyncio.get_event_loop()
        quotes = loop.run_until_complete(self.get_realtime_quotes())
        return [q.to_dict() for q in quotes]

    def get_klines_sync(
        self,
        code: str,
        period: str = "daily",
        adjust: str = "qfq",
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        limit: int = 500,
    ) -> List[Dict[str, Any]]:
        """同步获取K线"""

        period_map = {
            "daily": PeriodType.DAILY,
            "weekly": PeriodType.WEEKLY,
            "monthly": PeriodType.MONTHLY,
        }
        adjust_map = {
            "none": AdjustType.NONE,
            "qfq": AdjustType.QFQ,
            "hfq": AdjustType.HFQ,
        }

        start = datetime.strptime(start_date, "%Y%m%d").date() if start_date else None
        end = datetime.strptime(end_date, "%Y%m%d").date() if end_date else None

        loop = asyncio.get_event_loop()
        klines = loop.run_until_complete(
            self.get_klines(code, period_map[period], adjust_map[adjust], start, end, limit)
        )
        return [k.to_dict() for k in klines]

    # ==================== 缓存优先的查询方法 ====================

    def is_preload_complete(self) -> bool:
        """
        检查预加载是否已完成
        
        Returns:
            True 如果预加载已完成
        """
        if not self._preload_service:
            return False
        return self._preload_service.is_preload_complete()
    
    def get_preload_status(self) -> Dict[str, bool]:
        """
        获取预加载状态详情
        
        Returns:
            预加载状态字典，如果预加载服务未初始化则返回空字典
        """
        if not self._preload_service:
            return {}
        return self._preload_service._preload_status.copy()

    def is_cache_ready(self) -> bool:
        """
        检查缓存是否已准备就绪（有股票数据）
        
        Returns:
            True 如果缓存已准备就绪
        """
        # 首先检查预加载是否完成
        if not self.is_preload_complete():
            logger.debug("预加载尚未完成，缓存未就绪")
            return False
            
        if not self._cache or not self._cache._initialized:
            return False
        
        # 直接检查缓存中是否有股票列表数据
        import asyncio
        
        async def check_cache_data():
            try:
                logger.info(f"检查缓存数据 - scope: '{CacheScope.STOCK_LIST}', key: 'list:cn_a'")
                stock_list = await self._cache.get(CacheScope.STOCK_LIST, "list:cn_a")
                return stock_list is not None and len(stock_list) > 0
            except Exception:
                return False
        
        try:
            # 尝试获取当前事件循环
            loop = asyncio.get_running_loop()
            # 如果有运行中的循环，使用run_coroutine_threadsafe
            future = asyncio.run_coroutine_threadsafe(check_cache_data(), loop)
            return future.result(timeout=5)  # 5秒超时
        except RuntimeError:
            # 没有运行中的事件循环，可以安全使用run_until_complete
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                return loop.run_until_complete(check_cache_data())
            finally:
                loop.close()
        except Exception:
            return False



        loop = asyncio.get_event_loop()
        klines = loop.run_until_complete(
            self.get_klines(
                code,
                period=period_map.get(period, PeriodType.DAILY),
                adjust=adjust_map.get(adjust, AdjustType.QFQ),
                start_date=start,
                end_date=end,
                limit=limit,
            )
        )
        return [k.to_dict() for k in klines]

    def get_stock_detail_sync(self, code: str) -> Optional[Dict[str, Any]]:
        """同步获取股票详情"""
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            stock = loop.run_until_complete(self.get_stock_info(code))
            return stock.to_dict() if stock else None
        finally:
            loop.close()

    def get_financial_sync(
        self, code: str, report_type: str = "all"
    ) -> List[Dict[str, Any]]:
        """同步获取财务数据"""
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            financials = loop.run_until_complete(self.get_financial(code))
            return [f.to_dict() for f in financials]
        finally:
            loop.close()

    def get_fund_flow_sync(
        self, code: str, period: str = "daily"
    ) -> List[Dict[str, Any]]:
        """同步获取资金流向"""
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            flows = loop.run_until_complete(self.get_fund_flow(code))
            return [f.to_dict() for f in flows]
        finally:
            loop.close()

    def get_holders_sync(
        self, code: str, holder_type: str = "top10"
    ) -> List[Dict[str, Any]]:
        """同步获取股东信息"""
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            holders = loop.run_until_complete(self.get_holders(code, holder_type))
            return [h.to_dict() for h in holders]
        finally:
            loop.close()

    def get_announcements_sync(
        self, code: str, limit: int = 50
    ) -> List[Dict[str, Any]]:
        """同步获取公告"""
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            announcements = loop.run_until_complete(
                self.get_announcements(code, limit=limit)
            )
            return [a.to_dict() for a in announcements]
        finally:
            loop.close()

    def get_market_status_sync(self, exchange: str = "sse") -> Optional[Dict[str, Any]]:
        """同步获取市场状态"""
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            status = loop.run_until_complete(self.get_market_status(exchange))
            from dataclasses import asdict
            return asdict(status) if status else None
        finally:
            loop.close()

    def get_index_quotes_sync(self) -> List[Dict[str, Any]]:
        """同步获取指数行情"""
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            indices = loop.run_until_complete(self.get_index_quotes())
            return [i.to_dict() for i in indices]
        finally:
            loop.close()

    def get_lhb_data_sync(
        self, trade_date: Optional[str] = None
    ) -> List[Dict[str, Any]]:
        """同步获取龙虎榜"""
        # 龙虎榜功能暂时不支持
        return []

    def get_sector_list_sync(
        self, sector_type: str = "industry"
    ) -> List[Dict[str, Any]]:
        """同步获取板块列表"""
        # 板块列表功能暂时不支持
        return []

    def get_sector_stocks_sync(self, sector_code: str) -> List[str]:
        """同步获取板块成分股"""
        # 板块成分股功能暂时不支持
        return []

    def get_stock_ranking_sync(self, ranking_type: str = "gainers", limit: int = 10) -> List[Dict[str, Any]]:
        """同步获取股票排行榜
        
        Args:
            ranking_type: 排行榜类型 ("gainers": 涨幅榜, "losers": 跌幅榜, "hot": 人气榜)
            limit: 返回数量限制
            
        Returns:
            排行榜数据列表
        """
        # 检查缓存（2分钟缓存）
        cache_key = f"{ranking_type}:{limit}"
        if self._cache:
            cached = self._cache.get_ranking_sync(cache_key)
            if cached:
                logger.info(f"从缓存获取排行榜数据: {ranking_type}, 数量: {len(cached)}")
                return cached
        
        try:
            import akshare as ak
            
            if ranking_type == "gainers":
                # 使用创新高作为涨幅榜（获取涨幅较大的股票）
                try:
                    df = ak.stock_rank_lxsz_ths()
                    if df is not None and not df.empty:
                        # 转换为标准格式
                        results = []
                        for _, row in df.head(limit).iterrows():
                            results.append({
                                "code": str(row.get("股票代码", "")),
                                "name": str(row.get("股票简称", "")),
                                "price": float(row.get("最新价", 0)),
                                "change": float(row.get("涨跌幅", 0)),
                                "volume": int(row.get("成交量", 0)),
                                "type": "gainers"
                            })
                        # 缓存结果（2分钟）
                        if self._cache:
                            self._cache.set_ranking_sync(cache_key, results)
                        return results
                except Exception as e:
                    logger.warning(f"获取涨幅榜失败: {e}")
                    
            elif ranking_type == "losers":
                # 使用创新低作为跌幅榜（获取跌幅较大的股票）
                try:
                    df = ak.stock_rank_lxxd_ths()
                    if df is not None and not df.empty:
                        # 转换为标准格式
                        results = []
                        for _, row in df.head(limit).iterrows():
                            results.append({
                                "code": str(row.get("股票代码", "")),
                                "name": str(row.get("股票简称", "")),
                                "price": float(row.get("最新价", 0)),
                                "change": float(row.get("涨跌幅", 0)),
                                "volume": int(row.get("成交量", 0)),
                                "type": "losers"
                            })
                        # 缓存结果（2分钟）
                        if self._cache:
                            self._cache.set_ranking_sync(cache_key, results)
                        return results
                except Exception as e:
                    logger.warning(f"获取跌幅榜失败: {e}")
                    
            elif ranking_type == "hot":
                # 使用人气榜
                try:
                    df = ak.stock_hot_rank_em()
                    if df is not None and not df.empty:
                        # 转换为标准格式
                        results = []
                        for _, row in df.head(limit).iterrows():
                            results.append({
                                "code": str(row.get("代码", "")),
                                "name": str(row.get("股票名称", "")),
                                "price": float(row.get("最新价", 0)),
                                "change": float(row.get("涨跌幅", 0)),
                                "volume": int(row.get("成交量", 0)),
                                "type": "hot"
                            })
                        # 缓存结果（2分钟）
                        if self._cache:
                            self._cache.set_ranking_sync(cache_key, results)
                        return results
                except Exception as e:
                    logger.warning(f"获取人气榜失败: {e}")
                    
            elif ranking_type == "fund_flow":
                # 使用资金流向排行
                try:
                    df = ak.stock_individual_fund_flow_rank("今日")
                    if df is not None and not df.empty:
                        # 转换为标准格式
                        results = []
                        for _, row in df.head(limit).iterrows():
                            results.append({
                                "code": str(row.get("代码", "")),
                                "name": str(row.get("名称", "")),
                                "price": float(row.get("最新价", 0)),
                                "change": float(row.get("涨跌幅", 0)),
                                "fund_flow": float(row.get("主力净流入", 0)),
                                "type": "fund_flow"
                            })
                        # 缓存结果（2分钟）
                        if self._cache:
                            self._cache.set_ranking_sync(cache_key, results)
                        return results
                except Exception as e:
                    logger.warning(f"获取资金流向榜失败: {e}")
                    
        except Exception as e:
            logger.error(f"获取股票排行榜失败: {e}")
            
        return []

    # ==================== 统计接口 ====================

    def get_service_stats(self) -> Dict[str, Any]:
        """获取服务统计"""
        stats = {
            "initialized": self._initialized,
            "cache_enabled": self._cache is not None,
            "scheduler_enabled": self._scheduler is not None,
            "client_enabled": self._client is not None,
        }

        if self._scheduler:
            stats["scheduler"] = self._scheduler.get_stats()

        return stats

    async def close(self):
        """关闭服务"""
        # 停止事件处理器
        if self._event_handler:
            try:
                await self._event_handler.stop()
                logger.info("事件处理器已停止")
            except Exception as e:
                logger.error(f"事件处理器停止失败: {e}")
        
        # 停止任务队列
        if self._task_queue:
            await self._task_queue.stop()
        
        # 停止调度器
        if self._scheduler:
            await self._scheduler.stop()
        
        # 停止事件循环
        if self._event_loop and self._event_loop.is_running():
            self._event_loop.call_soon_threadsafe(self._event_loop.stop)

        logger.info("股票服务已关闭")
    
    def wait_for_background_tasks(self, timeout: int = 300):
        """等待后台任务完成（同步方法）
        
        Args:
            timeout: 超时时间（秒）
        """
        if not self._task_queue:
            logger.warning("任务队列未初始化")
            return
        
        logger.info(f"等待后台任务完成（超时: {timeout}秒）...")
        start_time = time.time()
        
        while time.time() - start_time < timeout:
            stats = self._task_queue.get_stats()
            progress = self._task_queue.get_progress()
            
            if stats['queue_size'] == 0 and stats['running_tasks'] == 0:
                logger.info(f"✅ 所有后台任务已完成: {progress['completed']}/{progress['total']}")
                return
            
            # 每5秒打印一次进度
            if int(time.time() - start_time) % 5 == 0:
                logger.info(
                    f"后台任务进度: {progress['completed']}/{progress['total']} "
                    f"({progress['progress']:.1%}), "
                    f"运行中: {stats['running_tasks']}, "
                    f"待处理: {stats['queue_size']}"
                )
            
            time.sleep(1)
        
        logger.warning(f"⚠️ 等待超时，部分任务可能未完成")


# 全局服务实例
_stock_service: Optional[StockService] = None


def get_stock_service() -> StockService:
    """获取全局股票服务"""
    global _stock_service
    if _stock_service is None:
        _stock_service = StockService()
    return _stock_service
