#!/usr/bin/env python
"""
股票数据模型模块

提供所有股票相关的数据类型定义，包括实时行情、K线数据、财务数据等。
"""

from datetime import datetime
from dataclasses import dataclass
from enum import Enum
from typing import List, Optional


class PeriodType(Enum):
    """周期类型"""
    DAILY = "daily"
    WEEKLY = "weekly"
    MONTHLY = "monthly"
    QUARTERLY = "quarterly"
    YEARLY = "yearly"


class AdjustType(Enum):
    """复权类型"""
    NONE = "none"  # 不复权
    FORWARD = "forward"  # 前复权 (QFQ)
    BACKWARD = "backward"  # 后复权 (HFQ)
    QFQ = "qfq"  # 前复权 (兼容旧代码)
    HFQ = "hfq"  # 后复权 (兼容旧代码)


class ReportType(Enum):
    """财报类型"""
    BALANCE_SHEET = "balance_sheet"
    INCOME_STATEMENT = "income_statement"
    CASH_FLOW_STATEMENT = "cash_flow_statement"


@dataclass(slots=True)
class Stock:
    """股票基本信息 - 优化内存使用"""
    code: str
    name: str
    market: str = ""
    industry: str = ""
    area: str = ""
    pe: float = 0.0
    pb: float = 0.0
    total_share: float = 0.0
    float_share: float = 0.0
    total_assets: float = 0.0
    liquid_assets: float = 0.0
    fixed_assets: float = 0.0
    reserved: float = 0.0
    reserved_per_share: float = 0.0
    eps: float = 0.0
    bvps: float = 0.0
    listing_date: Optional[datetime] = None
    
    def __post_init__(self):
        if self.listing_date is None:
            self.listing_date = datetime.now()
    
    def to_dict(self) -> dict:
        """转换为字典"""
        return {
            "code": self.code,
            "name": self.name,
            "market": self.market,
            "industry": self.industry,
            "area": self.area,
            "pe": self.pe,
            "pb": self.pb,
            "total_share": self.total_share,
            "float_share": self.float_share,
            "total_assets": self.total_assets,
            "liquid_assets": self.liquid_assets,
            "fixed_assets": self.fixed_assets,
            "reserved": self.reserved,
            "reserved_per_share": self.reserved_per_share,
            "eps": self.eps,
            "bvps": self.bvps,
            "listing_date": self.listing_date.isoformat() if self.listing_date else None
        }
    
    @classmethod
    def from_dict(cls, data: dict) -> 'Stock':
        """从字典创建实例"""
        return cls(
            code=data.get("code", ""),
            name=data.get("name", ""),
            market=data.get("market", ""),
            industry=data.get("industry", ""),
            area=data.get("area", ""),
            pe=data.get("pe", 0.0),
            pb=data.get("pb", 0.0),
            total_share=data.get("total_share", 0.0),
            float_share=data.get("float_share", 0.0),
            total_assets=data.get("total_assets", 0.0),
            liquid_assets=data.get("liquid_assets", 0.0),
            fixed_assets=data.get("fixed_assets", 0.0),
            reserved=data.get("reserved", 0.0),
            reserved_per_share=data.get("reserved_per_share", 0.0),
            eps=data.get("eps", 0.0),
            bvps=data.get("bvps", 0.0),
            listing_date=Stock._parse_datetime(data.get("listing_date"))
        )
    
    @staticmethod
    def _parse_datetime(value):
        """解析日期时间值，支持多种格式"""
        if value is None:
            return None
        
        # 如果已经是datetime对象，直接返回
        if isinstance(value, datetime):
            return value
        
        # 如果是Timestamp对象（来自pandas），转换为datetime
        try:
            from pandas import Timestamp
            if isinstance(value, Timestamp):
                return value.to_pydatetime()
        except ImportError:
            pass
        
        # 如果是字符串，使用fromisoformat解析
        if isinstance(value, str):
            try:
                return datetime.fromisoformat(value)
            except ValueError:
                return None
        
        # 其他情况返回None
        return None


