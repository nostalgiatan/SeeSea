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
股票 API 桥接模块

为 Rust 提供同步 API 调用接口。
配置由 Rust 端传入。

设计原则:
- 模块级函数（无类实例化）
- 延迟导入（避免循环导入）
- 最小化全局状态
- 正确的资源管理
"""

from __future__ import annotations

import json
import logging
from typing import Optional, Dict, Any, TYPE_CHECKING

if TYPE_CHECKING:
    from .service import StockService

logger = logging.getLogger(__name__)

# 配置（由 Rust 传递）
_config: Dict[str, Any] = {}
# 服务实例（惰性初始化）
_service: Optional["StockService"] = None


def _json_serializer(obj: Any) -> Any:
    """JSON 序列化辅助函数"""
    if hasattr(obj, "to_dict"):
        return obj.to_dict()
    if hasattr(obj, "__dict__"):
        return {k: v for k, v in obj.__dict__.items() if not k.startswith("_")}
    if hasattr(obj, "isoformat"):
        return obj.isoformat()
    return str(obj)


def _to_json(data: Any) -> str:
    """安全地转换为 JSON 字符串"""
    try:
        return json.dumps(data, default=_json_serializer, ensure_ascii=False)
    except Exception as e:
        return json.dumps({"error": f"Serialization error: {e}"})


def _get_service() -> "StockService":
    """获取服务实例（惰性初始化）"""
    global _service
    if _service is None:
        from .service import StockService

        cache_path = _config.get("stock_cache_path")
        _service = StockService(cache_path=cache_path)
        logger.info("Stock service initialized with cache_path=%s", cache_path)
    return _service


# ==================== 配置管理 ====================


def configure(config_json: str) -> str:
    """从 Rust 接收配置"""
    global _config, _service
    try:
        _config = json.loads(config_json)
        _service = None  # 重置服务
        return _to_json({"success": True})
    except Exception as e:
        logger.error(f"Configure failed: {e}")
        return _to_json({"error": str(e)})


# ==================== 股票搜索 ====================


def stock_search(query: str, limit: int = 20) -> str:
    """搜索股票"""
    try:
        service = _get_service()
        results = service.search_stocks(query, limit)
        return _to_json(results)
    except Exception as e:
        logger.error(f"Search failed: {e}")
        return _to_json({"error": str(e), "results": []})


# ==================== 实时行情 ====================


def stock_quotes(codes: str) -> str:
    """获取实时行情"""
    try:
        service = _get_service()
        code_list = [c.strip() for c in codes.split(",") if c.strip()]
        if not code_list:
            quotes = service.get_all_quotes_sync()
        else:
            quotes = service.get_quotes_sync(code_list)
        return _to_json(quotes)
    except Exception as e:
        logger.error(f"Quotes failed: {e}")
        return _to_json({"error": str(e), "quotes": []})


# ==================== K线数据 ====================


def stock_klines(
    code: str,
    period: str = "daily",
    adjust: str = "qfq",
    start_date: Optional[str] = None,
    end_date: Optional[str] = None,
    limit: int = 500,
) -> str:
    """获取K线数据"""
    try:
        service = _get_service()
        klines = service.get_klines_sync(
            code, period, adjust, start_date=start_date, end_date=end_date, limit=limit
        )
        return _to_json(klines)
    except Exception as e:
        logger.error(f"Klines failed: {e}")
        return _to_json({"error": str(e), "klines": []})


# ==================== 股票详情 ====================


def stock_detail(code: str) -> str:
    """获取股票详情"""
    try:
        service = _get_service()
        detail = service.get_stock_detail_sync(code)
        return _to_json(detail)
    except Exception as e:
        logger.error(f"Detail failed: {e}")
        return _to_json({"error": str(e)})


# ==================== 财务数据 ====================


def stock_financial(code: str, report_type: str = "all") -> str:
    """获取财务数据"""
    try:
        service = _get_service()
        financial = service.get_financial_sync(code, report_type)
        return _to_json(financial)
    except Exception as e:
        logger.error(f"Financial failed: {e}")
        return _to_json({"error": str(e), "financial": []})


# ==================== 资金流向 ====================


def stock_fund_flow(code: str, period: str = "daily") -> str:
    """获取资金流向"""
    try:
        service = _get_service()
        fund_flow = service.get_fund_flow_sync(code, period)
        return _to_json(fund_flow)
    except Exception as e:
        logger.error(f"Fund flow failed: {e}")
        return _to_json({"error": str(e), "fund_flow": []})


# ==================== 股东信息 ====================


def stock_holders(code: str, holder_type: str = "top10") -> str:
    """获取股东信息"""
    try:
        service = _get_service()
        holders = service.get_holders_sync(code, holder_type)
        return _to_json(holders)
    except Exception as e:
        logger.error(f"Holders failed: {e}")
        return _to_json({"error": str(e), "holders": []})


# ==================== 公告信息 ====================


def stock_announcements(code: str, limit: int = 50) -> str:
    """获取公告信息"""
    try:
        service = _get_service()
        announcements = service.get_announcements_sync(code, limit)
        return _to_json(announcements)
    except Exception as e:
        logger.error(f"Announcements failed: {e}")
        return _to_json({"error": str(e), "announcements": []})


# ==================== 市场数据 ====================


def market_status(exchange: str = "sse") -> str:
    """获取市场状态"""
    try:
        service = _get_service()
        status = service.get_market_status_sync(exchange)
        return _to_json(status)
    except Exception as e:
        logger.error(f"Market status failed: {e}")
        return _to_json({"error": str(e)})


def market_indices() -> str:
    """获取指数行情"""
    try:
        service = _get_service()
        indices = service.get_index_quotes_sync()
        return _to_json(indices)
    except Exception as e:
        logger.error(f"Indices failed: {e}")
        return _to_json({"error": str(e), "indices": []})


def market_lhb(trade_date: Optional[str] = None) -> str:
    """获取龙虎榜数据"""
    try:
        service = _get_service()
        lhb = service.get_lhb_data_sync(trade_date)
        return _to_json(lhb)
    except Exception as e:
        logger.error(f"LHB failed: {e}")
        return _to_json({"error": str(e), "lhb": []})


# ==================== 板块数据 ====================


def sector_list(sector_type: str = "industry") -> str:
    """获取板块列表"""
    try:
        service = _get_service()
        sectors = service.get_sector_list_sync(sector_type)
        return _to_json(sectors)
    except Exception as e:
        logger.error(f"Sectors failed: {e}")
        return _to_json({"error": str(e), "sectors": []})


def sector_stocks(code: str) -> str:
    """获取板块成分股"""
    try:
        service = _get_service()
        stocks = service.get_sector_stocks_sync(code)
        return _to_json(stocks)
    except Exception as e:
        logger.error(f"Sector stocks failed: {e}")
        return _to_json({"error": str(e), "stocks": []})


# ==================== 资源管理 ====================


def cleanup() -> str:
    """清理资源"""
    global _service, _config
    try:
        if _service is not None:
            if hasattr(_service, "close"):
                _service.close()
            _service = None
        _config = {}
        return _to_json({"success": True})
    except Exception as e:
        logger.error(f"Cleanup failed: {e}")
        return _to_json({"error": str(e)})
