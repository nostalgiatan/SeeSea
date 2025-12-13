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

from .providers import AKShareProvider, CNInfoProvider
from .cache import StockCacheManager, CacheScope
from .scheduler import StockScheduler, UpdateFrequency

from seesea.models import (
    Stock,
    StockQuote,
    StockKLine,
    StockFinancial,
    StockHolder,
    StockAnnouncement,
    StockFundFlow,
    PeriodType,
    AdjustType,
)
from seesea.models.exchange import IndexQuote, MarketStatus

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
        cache_path: Optional[str] = None,
    ):
        """
        初始化股票服务

        Args:
            enable_cache: 是否启用缓存
            enable_scheduler: 是否启用调度器
            cache_path: 缓存路径，None 则使用平台默认路径
        """
        self._enable_cache = enable_cache
        self._enable_scheduler = enable_scheduler

        # 缓存路径：优先使用传入值，否则使用后备默认路径
        if cache_path is None:
            cache_path = _get_default_cache_path()

        # 数据提供者
        self._akshare_provider: Optional[AKShareProvider] = None
        self._cninfo_provider: Optional[CNInfoProvider] = None

        # 缓存管理器
        self._cache: Optional[StockCacheManager] = None
        if enable_cache:
            self._cache = StockCacheManager(cache_path)

        # 调度器
        self._scheduler: Optional[StockScheduler] = None
        if enable_scheduler:
            self._scheduler = StockScheduler()

        # 回调
        self._on_quote_update: Optional[Callable] = None
        self._on_announcement_update: Optional[Callable] = None

        # 初始化状态
        self._initialized = False

    async def initialize(self) -> bool:
        """初始化服务"""
        if self._initialized:
            return True

        try:
            # 初始化 AKShare 提供者
            try:
                self._akshare_provider = AKShareProvider()
                logger.info("AKShare 提供者初始化成功")
            except ImportError as e:
                logger.warning(f"AKShare 不可用: {e}")

            # 初始化巨潮资讯提供者
            try:
                self._cninfo_provider = CNInfoProvider()
                logger.info("巨潮资讯提供者初始化成功")
            except ImportError as e:
                logger.warning(f"巨潮资讯不可用: {e}")

            # 初始化缓存
            if self._cache:
                await self._cache.initialize()

            # 注册调度任务
            if self._scheduler:
                self._register_scheduled_tasks()

            self._initialized = True
            logger.info("股票服务初始化完成")
            return True

        except Exception as e:
            logger.error(f"股票服务初始化失败: {e}")
            return False

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

        # 缓存更新
        if self._cache and quotes:
            for quote in quotes:
                await self._cache.set_realtime_quote(quote.code, quote.to_dict())

        # 回调
        if self._on_quote_update and quotes:
            await self._on_quote_update(quotes)

        return quotes

    async def _update_index_quotes(self) -> List[IndexQuote]:
        """更新指数行情"""
        if not self._akshare_provider:
            return []

        indices = await self._akshare_provider.get_index_quotes()

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

        if not codes or not self._akshare_provider:
            return []

        all_flows = []
        for code in codes[:10]:  # 限制数量避免过多请求
            flows = await self._akshare_provider.get_fund_flow(code)
            all_flows.extend(flows)

            # 缓存
            if self._cache and flows:
                await self._cache.set(
                    CacheScope.STOCK_FUND_FLOW,
                    code,
                    [f.to_dict() for f in flows],
                )

        return all_flows

    async def _update_announcements(self) -> List[StockAnnouncement]:
        """更新公告"""
        if not self._cninfo_provider:
            return []

        announcements = await self._cninfo_provider.get_announcements(limit=50)

        # 缓存
        if self._cache and announcements:
            await self._cache.set(
                CacheScope.STOCK_ANNOUNCEMENT,
                "latest",
                [a.to_dict() for a in announcements],
            )

        # 回调
        if self._on_announcement_update and announcements:
            await self._on_announcement_update(announcements)

        return announcements

    async def _update_stock_info(self) -> List[Stock]:
        """更新股票基础信息"""
        if not self._akshare_provider:
            return []

        stocks = await self._akshare_provider.get_stock_list()

        # 缓存
        if self._cache and stocks:
            for stock in stocks:
                await self._cache.set_stock_info(stock.code, stock.to_dict())

        return stocks

    # ==================== 公开接口 ====================

    async def get_stock_list(self, market: str = "cn_a") -> List[Stock]:
        """
        获取股票列表

        Args:
            market: 市场类型

        Returns:
            股票列表
        """
        # 检查缓存
        if self._cache:
            cached = await self._cache.get(CacheScope.STOCK_LIST, market)
            if cached:
                return [Stock.from_dict(s) for s in cached]

        # 从数据源获取
        if self._akshare_provider:
            stocks = await self._akshare_provider.get_stock_list(market)

            # 缓存
            if self._cache and stocks:
                await self._cache.set(
                    CacheScope.STOCK_LIST,
                    market,
                    [s.to_dict() for s in stocks],
                )

            return stocks

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
        if self._akshare_provider:
            return self._akshare_provider.get_code_by_name(name)
        return None

    def get_name_by_code(self, code: str) -> Optional[str]:
        """
        根据股票代码获取名称

        Args:
            code: 股票代码，如 "000001"

        Returns:
            股票名称，如 "平安银行"，未找到返回 None
        """
        if self._akshare_provider:
            return self._akshare_provider.get_name_by_code(code)
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

        # 从数据源获取
        if self._akshare_provider:
            stock = await self._akshare_provider.get_stock_info(code)

            # 缓存
            if self._cache and stock:
                await self._cache.set_stock_info(code, stock.to_dict())

            return stock

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

        # 从数据源获取
        if self._akshare_provider:
            quote = await self._akshare_provider.get_quote(code)

            # 缓存
            if self._cache and quote:
                await self._cache.set_realtime_quote(code, quote.to_dict())

            return quote

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
        if self._akshare_provider:
            return await self._akshare_provider.get_realtime_quotes(codes)
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
        # cache_key = f"{code}:{period.value}:{adjust.value}"

        # 检查缓存
        if use_cache and self._cache:
            cached = await self._cache.get_klines(code, period.value)
            if cached:
                return [StockKLine(**k) for k in cached]

        # 从数据源获取
        if self._akshare_provider:
            klines = await self._akshare_provider.get_klines(
                code=code,
                period=period,
                adjust=adjust,
                start_date=start_date,
                end_date=end_date,
                limit=limit,
            )

            # 缓存
            if self._cache and klines:
                await self._cache.set_klines(
                    code,
                    period.value,
                    [k.to_dict() for k in klines],
                )

            return klines

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

        # 从数据源获取
        financials = []

        if self._akshare_provider:
            financials = await self._akshare_provider.get_financial(code)

        # 补充巨潮资讯数据
        if self._cninfo_provider:
            # cninfo_financials = await self._cninfo_provider.get_financial(code)
            # 合并数据...
            pass

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
    ) -> List[StockHolder]:
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
                return [StockHolder(**h) for h in cached]

        # 从数据源获取
        if self._akshare_provider:
            holders = await self._akshare_provider.get_holders(code, holder_type)

            # 缓存
            if self._cache and holders:
                await self._cache.set(
                    CacheScope.STOCK_HOLDER,
                    cache_key,
                    [h.to_dict() for h in holders],
                )

            return holders

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

        # 从巨潮资讯获取
        if self._cninfo_provider:
            announcements = await self._cninfo_provider.get_announcements(
                code=code,
                limit=limit,
            )

            # 缓存
            if self._cache and announcements:
                await self._cache.set_announcements(
                    cache_key,
                    [a.to_dict() for a in announcements],
                )

            return announcements

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

        # 从数据源获取
        if self._akshare_provider:
            flows = await self._akshare_provider.get_fund_flow(code)

            # 缓存
            if self._cache and flows:
                await self._cache.set(
                    CacheScope.STOCK_FUND_FLOW,
                    code,
                    [f.to_dict() for f in flows],
                )

            return flows

        return []

    async def get_index_quotes(self) -> List[IndexQuote]:
        """
        获取指数行情

        Returns:
            指数行情列表
        """
        if self._akshare_provider:
            return await self._akshare_provider.get_index_quotes()
        return []

    async def get_market_status(self, exchange: str = "sse") -> Optional[MarketStatus]:
        """
        获取市场状态

        Args:
            exchange: 交易所代码

        Returns:
            市场状态
        """
        if self._akshare_provider:
            return await self._akshare_provider.get_market_status(exchange)
        return None

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

    def set_on_quote_update(self, callback: Callable):
        """设置行情更新回调"""
        self._on_quote_update = callback

    def set_on_announcement_update(self, callback: Callable):
        """设置公告更新回调"""
        self._on_announcement_update = callback

    async def start_realtime(self):
        """启动实时数据更新"""
        if self._scheduler:
            await self._scheduler.start()

    async def stop_realtime(self):
        """停止实时数据更新"""
        if self._scheduler:
            await self._scheduler.stop()

    # ==================== 同步方法（供Rust调用） ====================

    def search_stocks(self, query: str, limit: int = 20) -> List[Dict[str, Any]]:
        """
        同步搜索股票

        Args:
            query: 搜索关键词
            limit: 返回数量限制

        Returns:
            搜索结果列表
        """
        from .providers.akshare_provider import get_stock_mapping

        mapping = get_stock_mapping()

        # 如果映射为空或过期，尝试刷新
        if mapping.is_stale() and self._akshare_provider:
            try:
                loop = asyncio.get_event_loop()
                stocks = loop.run_until_complete(
                    self._akshare_provider.get_stock_list()
                )
                mapping.build_from_stocks(stocks)
            except Exception as e:
                logger.warning(f"刷新股票映射失败: {e}")

        return mapping.search(query, limit)

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
        loop = asyncio.get_event_loop()
        stock = loop.run_until_complete(self.get_stock_info(code))
        return stock.to_dict() if stock else None

    def get_financial_sync(
        self, code: str, report_type: str = "all"
    ) -> List[Dict[str, Any]]:
        """同步获取财务数据"""
        loop = asyncio.get_event_loop()
        financials = loop.run_until_complete(self.get_financial(code))
        return [f.to_dict() for f in financials]

    def get_fund_flow_sync(
        self, code: str, period: str = "daily"
    ) -> List[Dict[str, Any]]:
        """同步获取资金流向"""
        loop = asyncio.get_event_loop()
        flows = loop.run_until_complete(self.get_fund_flow(code))
        return [f.to_dict() for f in flows]

    def get_holders_sync(
        self, code: str, holder_type: str = "top10"
    ) -> List[Dict[str, Any]]:
        """同步获取股东信息"""
        loop = asyncio.get_event_loop()
        holders = loop.run_until_complete(self.get_holders(code, holder_type))
        return [h.to_dict() for h in holders]

    def get_announcements_sync(
        self, code: str, limit: int = 50
    ) -> List[Dict[str, Any]]:
        """同步获取公告"""
        loop = asyncio.get_event_loop()
        announcements = loop.run_until_complete(
            self.get_announcements(code, limit=limit)
        )
        return [a.to_dict() for a in announcements]

    def get_market_status_sync(self, exchange: str = "sse") -> Optional[Dict[str, Any]]:
        """同步获取市场状态"""
        loop = asyncio.get_event_loop()
        status = loop.run_until_complete(self.get_market_status(exchange))
        from dataclasses import asdict

        return asdict(status) if status else None

    def get_index_quotes_sync(self) -> List[Dict[str, Any]]:
        """同步获取指数行情"""
        loop = asyncio.get_event_loop()
        indices = loop.run_until_complete(self.get_index_quotes())
        return [i.to_dict() for i in indices]

    def get_lhb_data_sync(
        self, trade_date: Optional[str] = None
    ) -> List[Dict[str, Any]]:
        """同步获取龙虎榜"""

        d = datetime.strptime(trade_date, "%Y%m%d").date() if trade_date else None
        loop = asyncio.get_event_loop()

        if self._akshare_provider:
            lhb = loop.run_until_complete(self._akshare_provider.get_lhb_data(d))
            return lhb
        return []

    def get_sector_list_sync(
        self, sector_type: str = "industry"
    ) -> List[Dict[str, Any]]:
        """同步获取板块列表"""
        loop = asyncio.get_event_loop()

        if self._akshare_provider:
            sectors = loop.run_until_complete(
                self._akshare_provider.get_sector_list(sector_type)
            )
            return sectors
        return []

    def get_sector_stocks_sync(self, sector_code: str) -> List[str]:
        """同步获取板块成分股"""
        loop = asyncio.get_event_loop()

        if self._akshare_provider:
            stocks = loop.run_until_complete(
                self._akshare_provider.get_sector_stocks(sector_code)
            )
            return stocks
        return []

    # ==================== 统计接口 ====================

    def get_service_stats(self) -> Dict[str, Any]:
        """获取服务统计"""
        stats = {
            "initialized": self._initialized,
            "cache_enabled": self._cache is not None,
            "scheduler_enabled": self._scheduler is not None,
            "providers": {
                "akshare": self._akshare_provider is not None,
                "cninfo": self._cninfo_provider is not None,
            },
        }

        if self._scheduler:
            stats["scheduler"] = self._scheduler.get_stats()

        return stats

    async def close(self):
        """关闭服务"""
        if self._scheduler:
            await self._scheduler.stop()

        if self._cninfo_provider:
            await self._cninfo_provider.close()

        logger.info("股票服务已关闭")


# 全局服务实例
_stock_service: Optional[StockService] = None


def get_stock_service() -> StockService:
    """获取全局股票服务"""
    global _stock_service
    if _stock_service is None:
        _stock_service = StockService()
    return _stock_service
