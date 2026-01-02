"""
API响应格式统一规范

定义所有API端点的标准响应结构，确保数据一致性。
"""

from typing import Dict, Any, List, Optional, Union
from dataclasses import dataclass, asdict
from datetime import datetime


@dataclass
class ApiResponse:
    """统一API响应格式"""
    success: bool
    data: Optional[Any] = None
    error: Optional[str] = None
    timestamp: Optional[str] = None
    count: Optional[int] = None
    message: Optional[str] = None
    
    def __post_init__(self):
        if self.timestamp is None:
            self.timestamp = datetime.now().isoformat()


class ResponseFormatter:
    """响应格式化器"""
    
    @staticmethod
    def success(data: Any = None, count: Optional[int] = None, message: Optional[str] = None) -> Dict[str, Any]:
        """成功响应"""
        response = ApiResponse(
            success=True,
            data=data,
            count=count,
            message=message
        )
        return asdict(response)
    
    @staticmethod
    def error(error_message: str, data: Any = None) -> Dict[str, Any]:
        """错误响应"""
        response = ApiResponse(
            success=False,
            error=error_message,
            data=data
        )
        return asdict(response)
    
    @staticmethod
    def format_stock_search(results: List[Dict[str, Any]]) -> Dict[str, Any]:
        """格式化股票搜索结果"""
        return ResponseFormatter.success(
            data={"results": results, "count": len(results)},
            count=len(results),
            message=f"找到 {len(results)} 只股票"
        )
    
    @staticmethod
    def format_stock_quotes(quotes: List[Dict[str, Any]]) -> Dict[str, Any]:
        """格式化股票行情"""
        return ResponseFormatter.success(
            data={"quotes": quotes},
            count=len(quotes),
            message=f"获取 {len(quotes)} 只股票行情"
        )
    
    @staticmethod
    def format_stock_detail(detail: Optional[Dict[str, Any]]) -> Dict[str, Any]:
        """格式化股票详情"""
        if detail:
            return ResponseFormatter.success(
                data={"detail": detail},
                message="获取股票详情成功"
            )
        else:
            return ResponseFormatter.error("未找到股票详情")
    
    @staticmethod
    def format_klines(klines: List[Dict[str, Any]]) -> Dict[str, Any]:
        """格式化K线数据"""
        return ResponseFormatter.success(
            data={"klines": klines},
            count=len(klines),
            message=f"获取 {len(klines)} 条K线数据"
        )
    
    @staticmethod
    def format_financial(financial: List[Dict[str, Any]]) -> Dict[str, Any]:
        """格式化财务数据"""
        return ResponseFormatter.success(
            data={"financial": financial},
            count=len(financial),
            message=f"获取 {len(financial)} 条财务数据"
        )
    
    @staticmethod
    def format_fund_flow(fund_flow: List[Dict[str, Any]]) -> Dict[str, Any]:
        """格式化资金流向"""
        return ResponseFormatter.success(
            data={"fund_flow": fund_flow},
            count=len(fund_flow),
            message=f"获取 {len(fund_flow)} 条资金流向数据"
        )
    
    @staticmethod
    def format_announcements(announcements: List[Dict[str, Any]]) -> Dict[str, Any]:
        """格式化公告信息"""
        return ResponseFormatter.success(
            data={"announcements": announcements},
            count=len(announcements),
            message=f"获取 {len(announcements)} 条公告"
        )
    
    @staticmethod
    def format_holders(holders: List[Dict[str, Any]]) -> Dict[str, Any]:
        """格式化股东信息"""
        return ResponseFormatter.success(
            data={"holders": holders},
            count=len(holders),
            message=f"获取 {len(holders)} 条股东信息"
        )
    
    @staticmethod
    def format_market_status(status: Optional[Dict[str, Any]]) -> Dict[str, Any]:
        """格式化市场状态"""
        if status:
            return ResponseFormatter.success(
                data={"status": status},
                message="获取市场状态成功"
            )
        else:
            return ResponseFormatter.error("未获取到市场状态")
    
    @staticmethod
    def format_market_indices(indices: List[Dict[str, Any]]) -> Dict[str, Any]:
        """格式化指数行情"""
        return ResponseFormatter.success(
            data={"indices": indices},
            count=len(indices),
            message=f"获取 {len(indices)} 个指数行情"
        )
    
    @staticmethod
    def format_sectors(sectors: List[Dict[str, Any]]) -> Dict[str, Any]:
        """格式化板块数据"""
        return ResponseFormatter.success(
            data={"sectors": sectors},
            count=len(sectors),
            message=f"获取 {len(sectors)} 个板块"
        )
    
    @staticmethod
    def format_sector_stocks(stocks: List[Dict[str, Any]]) -> Dict[str, Any]:
        """格式化板块成分股"""
        return ResponseFormatter.success(
            data={"stocks": stocks},
            count=len(stocks),
            message=f"获取 {len(stocks)} 只成分股"
        )

    @staticmethod
    def format_ranking_data(ranking_data: List[Dict[str, Any]], ranking_type: str) -> Dict[str, Any]:
        """格式化股票排行榜数据"""
        type_names = {
            "gainers": "涨幅榜",
            "losers": "跌幅榜", 
            "hot": "人气榜",
            "fund_flow": "资金流向榜"
        }
        type_name = type_names.get(ranking_type, "排行榜")
        
        return ResponseFormatter.success(
            data={"ranking": ranking_data, "type": ranking_type},
            count=len(ranking_data),
            message=f"获取 {type_name} {len(ranking_data)} 只股票"
        )


# 统一的字段映射规范
FIELD_MAPPINGS = {
    "stock_search": {
        "required_fields": ["code", "name", "market", "industry", "area", "pe", "pb", "listing_date"],
        "optional_fields": ["total_share", "float_share", "total_assets", "liquid_assets", "eps", "bvps"]
    },
    "stock_quote": {
        "required_fields": ["code", "name", "price", "change", "change_pct", "volume", "amount", "open", "high", "low", "close", "prev_close", "time"],
        "optional_fields": []
    },
    "stock_detail": {
        "required_fields": ["code", "name", "market", "industry", "area"],
        "optional_fields": ["pe", "pb", "total_market_cap", "circulation_market_cap", "listing_date"]
    },
    "kline": {
        "required_fields": ["date", "open", "high", "low", "close", "volume", "amount", "change", "change_pct"],
        "optional_fields": []
    }
}