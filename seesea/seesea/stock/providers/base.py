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
数据提供者基类

定义数据提供者的抽象接口。
"""

from abc import ABC, abstractmethod
from typing import Optional, List, Dict, Any
from datetime import date

from seesea.models import (
    Stock,
    StockQuote,
    StockKLine,
    StockFinancial,
    StockHolder,
    StockAnnouncement,
    StockFundFlow,
    StockIndustry,
    PeriodType,
    AdjustType,
)
from seesea.models.exchange import IndexQuote, MarketStatus


class BaseProvider(ABC):
    """
    数据提供者基类

    定义所有数据提供者必须实现的接口。
    不同的数据源通过继承此类并实现相应方法来提供数据。
    """

    @property
    @abstractmethod
    def name(self) -> str:
        """提供者名称"""
        pass

    @property
    @abstractmethod
    def supported_markets(self) -> List[str]:
        """支持的市场列表"""
        pass

    # ==================== 股票列表 ====================

    @abstractmethod
    async def get_stock_list(self, market: str = "cn_a") -> List[Stock]:
        """
        获取股票列表

        Args:
            market: 市场类型 (cn_a, cn_b, hk, us)

        Returns:
            股票列表
        """
        pass

    @abstractmethod
    async def get_stock_info(self, code: str) -> Optional[Stock]:
        """
        获取单只股票基础信息

        Args:
            code: 股票代码

        Returns:
            股票信息
        """
        pass

    # ==================== 实时行情 ====================

    @abstractmethod
    async def get_realtime_quotes(
        self, codes: List[str] | None = None
    ) -> List[StockQuote]:
        """
        获取实时行情

        Args:
            codes: 股票代码列表，None表示获取全部

        Returns:
            实时行情列表
        """
        pass

    @abstractmethod
    async def get_quote(self, code: str) -> Optional[StockQuote]:
        """
        获取单只股票实时行情

        Args:
            code: 股票代码

        Returns:
            实时行情
        """
        pass

    # ==================== 历史K线 ====================

    @abstractmethod
    async def get_klines(
        self,
        code: str,
        period: PeriodType = PeriodType.DAILY,
        adjust: AdjustType = AdjustType.QFQ,
        start_date: Optional[date] = None,
        end_date: Optional[date] = None,
        limit: int = 1000,
    ) -> List[StockKLine]:
        """
        获取历史K线数据

        Args:
            code: 股票代码
            period: K线周期
            adjust: 复权类型
            start_date: 开始日期
            end_date: 结束日期
            limit: 数据条数限制

        Returns:
            K线数据列表
        """
        pass

    # ==================== 财务数据 ====================

    @abstractmethod
    async def get_financial(
        self,
        code: str,
        report_type: str = "all",
    ) -> List[StockFinancial]:
        """
        获取财务数据

        Args:
            code: 股票代码
            report_type: 报告类型 (all, annual, quarterly)

        Returns:
            财务数据列表
        """
        pass

    # ==================== 股东信息 ====================

    @abstractmethod
    async def get_holders(
        self,
        code: str,
        holder_type: str = "top10",
    ) -> List[StockHolder]:
        """
        获取股东信息

        Args:
            code: 股票代码
            holder_type: 股东类型 (top10, top10_float, institution, fund)

        Returns:
            股东信息列表
        """
        pass

    # ==================== 公告信息 ====================

    @abstractmethod
    async def get_announcements(
        self,
        code: str | None = None,
        start_date: Optional[date] = None,
        end_date: Optional[date] = None,
        limit: int = 100,
    ) -> List[StockAnnouncement]:
        """
        获取公告信息

        Args:
            code: 股票代码，None表示全市场
            start_date: 开始日期
            end_date: 结束日期
            limit: 数据条数限制

        Returns:
            公告列表
        """
        pass

    # ==================== 资金流向 ====================

    @abstractmethod
    async def get_fund_flow(
        self,
        code: str,
        period: str = "daily",
    ) -> List[StockFundFlow]:
        """
        获取资金流向

        Args:
            code: 股票代码
            period: 周期 (realtime, daily, 3day, 5day, 10day)

        Returns:
            资金流向数据
        """
        pass

    # ==================== 行业分类 ====================

    @abstractmethod
    async def get_industry(
        self,
        code: str,
        classification: str = "sw",
    ) -> Optional[StockIndustry]:
        """
        获取行业分类

        Args:
            code: 股票代码
            classification: 分类标准 (csrc, sw, ths, em)

        Returns:
            行业分类信息
        """
        pass

    # ==================== 指数数据 ====================

    @abstractmethod
    async def get_index_quotes(self) -> List[IndexQuote]:
        """
        获取主要指数行情

        Returns:
            指数行情列表
        """
        pass

    # ==================== 市场状态 ====================

    @abstractmethod
    async def get_market_status(self, exchange: str = "sse") -> Optional[MarketStatus]:
        """
        获取市场状态

        Args:
            exchange: 交易所代码

        Returns:
            市场状态
        """
        pass

    # ==================== 龙虎榜数据 ====================

    async def get_lhb_data(
        self,
        trade_date: Optional[date] = None,
    ) -> List[Dict[str, Any]]:
        """
        获取龙虎榜数据

        Args:
            trade_date: 交易日期，默认最新

        Returns:
            龙虎榜数据
        """
        return []

    # ==================== 板块数据 ====================

    async def get_sector_list(
        self, sector_type: str = "industry"
    ) -> List[Dict[str, Any]]:
        """
        获取板块列表

        Args:
            sector_type: 板块类型 (industry, concept, region)

        Returns:
            板块列表
        """
        return []

    async def get_sector_stocks(self, sector_code: str) -> List[str]:
        """
        获取板块成分股

        Args:
            sector_code: 板块代码

        Returns:
            成分股代码列表
        """
        return []