@dataclass(slots=True)
class StockQuote:
    """股票实时行情 - 优化内存使用"""
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
    close: float
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
            "close": self.close,
            "prev_close": self.prev_close,
            "time": self.time.isoformat() if self.time else None
        }


@dataclass(slots=True)
class StockKLine:
    """K线数据 - 优化内存使用"""
    date: datetime
    open: float
    high: float
    low: float
    close: float
    volume: int
    amount: float
    change: float
    change_pct: float
    
    def to_dict(self) -> dict:
        """转换为字典"""
        return {
            "date": self.date.isoformat() if self.date else None,
            "open": self.open,
            "high": self.high,
            "low": self.low,
            "close": self.close,
            "volume": self.volume,
            "amount": self.amount,
            "change": self.change,
            "change_pct": self.change_pct
        }


@dataclass
class StockInfo:
    """股票信息"""
    code: str
    name: str
    total_market_cap: str
    circulation_market_cap: str
    industry: str
    pe_ratio: str
    pb_ratio: str
    dividend_rate: str
    
    def to_dict(self) -> dict:
        """转换为字典"""
        return {
            "code": self.code,
            "name": self.name,
            "total_market_cap": self.total_market_cap,
            "circulation_market_cap": self.circulation_market_cap,
            "industry": self.industry,
            "pe_ratio": self.pe_ratio,
            "pb_ratio": self.pb_ratio,
            "dividend_rate": self.dividend_rate
        }


@dataclass
class StockBasic:
    """股票基本信息（兼容AKShare格式）"""
    code: str
    name: str
    industry: str
    area: str
    pe: float
    pb: float
    total_share: float
    float_share: float
    total_assets: float
    liquid_assets: float
    fixed_assets: float
    reserved: float
    reserved_per_share: float
    eps: float
    bvps: float
    listing_date: Optional[datetime] = None


@dataclass
class StockFinancial:
    """财务数据"""
    code: str
    name: str
    report_date: datetime
    revenue: float
    profit: float
    net_profit: float
    total_assets: float
    total_liability: float
    net_assets: float
    gross_margin: float
    
    def to_dict(self) -> dict:
        """转换为字典"""
        return {
            "code": self.code,
            "name": self.name,
            "report_date": self.report_date.isoformat() if self.report_date else None,
            "revenue": self.revenue,
            "profit": self.profit,
            "net_profit": self.net_profit,
            "total_assets": self.total_assets,
            "total_liability": self.total_liability,
            "net_assets": self.net_assets,
            "gross_margin": self.gross_margin
        }


@dataclass
class StockAnnouncement:
    """公告信息"""
    code: str
    name: str
    title: str
    type: str
    date: datetime
    url: str
    
    def to_dict(self) -> dict:
        """转换为字典"""
        return {
            "code": self.code,
            "name": self.name,
            "title": self.title,
            "type": self.type,
            "date": self.date.isoformat() if self.date else None,
            "url": self.url
        }


@dataclass
class StockFundFlow:
    """资金流向"""
    code: str
    name: str
    date: datetime
    main_inflow: float
    main_outflow: float
    net_inflow: float
    small_inflow: float
    small_outflow: float
    net_small_inflow: float
    large_inflow: float
    large_outflow: float
    
    def to_dict(self) -> dict:
        """转换为字典"""
        return {
            "code": self.code,
            "name": self.name,
            "date": self.date.isoformat() if self.date else None,
            "main_inflow": self.main_inflow,
            "main_outflow": self.main_outflow,
            "net_inflow": self.net_inflow,
            "small_inflow": self.small_inflow,
            "small_outflow": self.small_outflow,
            "net_small_inflow": self.net_small_inflow,
            "large_inflow": self.large_inflow,
            "large_outflow": self.large_outflow
        }


