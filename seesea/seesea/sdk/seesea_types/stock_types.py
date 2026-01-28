"""
股票相关类型定义
"""

from typing import Dict, List, Optional, Any, Literal
from dataclasses import dataclass
from datetime import datetime
from decimal import Decimal

ExchangeType = Literal["SSE", "SZSE", "NYSE", "NASDAQ", "HKEX", "TSE"]
StockType = Literal["A", "B", "H", "ADR", "GDR"]
MarketStatus = Literal["OPEN", "CLOSED", "PRE_MARKET", "AFTER_HOURS", "HOLIDAY"]

__all__ = [
    "ExchangeType",
    "StockType",
    "MarketStatus",
    "Exchange",
    "StockInfo",
    "StockPrice",
    "MarketData",
    "StockQuery",
    "StockSearchResult",
]


@dataclass
class Exchange:
    """交易所"""

    code: ExchangeType
    name: str
    timezone: str
    currency: str
    market_status: MarketStatus
    trading_hours: Dict[str, str]  # {"open": "09:30", "close": "15:00"}
    holidays: List[str]  # ["2024-01-01", "2024-02-10"]


@dataclass
class StockInfo:
    """股票信息"""

    symbol: str
    name: str
    exchange: ExchangeType
    stock_type: StockType
    industry: Optional[str] = None
    sector: Optional[str] = None
    market_cap: Optional[Decimal] = None
    shares_outstanding: Optional[int] = None
    currency: str = "CNY"
    listing_date: Optional[datetime] = None
    delisting_date: Optional[datetime] = None
    is_active: bool = True


@dataclass
class StockPrice:
    """股票价格"""

    symbol: str
    price: Decimal
    timestamp: datetime
    volume: int = 0
    turnover: Optional[Decimal] = None
    high: Optional[Decimal] = None
    low: Optional[Decimal] = None
    open: Optional[Decimal] = None
    close: Optional[Decimal] = None
    change: Optional[Decimal] = None
    change_percent: Optional[Decimal] = None


@dataclass
class MarketData:
    """市场数据"""

    symbol: str
    exchange: ExchangeType
    current_price: StockPrice
    daily_stats: Dict[str, Decimal]  # {"avg_price", "max_volume", etc.}
    technical_indicators: Dict[str, float]  # {"rsi", "macd", "ma5", etc.}
    fundamentals: Dict[str, Any]  # {"pe_ratio", "pb_ratio", etc.}
    last_updated: datetime


@dataclass
class StockQuery:
    """股票查询"""

    query: str
    exchange: Optional[ExchangeType] = None
    stock_type: Optional[StockType] = None
    industry: Optional[str] = None
    limit: int = 50
    include_delisted: bool = False


@dataclass
class StockSearchResult:
    """股票搜索结果"""

    stocks: List[StockInfo]
    total: int
    query: str
    took_ms: int
