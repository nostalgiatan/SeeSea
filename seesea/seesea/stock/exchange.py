#!/usr/bin/env python
"""
交易所相关模型
"""

from datetime import datetime
from dataclasses import dataclass
from typing import List, Optional


@dataclass
class IndexQuote:
    """指数行情"""
    code: str
    name: str
    price: float
    change: float
    change_pct: float
    volume: int
    amount: float
    open: float
    high: float
    low: float
    prev_close: float
    time: datetime
    
    def to_dict(self) -> dict:
        """转换为字典"""
        return {
            "code": self.code,
            "name": self.name,
            "price": self.price,
            "change": self.change,
            "change_pct": self.change_pct,
            "volume": self.volume,
            "amount": self.amount,
            "open": self.open,
            "high": self.high,
            "low": self.low,
            "prev_close": self.prev_close,
            "time": self.time.isoformat() if self.time else None
        }


@dataclass
class MarketStatus:
    """市场状态"""
    market: str
    is_open: bool
    open_time: Optional[datetime]
    close_time: Optional[datetime]
    status: str  # open, close, break, pre_open, after_close
    description: str