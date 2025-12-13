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
股票交易所模型

提供交易所级别的管理功能和市场概况数据。
"""

from dataclasses import dataclass, field
from datetime import datetime, time
from typing import Optional, List, Dict, Any
from enum import Enum

from .enums import Exchange


class TradingSession(str, Enum):
    """交易时段"""

    PRE_MARKET = "pre_market"  # 盘前
    MORNING = "morning"  # 上午盘
    LUNCH_BREAK = "lunch_break"  # 午休
    AFTERNOON = "afternoon"  # 下午盘
    AFTER_MARKET = "after_market"  # 盘后
    CLOSED = "closed"  # 休市


@dataclass
class TradingTime:
    """交易时间配置"""

    session: TradingSession
    start_time: time
    end_time: time


@dataclass
class MarketStatus:
    """
    市场状态

    描述某个交易所/市场的当前状态。
    """

    exchange: Exchange
    is_trading_day: bool  # 是否交易日
    current_session: TradingSession  # 当前交易时段
    next_open_time: Optional[datetime] = None  # 下次开盘时间

    # 市场概况
    total_stocks: int = 0  # 股票总数
    trading_stocks: int = 0  # 交易中股票数
    suspended_stocks: int = 0  # 停牌股票数

    # 涨跌统计
    up_count: int = 0  # 上涨家数
    down_count: int = 0  # 下跌家数
    flat_count: int = 0  # 平盘家数
    limit_up_count: int = 0  # 涨停家数
    limit_down_count: int = 0  # 跌停家数

    # 成交统计
    total_volume: float = 0.0  # 总成交量 (亿手)
    total_turnover: float = 0.0  # 总成交额 (亿元)

    # 时间戳
    update_time: Optional[datetime] = None


@dataclass
class IndexQuote:
    """
    指数行情

    主要股票指数的实时行情。
    """

    code: str  # 指数代码
    name: str  # 指数名称

    # 价格数据
    price: float = 0.0  # 最新点位
    change: float = 0.0  # 涨跌点数
    change_pct: float = 0.0  # 涨跌幅 (%)

    # 成交数据
    volume: float = 0.0  # 成交量 (亿手)
    turnover: float = 0.0  # 成交额 (亿元)

    # 时间戳
    update_time: Optional[datetime] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "code": self.code,
            "name": self.name,
            "price": self.price,
            "change": self.change,
            "change_pct": self.change_pct,
            "volume": self.volume,
            "turnover": self.turnover,
            "update_time": self.update_time.isoformat() if self.update_time else None,
        }


@dataclass
class StockExchange:
    """
    股票交易所

    提供交易所级别的信息和管理功能。
    包括交易时间、市场状态、主要指数等。
    """

    exchange: Exchange
    name: str  # 交易所名称
    name_en: str  # 英文名称
    timezone: str = "Asia/Shanghai"  # 时区
    currency: str = "CNY"  # 交易货币

    # 交易时间配置
    trading_times: List[TradingTime] = field(default_factory=list)

    # 市场状态
    status: Optional[MarketStatus] = None

    # 主要指数
    main_indices: List[IndexQuote] = field(default_factory=list)

    # 元数据
    update_time: Optional[datetime] = None

    @classmethod
    def create_sse(cls) -> "StockExchange":
        """创建上海证券交易所"""
        return cls(
            exchange=Exchange.SSE,
            name="上海证券交易所",
            name_en="Shanghai Stock Exchange",
            timezone="Asia/Shanghai",
            currency="CNY",
            trading_times=[
                TradingTime(TradingSession.MORNING, time(9, 30), time(11, 30)),
                TradingTime(TradingSession.AFTERNOON, time(13, 0), time(15, 0)),
            ],
        )

    @classmethod
    def create_szse(cls) -> "StockExchange":
        """创建深圳证券交易所"""
        return cls(
            exchange=Exchange.SZSE,
            name="深圳证券交易所",
            name_en="Shenzhen Stock Exchange",
            timezone="Asia/Shanghai",
            currency="CNY",
            trading_times=[
                TradingTime(TradingSession.MORNING, time(9, 30), time(11, 30)),
                TradingTime(TradingSession.AFTERNOON, time(13, 0), time(15, 0)),
            ],
        )

    @classmethod
    def create_bse(cls) -> "StockExchange":
        """创建北京证券交易所"""
        return cls(
            exchange=Exchange.BSE,
            name="北京证券交易所",
            name_en="Beijing Stock Exchange",
            timezone="Asia/Shanghai",
            currency="CNY",
            trading_times=[
                TradingTime(TradingSession.MORNING, time(9, 30), time(11, 30)),
                TradingTime(TradingSession.AFTERNOON, time(13, 0), time(15, 0)),
            ],
        )

    @classmethod
    def create_hkex(cls) -> "StockExchange":
        """创建香港交易所"""
        return cls(
            exchange=Exchange.HKEX,
            name="香港交易所",
            name_en="Hong Kong Stock Exchange",
            timezone="Asia/Hong_Kong",
            currency="HKD",
            trading_times=[
                TradingTime(TradingSession.MORNING, time(9, 30), time(12, 0)),
                TradingTime(TradingSession.AFTERNOON, time(13, 0), time(16, 0)),
            ],
        )

    def get_current_session(self) -> TradingSession:
        """获取当前交易时段"""
        now = datetime.now().time()

        for trading_time in self.trading_times:
            if trading_time.start_time <= now <= trading_time.end_time:
                return trading_time.session

        # 判断是否在午休
        if len(self.trading_times) >= 2:
            morning = self.trading_times[0]
            afternoon = self.trading_times[1]
            if morning.end_time < now < afternoon.start_time:
                return TradingSession.LUNCH_BREAK

        # 判断盘前盘后
        if self.trading_times:
            first_session = self.trading_times[0]
            last_session = self.trading_times[-1]

            if now < first_session.start_time:
                return TradingSession.PRE_MARKET
            elif now > last_session.end_time:
                return TradingSession.AFTER_MARKET

        return TradingSession.CLOSED

    def is_trading_time(self) -> bool:
        """判断当前是否为交易时间"""
        session = self.get_current_session()
        return session in (TradingSession.MORNING, TradingSession.AFTERNOON)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "exchange": self.exchange.value,
            "name": self.name,
            "name_en": self.name_en,
            "timezone": self.timezone,
            "currency": self.currency,
            "is_trading_time": self.is_trading_time(),
            "current_session": self.get_current_session().value,
            "main_indices": [idx.to_dict() for idx in self.main_indices],
            "update_time": self.update_time.isoformat() if self.update_time else None,
        }


# 预定义交易所实例
EXCHANGES: Dict[Exchange, StockExchange] = {
    Exchange.SSE: StockExchange.create_sse(),
    Exchange.SZSE: StockExchange.create_szse(),
    Exchange.BSE: StockExchange.create_bse(),
    Exchange.HKEX: StockExchange.create_hkex(),
}


def get_exchange(exchange: Exchange) -> StockExchange:
    """获取交易所实例"""
    return EXCHANGES.get(exchange, StockExchange.create_sse())
