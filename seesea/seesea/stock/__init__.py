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
SeeSea Stock - 股票数据模块

提供完整的股票数据获取、缓存和管理功能。

主要功能:
- 实时行情: 5秒更新的实时股票行情
- 历史K线: 日/周/月线历史数据
- 财务数据: 财务报表和指标
- 资金流向: 主力资金流入流出
- 公告信息: 上市公司公告
- 股东变动: 股东持仓变化
- 行业分类: 多种行业分类标准

架构设计:
- providers: 数据源适配器 (AKShare, 巨潮资讯等)
- cache: 基于 seesea_core 的多级缓存
- services: 业务逻辑服务层
- scheduler: 数据更新调度器

使用示例:
    >>> from seesea.stock import StockService
    >>>
    >>> # 创建股票服务
    >>> service = StockService()
    >>>
    >>> # 获取实时行情
    >>> quote = await service.get_quote("000001")
    >>> print(f"{quote.name}: {quote.price} ({quote.change_pct}%)")
    >>>
    >>> # 获取历史K线
    >>> klines = await service.get_klines("000001", period="daily", limit=100)
    >>>
    >>> # 获取财务数据
    >>> financial = await service.get_financial("000001")
    >>>
    >>> # 启动实时数据更新
    >>> await service.start_realtime_updates(["000001", "600519"])
    >>>
    >>> # 通过名称查找股票代码
    >>> code = service.get_code_by_name("平安银行")
    >>> # 或者使用全局映射
    >>> from seesea.stock import get_stock_mapping
    >>> mapping = get_stock_mapping()
    >>> results = mapping.search("平安")
"""

from .service import StockService, get_stock_service
from .providers import (
    BaseProvider,
    AKShareProvider,
    CNInfoProvider,
    StockNameMapping,
    get_stock_mapping,
)
from .cache import StockCacheManager
from .scheduler import StockScheduler

# api_bridge 模块由 Rust 直接导入，不在此导出

__all__ = [
    # 主服务
    "StockService",
    "get_stock_service",
    # 数据提供者
    "BaseProvider",
    "AKShareProvider",
    "CNInfoProvider",
    # 股票映射
    "StockNameMapping",
    "get_stock_mapping",
    # 缓存管理
    "StockCacheManager",
    # 调度器
    "StockScheduler",
]
