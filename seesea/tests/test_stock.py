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
股票数据模块测试

包含:
- 数据模型测试
- 数据提供者测试
- 缓存测试
- 服务层测试
"""

import pytest
from datetime import date, datetime


class TestStockEnums:
    """股票枚举测试"""

    def test_market_enum(self):
        """测试市场枚举"""
        from seesea.models import Market

        assert Market.CN_A.value == "cn_a"
        assert Market.CN_B.value == "cn_b"
        assert Market.HK.value == "hk"
        assert Market.US.value == "us"

    def test_exchange_enum(self):
        """测试交易所枚举"""
        from seesea.models import Exchange

        assert Exchange.SSE.value == "sse"
        assert Exchange.SZSE.value == "szse"
        assert Exchange.BSE.value == "bse"
        assert Exchange.HKEX.value == "hkex"

    def test_board_type_enum(self):
        """测试板块类型枚举"""
        from seesea.models import BoardType

        assert BoardType.MAIN_SSE.value == "main_sse"
        assert BoardType.MAIN_SZSE.value == "main_szse"
        assert BoardType.GEM.value == "gem"
        assert BoardType.STAR.value == "star"

    def test_period_type_enum(self):
        """测试周期类型枚举"""
        from seesea.models import PeriodType

        assert PeriodType.DAILY.value == "daily"
        assert PeriodType.WEEKLY.value == "weekly"
        assert PeriodType.MONTHLY.value == "monthly"

    def test_adjust_type_enum(self):
        """测试复权类型枚举"""
        from seesea.models import AdjustType

        assert AdjustType.NONE.value == "none"
        assert AdjustType.QFQ.value == "qfq"
        assert AdjustType.HFQ.value == "hfq"


class TestStockModels:
    """股票数据模型测试"""

    def test_stock_model_creation(self):
        """测试 Stock 模型创建"""
        from seesea.models import Stock, Market, Exchange, BoardType, DataSource

        stock = Stock(
            code="000001",
            name="平安银行",
            full_code="000001.SZ",
            market=Market.CN_A,
            exchange=Exchange.SZSE,
            board=BoardType.MAIN_SZSE,
            data_source=DataSource.AKSHARE,
        )

        assert stock.code == "000001"
        assert stock.name == "平安银行"
        assert stock.full_code == "000001.SZ"
        assert stock.market == Market.CN_A
        assert stock.exchange == Exchange.SZSE

    def test_stock_model_to_dict(self):
        """测试 Stock 模型序列化"""
        from seesea.models import Stock, Market, Exchange, BoardType, DataSource

        stock = Stock(
            code="000001",
            name="平安银行",
            full_code="000001.SZ",
            market=Market.CN_A,
            exchange=Exchange.SZSE,
            board=BoardType.MAIN_SZSE,
            data_source=DataSource.AKSHARE,
        )

        data = stock.to_dict()
        assert data["code"] == "000001"
        assert data["name"] == "平安银行"
        assert data["market"] == "cn_a"

    def test_stock_model_from_dict(self):
        """测试 Stock 模型反序列化"""
        from seesea.models import Stock, Market, Exchange

        data = {
            "code": "000001",
            "name": "平安银行",
            "full_code": "000001.SZ",
            "market": "cn_a",
            "exchange": "szse",
            "board": "main_szse",
            "data_source": "akshare",
        }

        stock = Stock.from_dict(data)
        assert stock.code == "000001"
        assert stock.name == "平安银行"
        assert stock.market == Market.CN_A
        assert stock.exchange == Exchange.SZSE

    def test_stock_quote_model(self):
        """测试 StockQuote 模型"""
        from seesea.models import StockQuote, DataSource

        quote = StockQuote(
            code="000001",
            name="平安银行",
            price=10.50,
            open=10.20,
            high=10.80,
            low=10.10,
            pre_close=10.30,
            volume=1000000,
            amount=10500000.0,
            change=0.20,
            change_pct=1.94,
            timestamp=datetime.now(),
            data_source=DataSource.AKSHARE,
        )

        assert quote.code == "000001"
        assert quote.price == 10.50
        assert quote.change_pct == 1.94

    def test_stock_kline_model(self):
        """测试 StockKLine 模型"""
        from seesea.models import StockKLine, PeriodType, AdjustType, DataSource

        kline = StockKLine(
            code="000001",
            date=date.today(),
            open=10.20,
            high=10.80,
            low=10.10,
            close=10.50,
            volume=1000000,
            amount=10500000.0,
            period=PeriodType.DAILY,
            adjust=AdjustType.QFQ,
            data_source=DataSource.AKSHARE,
        )

        assert kline.code == "000001"
        assert kline.close == 10.50
        assert kline.period == PeriodType.DAILY


class TestStockExchange:
    """交易所模型测试"""

    def test_create_sse(self):
        """测试创建上交所"""
        from seesea.models.exchange import StockExchange

        sse = StockExchange.create_sse()
        assert sse.code == "SSE"
        assert sse.name == "上海证券交易所"
        assert sse.timezone == "Asia/Shanghai"

    def test_create_szse(self):
        """测试创建深交所"""
        from seesea.models.exchange import StockExchange

        szse = StockExchange.create_szse()
        assert szse.code == "SZSE"
        assert szse.name == "深圳证券交易所"

    def test_get_exchange(self):
        """测试获取交易所"""
        from seesea.models.exchange import get_exchange

        sse = get_exchange("sse")
        assert sse is not None
        assert sse.code == "SSE"

        szse = get_exchange("szse")
        assert szse is not None
        assert szse.code == "SZSE"

        # 不存在的交易所
        unknown = get_exchange("unknown")
        assert unknown is None


class TestStockNameMapping:
    """股票名称映射测试"""

    def test_mapping_creation(self):
        """测试映射创建"""
        from seesea.stock.providers import StockNameMapping

        mapping = StockNameMapping()
        assert len(mapping) == 0
        assert mapping.is_stale()

    def test_build_from_stocks(self):
        """测试从股票列表构建映射"""
        from seesea.stock.providers import StockNameMapping
        from seesea.models import Stock, Market, Exchange, BoardType, DataSource

        mapping = StockNameMapping()

        stocks = [
            Stock(
                code="000001",
                name="平安银行",
                full_code="000001.SZ",
                market=Market.CN_A,
                exchange=Exchange.SZSE,
                board=BoardType.MAIN_SZSE,
                data_source=DataSource.AKSHARE,
            ),
            Stock(
                code="600519",
                name="贵州茅台",
                full_code="600519.SH",
                market=Market.CN_A,
                exchange=Exchange.SSE,
                board=BoardType.MAIN_SSE,
                data_source=DataSource.AKSHARE,
            ),
        ]

        mapping.build_from_stocks(stocks)

        assert len(mapping) == 2
        assert not mapping.is_stale()

    def test_get_code_by_name(self):
        """测试按名称获取代码"""
        from seesea.stock.providers import StockNameMapping
        from seesea.models import Stock, Market, Exchange, BoardType, DataSource

        mapping = StockNameMapping()
        stocks = [
            Stock(
                code="000001",
                name="平安银行",
                full_code="000001.SZ",
                market=Market.CN_A,
                exchange=Exchange.SZSE,
                board=BoardType.MAIN_SZSE,
                data_source=DataSource.AKSHARE,
            ),
        ]
        mapping.build_from_stocks(stocks)

        code = mapping.get_code_by_name("平安银行")
        assert code == "000001"

        # 不存在的名称
        code = mapping.get_code_by_name("不存在")
        assert code is None

    def test_get_name_by_code(self):
        """测试按代码获取名称"""
        from seesea.stock.providers import StockNameMapping
        from seesea.models import Stock, Market, Exchange, BoardType, DataSource

        mapping = StockNameMapping()
        stocks = [
            Stock(
                code="000001",
                name="平安银行",
                full_code="000001.SZ",
                market=Market.CN_A,
                exchange=Exchange.SZSE,
                board=BoardType.MAIN_SZSE,
                data_source=DataSource.AKSHARE,
            ),
        ]
        mapping.build_from_stocks(stocks)

        name = mapping.get_name_by_code("000001")
        assert name == "平安银行"

    def test_search_exact_code(self):
        """测试精确代码搜索"""
        from seesea.stock.providers import StockNameMapping
        from seesea.models import Stock, Market, Exchange, BoardType, DataSource

        mapping = StockNameMapping()
        stocks = [
            Stock(
                code="000001",
                name="平安银行",
                full_code="000001.SZ",
                market=Market.CN_A,
                exchange=Exchange.SZSE,
                board=BoardType.MAIN_SZSE,
                data_source=DataSource.AKSHARE,
            ),
        ]
        mapping.build_from_stocks(stocks)

        results = mapping.search("000001")
        assert len(results) == 1
        assert results[0]["code"] == "000001"
        assert results[0]["match_type"] == "code_exact"

    def test_search_exact_name(self):
        """测试精确名称搜索"""
        from seesea.stock.providers import StockNameMapping
        from seesea.models import Stock, Market, Exchange, BoardType, DataSource

        mapping = StockNameMapping()
        stocks = [
            Stock(
                code="000001",
                name="平安银行",
                full_code="000001.SZ",
                market=Market.CN_A,
                exchange=Exchange.SZSE,
                board=BoardType.MAIN_SZSE,
                data_source=DataSource.AKSHARE,
            ),
        ]
        mapping.build_from_stocks(stocks)

        results = mapping.search("平安银行")
        assert len(results) == 1
        assert results[0]["code"] == "000001"
        assert results[0]["match_type"] == "name_exact"

    def test_search_fuzzy(self):
        """测试模糊搜索"""
        from seesea.stock.providers import StockNameMapping
        from seesea.models import Stock, Market, Exchange, BoardType, DataSource

        mapping = StockNameMapping()
        stocks = [
            Stock(
                code="000001",
                name="平安银行",
                full_code="000001.SZ",
                market=Market.CN_A,
                exchange=Exchange.SZSE,
                board=BoardType.MAIN_SZSE,
                data_source=DataSource.AKSHARE,
            ),
            Stock(
                code="601318",
                name="中国平安",
                full_code="601318.SH",
                market=Market.CN_A,
                exchange=Exchange.SSE,
                board=BoardType.MAIN_SSE,
                data_source=DataSource.AKSHARE,
            ),
        ]
        mapping.build_from_stocks(stocks)

        results = mapping.search("平安")
        assert len(results) == 2
        # 两个都包含 "平安"


class TestCacheScope:
    """缓存作用域测试"""

    def test_cache_scope_values(self):
        """测试缓存作用域值"""
        from seesea.stock.cache import CacheScope

        assert CacheScope.STOCK_REALTIME == "stock:realtime"
        assert CacheScope.STOCK_KLINE == "stock:kline"
        assert CacheScope.STOCK_FINANCIAL == "stock:financial"

    def test_cache_ttl_values(self):
        """测试缓存 TTL 值"""
        from seesea.stock.cache import CacheTTL

        assert CacheTTL.REALTIME == 5
        assert CacheTTL.SHORT == 300
        assert CacheTTL.MEDIUM == 1800
        assert CacheTTL.LONG == 43200


class TestStockCacheManager:
    """缓存管理器测试"""

    @pytest.mark.asyncio
    async def test_memory_cache_fallback(self):
        """测试内存缓存回退"""
        from seesea.stock.cache import StockCacheManager, CacheScope

        manager = StockCacheManager(":memory:")
        await manager.initialize()

        # 设置值
        await manager.set(CacheScope.STOCK_INFO, "test", {"name": "test"})

        # 获取值
        result = await manager.get(CacheScope.STOCK_INFO, "test")
        assert result is not None
        assert result["name"] == "test"

    @pytest.mark.asyncio
    async def test_realtime_quote_cache(self):
        """测试实时行情缓存"""
        from seesea.stock.cache import StockCacheManager

        manager = StockCacheManager(":memory:")
        await manager.initialize()

        quote_data = {
            "code": "000001",
            "name": "平安银行",
            "price": 10.50,
        }

        await manager.set_realtime_quote("000001", quote_data)
        result = await manager.get_realtime_quote("000001")

        assert result is not None
        assert result["code"] == "000001"
        assert result["price"] == 10.50

    @pytest.mark.asyncio
    async def test_cache_delete(self):
        """测试缓存删除"""
        from seesea.stock.cache import StockCacheManager, CacheScope

        manager = StockCacheManager(":memory:")
        await manager.initialize()

        await manager.set(CacheScope.STOCK_INFO, "test", {"name": "test"})
        await manager.delete(CacheScope.STOCK_INFO, "test")

        result = await manager.get(CacheScope.STOCK_INFO, "test")
        assert result is None


class TestStockScheduler:
    """调度器测试"""

    def test_scheduler_creation(self):
        """测试调度器创建"""
        from seesea.stock.scheduler import StockScheduler

        scheduler = StockScheduler()
        assert scheduler is not None
        assert not scheduler.is_running()

    def test_register_task(self):
        """测试任务注册"""
        from seesea.stock.scheduler import StockScheduler, UpdateFrequency

        scheduler = StockScheduler()

        async def dummy_callback():
            pass

        scheduler.register_task(
            name="test_task",
            callback=dummy_callback,
            frequency=UpdateFrequency.SHORT,
        )

        # 注册后应该有一个任务
        tasks = scheduler.get_tasks()
        assert len(tasks) == 1
        assert tasks[0]["name"] == "test_task"

    def test_subscribe_codes(self):
        """测试订阅代码"""
        from seesea.stock.scheduler import StockScheduler

        scheduler = StockScheduler()

        scheduler.subscribe(["000001", "600519"])
        codes = scheduler.get_subscribed_codes()

        assert "000001" in codes
        assert "600519" in codes

    def test_unsubscribe_codes(self):
        """测试取消订阅"""
        from seesea.stock.scheduler import StockScheduler

        scheduler = StockScheduler()

        scheduler.subscribe(["000001", "600519"])
        scheduler.unsubscribe(["000001"])

        codes = scheduler.get_subscribed_codes()
        assert "000001" not in codes
        assert "600519" in codes

    def test_is_trading_time(self):
        """测试交易时间判断"""
        from seesea.stock.scheduler import StockScheduler

        scheduler = StockScheduler()

        # 这个测试结果取决于执行时间
        result = scheduler.is_trading_time()
        assert isinstance(result, bool)


class TestStockService:
    """股票服务测试"""

    def test_service_creation(self):
        """测试服务创建"""
        from seesea.stock import StockService

        service = StockService(enable_cache=False, enable_scheduler=False)
        assert service is not None

    @pytest.mark.asyncio
    async def test_service_initialize(self):
        """测试服务初始化"""
        from seesea.stock import StockService

        service = StockService(enable_cache=False, enable_scheduler=False)
        result = await service.initialize()

        # 初始化可能成功或失败（取决于依赖）
        assert isinstance(result, bool)

    def test_search_stocks_without_init(self):
        """测试未初始化时搜索"""
        from seesea.stock import StockService

        service = StockService(enable_cache=False, enable_scheduler=False)

        # 未初始化时返回空列表
        results = service.search_stocks("平安")
        assert results == []

    def test_get_code_by_name_without_init(self):
        """测试未初始化时按名称获取代码"""
        from seesea.stock import StockService

        service = StockService(enable_cache=False, enable_scheduler=False)

        # 未初始化时返回 None
        code = service.get_code_by_name("平安银行")
        assert code is None


class TestImports:
    """导入测试 - 确保所有模块可以正常导入"""

    def test_import_models(self):
        """测试导入模型"""
        from seesea.models import (
            Stock,
            Market,
        )

        # 只要能导入就通过
        assert Stock is not None
        assert Market is not None

    def test_import_exchange(self):
        """测试导入交易所模块"""
        from seesea.models.exchange import (
            StockExchange,
            EXCHANGES,
        )

        assert StockExchange is not None
        assert EXCHANGES is not None

    def test_import_stock_module(self):
        """测试导入股票模块"""
        from seesea.stock import (
            StockService,
            AKShareProvider,
        )

        assert StockService is not None
        assert AKShareProvider is not None

    def test_import_cache(self):
        """测试导入缓存模块"""
        from seesea.stock.cache import (
            CacheScope,
            StockCacheManager,
        )

        assert CacheScope is not None
        assert StockCacheManager is not None


# ==================== 集成测试 ====================


@pytest.mark.integration
class TestIntegration:
    """集成测试 - 需要网络和外部依赖"""

    @pytest.mark.asyncio
    async def test_akshare_provider_get_stock_list(self):
        """测试 AKShare 获取股票列表"""
        try:
            from seesea.stock.providers import AKShareProvider

            provider = AKShareProvider()
            stocks = await provider.get_stock_list()

            assert len(stocks) > 0
            assert stocks[0].code is not None
            assert stocks[0].name is not None
        except ImportError:
            pytest.skip("akshare not installed")

    @pytest.mark.asyncio
    async def test_service_get_stock_list(self):
        """测试服务获取股票列表"""
        try:
            from seesea.stock import StockService

            service = StockService(enable_cache=False, enable_scheduler=False)
            await service.initialize()

            stocks = await service.get_stock_list()

            if stocks:  # 可能因为网络原因为空
                assert stocks[0].code is not None
        except ImportError:
            pytest.skip("Dependencies not installed")

    @pytest.mark.asyncio
    async def test_service_search_after_init(self):
        """测试初始化后搜索"""
        try:
            from seesea.stock import StockService

            service = StockService(enable_cache=False, enable_scheduler=False)
            await service.initialize()

            # 需要先获取股票列表来初始化映射
            await service.get_stock_list()

            # 搜索
            results = service.search_stocks("平安")

            # 结果可能为空（取决于网络）
            if results:
                assert "code" in results[0]
                assert "name" in results[0]
        except ImportError:
            pytest.skip("Dependencies not installed")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