@dataclass
class StockValuation:
    """股票估值分析"""
    code: str
    name: str
    date: datetime
    close_price: float
    pe_ratio: float
    pb_ratio: float
    ps_ratio: float
    pcf_ratio: float
    dividend_yield: float
    market_cap: float
    enterprise_value: float


@dataclass
class StockMargin:
    """融资融券数据"""
    code: str
    name: str
    date: datetime
    financing_balance: float
    financing_buy: float
    margin_balance: float
    margin_sell: float
    margin_ratio: float


@dataclass
class StockLHB:
    """龙虎榜数据"""
    code: str
    name: str
    date: datetime
    close_price: float
    change_pct: float
    turnover_rate: float
    amount: float
    net_buy: float
    buy_amount: float
    sell_amount: float
    buy_institutions: int
    sell_institutions: int


@dataclass
class StockBalanceSheet:
    """资产负债表"""
    code: str
    name: str
    report_date: datetime
    total_assets: float
    total_liabilities: float
    total_equity: float
    current_assets: float
    current_liabilities: float
    inventory: float
    accounts_receivable: float
    accounts_payable: float


@dataclass
class StockIncomeStatement:
    """利润表"""
    code: str
    name: str
    report_date: datetime
    total_revenue: float
    operating_revenue: float
    total_cost: float
    operating_cost: float
    operating_profit: float
    net_profit: float
    gross_profit: float


@dataclass
class StockCashFlow:
    """现金流量表"""
    code: str
    name: str
    report_date: datetime
    operating_cash_flow: float
    investing_cash_flow: float
    financing_cash_flow: float
    net_cash_flow: float
    cash_at_end: float


@dataclass
class StockShareholder:
    """股东户数"""
    code: str
    name: str
    end_date: datetime
    holder_num: int
    holder_num_change: int
    holder_num_ratio: float
    avg_hold_num: float
    avg_market_cap: float
    
    def to_dict(self) -> dict:
        """转换为字典"""
        return {
            "code": self.code,
            "name": self.name,
            "end_date": self.end_date.isoformat() if self.end_date else None,
            "holder_num": self.holder_num,
            "holder_num_change": self.holder_num_change,
            "holder_num_ratio": self.holder_num_ratio,
            "avg_hold_num": self.avg_hold_num,
            "avg_market_cap": self.avg_market_cap
        }


@dataclass
class StockDividend:
    """分红送配"""
    code: str
    name: str
    report_date: datetime
    dividend_per_share: float
    bonus_per_share: float
    transfer_per_share: float
    total_dividend: float
    dividend_date: datetime
    ex_dividend_date: datetime


@dataclass
class StockResearchReport:
    """研究报告"""
    code: str
    name: str
    title: str
    author: str
    institution: str
    publish_date: datetime
    rating: str
    target_price: float
    summary: str
    url: str


@dataclass
class StockPledge:
    """股权质押"""
    code: str
    name: str
    shareholder: str
    pledge_amount: float
    pledge_ratio: float
    pledge_date: datetime
    release_date: datetime
    pledgee: str
    purpose: str


@dataclass
class StockRestricted:
    """限售解禁"""
    code: str
    name: str
    release_date: datetime
    release_amount: float
    release_ratio: float
    release_market_cap: float
    holder: str
    holder_type: str


@dataclass
class StockResearchVisit:
    """机构调研"""
    code: str
    name: str
    visit_date: datetime
    institution: str
    participants: str
    visit_method: str
    visit_purpose: str
    main_content: str


@dataclass
class StockRepurchase:
    """股票回购"""
    code: str
    name: str
    repurchase_date: datetime
    repurchase_amount: float
    repurchase_price: float
    repurchase_ratio: float
    repurchase_purpose: str
    repurchase_method: str


@dataclass
class StockMarginList:
    """融资融券标的列表"""
    code: str
    name: str
    list_date: datetime
    margin_type: str
    status: str