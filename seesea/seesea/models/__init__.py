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
SeeSea Models - 数据模型模块

提供股票、交易所等核心数据类型定义。
"""

from .enums import (
    Market,
    Exchange,
    BoardType,
    StockStatus,
    AdjustType,
    PeriodType,
    FundFlowDirection,
    AnnouncementType,
    IndustryClassification,
    UpdateFrequency,
    DataSource,
)

from .stock import (
    Stock,
    StockQuote,
    StockKLine,
    StockFinancial,
    StockHolder,
    StockAnnouncement,
    StockFundFlow,
    StockIndustry,
    StockTechnicalIndicator,
)

from .exchange import (
    StockExchange,
    TradingSession,
    TradingTime,
    MarketStatus,
    IndexQuote,
    EXCHANGES,
    get_exchange,
)

__all__ = [
    # 枚举类型
    "Market",
    "Exchange",
    "BoardType",
    "StockStatus",
    "AdjustType",
    "PeriodType",
    "FundFlowDirection",
    "AnnouncementType",
    "IndustryClassification",
    "UpdateFrequency",
    "DataSource",
    # 股票数据类
    "Stock",
    "StockQuote",
    "StockKLine",
    "StockFinancial",
    "StockHolder",
    "StockAnnouncement",
    "StockFundFlow",
    "StockIndustry",
    "StockTechnicalIndicator",
    # 交易所
    "StockExchange",
    "TradingSession",
    "TradingTime",
    "MarketStatus",
    "IndexQuote",
    "EXCHANGES",
    "get_exchange",
]
