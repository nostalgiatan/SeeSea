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
股票数据模型

提供股票相关的核心数据类定义，包括：
- Stock: 股票基础信息
- StockQuote: 实时行情
- StockKLine: K线数据
- StockFinancial: 财务数据
- StockHolder: 股东信息
- StockAnnouncement: 公告信息
- StockFundFlow: 资金流向
- StockIndustry: 行业信息
- StockTechnicalIndicator: 技术指标
"""

from dataclasses import dataclass, field
from datetime import datetime, date
from typing import Optional, List, Dict, Any

from .enums import (
    Market,
    Exchange,
    BoardType,
    StockStatus,
    AdjustType,
    PeriodType,
    AnnouncementType,
    IndustryClassification,
    DataSource,
)


@dataclass
class Stock:
    """
    股票基础信息

    包含股票的静态基础信息，如代码、名称、上市日期等。
    此类数据更新频率较低，适合持久化存储。

    更新频率: STATIC / DAILY
    """

    # 核心标识
    code: str  # 股票代码 (如 000001)
    name: str  # 股票名称 (如 平安银行)
    full_code: str  # 完整代码 (如 000001.SZ)

    # 市场信息
    market: Market = Market.CN_A  # 市场类型
    exchange: Exchange = Exchange.SZSE  # 交易所
    board: BoardType = BoardType.MAIN_SZSE  # 板块类型

    # 基础信息
    status: StockStatus = StockStatus.NORMAL  # 股票状态
    list_date: Optional[date] = None  # 上市日期
    delist_date: Optional[date] = None  # 退市日期 (如已退市)

    # 股本信息
    total_share: Optional[float] = None  # 总股本 (亿股)
    float_share: Optional[float] = None  # 流通股本 (亿股)

    # 行业分类
    industry_csrc: Optional[str] = None  # 证监会行业
    industry_sw_l1: Optional[str] = None  # 申万一级行业
    industry_sw_l2: Optional[str] = None  # 申万二级行业
    industry_sw_l3: Optional[str] = None  # 申万三级行业

    # 公司信息
    company_name: Optional[str] = None  # 公司全称
    company_name_en: Optional[str] = None  # 公司英文名
    legal_representative: Optional[str] = None  # 法定代表人
    secretary: Optional[str] = None  # 董秘
    registered_capital: Optional[float] = None  # 注册资本 (万元)
    province: Optional[str] = None  # 所在省份
    city: Optional[str] = None  # 所在城市
    website: Optional[str] = None  # 公司网站
    main_business: Optional[str] = None  # 主营业务

    # 元数据
    update_time: Optional[datetime] = None  # 更新时间
    data_source: DataSource = DataSource.AKSHARE  # 数据来源

    def __post_init__(self):
        """初始化后处理"""
        if not self.full_code:
            self.full_code = self._build_full_code()
        if not self.update_time:
            self.update_time = datetime.now()

    def _build_full_code(self) -> str:
        """构建完整股票代码"""
        suffix_map = {
            Exchange.SSE: ".SH",
            Exchange.SZSE: ".SZ",
            Exchange.BSE: ".BJ",
            Exchange.HKEX: ".HK",
            Exchange.NYSE: ".N",
            Exchange.NASDAQ: ".O",
        }
        suffix = suffix_map.get(self.exchange, "")
        return f"{self.code}{suffix}"

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Stock":
        """从字典创建Stock对象"""
        return cls(
            code=data.get("code", ""),
            name=data.get("name", ""),
            full_code=data.get("full_code", ""),
            market=Market(data.get("market", "cn_a")),
            exchange=Exchange(data.get("exchange", "szse")),
            board=BoardType(data.get("board", "main_szse")),
            status=StockStatus(data.get("status", "normal")),
            list_date=data.get("list_date"),
            delist_date=data.get("delist_date"),
            total_share=data.get("total_share"),
            float_share=data.get("float_share"),
            industry_csrc=data.get("industry_csrc"),
            industry_sw_l1=data.get("industry_sw_l1"),
            industry_sw_l2=data.get("industry_sw_l2"),
            industry_sw_l3=data.get("industry_sw_l3"),
            company_name=data.get("company_name"),
            company_name_en=data.get("company_name_en"),
            legal_representative=data.get("legal_representative"),
            secretary=data.get("secretary"),
            registered_capital=data.get("registered_capital"),
            province=data.get("province"),
            city=data.get("city"),
            website=data.get("website"),
            main_business=data.get("main_business"),
            update_time=data.get("update_time"),
        )

    def to_dict(self) -> Dict[str, Any]:
        """转换为字典"""
        return {
            "code": self.code,
            "name": self.name,
            "full_code": self.full_code,
            "market": self.market.value,
            "exchange": self.exchange.value,
            "board": self.board.value,
            "status": self.status.value,
            "list_date": self.list_date.isoformat() if self.list_date else None,
            "delist_date": self.delist_date.isoformat() if self.delist_date else None,
            "total_share": self.total_share,
            "float_share": self.float_share,
            "industry_csrc": self.industry_csrc,
            "industry_sw_l1": self.industry_sw_l1,
            "industry_sw_l2": self.industry_sw_l2,
            "industry_sw_l3": self.industry_sw_l3,
            "company_name": self.company_name,
            "company_name_en": self.company_name_en,
            "legal_representative": self.legal_representative,
            "secretary": self.secretary,
            "registered_capital": self.registered_capital,
            "province": self.province,
            "city": self.city,
            "website": self.website,
            "main_business": self.main_business,
            "update_time": self.update_time.isoformat() if self.update_time else None,
            "data_source": self.data_source.value,
        }


@dataclass
class StockQuote:
    """
    股票实时行情

    包含股票的实时交易数据，如价格、成交量等。
    此类数据更新频率高，适合实时缓存。

    更新频率: REALTIME (5秒)
    """

    # 核心标识
    code: str  # 股票代码
    name: str  # 股票名称

    # 价格数据
    price: float  # 最新价
    open: float  # 今开
    high: float  # 最高
    low: float  # 最低
    prev_close: float  # 昨收

    # 涨跌数据
    change: float  # 涨跌额
    change_pct: float  # 涨跌幅 (%)

    # 成交数据
    volume: int  # 成交量 (手)
    turnover: float  # 成交额 (元)
    turnover_rate: Optional[float] = None  # 换手率 (%)

    # 盘口数据
    amplitude: Optional[float] = None  # 振幅 (%)
    volume_ratio: Optional[float] = None  # 量比

    # 估值数据
    pe_ttm: Optional[float] = None  # 市盈率TTM
    pe_dynamic: Optional[float] = None  # 市盈率(动态)
    pb: Optional[float] = None  # 市净率

    # 市值数据
    total_mv: Optional[float] = None  # 总市值 (元)
    float_mv: Optional[float] = None  # 流通市值 (元)

    # 时间数据
    timestamp: Optional[datetime] = None  # 行情时间戳
    update_time: Optional[datetime] = None  # 更新时间

    # 元数据
    data_source: DataSource = DataSource.AKSHARE

    def __post_init__(self):
        if not self.update_time:
            self.update_time = datetime.now()

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "StockQuote":
        """从字典创建StockQuote对象"""
        return cls(
            code=data.get("code", ""),
            name=data.get("name", ""),
            price=float(data.get("price", 0)),
            open=float(data.get("open", 0)),
            high=float(data.get("high", 0)),
            low=float(data.get("low", 0)),
            prev_close=float(data.get("prev_close", 0)),
            change=float(data.get("change", 0)),
            change_pct=float(data.get("change_pct", 0)),
            volume=int(data.get("volume", 0)),
            turnover=float(data.get("turnover", 0)),
            turnover_rate=data.get("turnover_rate"),
            amplitude=data.get("amplitude"),
            volume_ratio=data.get("volume_ratio"),
            pe_ttm=data.get("pe_ttm"),
            pe_dynamic=data.get("pe_dynamic"),
            pb=data.get("pb"),
            total_mv=data.get("total_mv"),
            float_mv=data.get("float_mv"),
            timestamp=data.get("timestamp"),
            update_time=data.get("update_time"),
        )

    def to_dict(self) -> Dict[str, Any]:
        """转换为字典"""
        return {
            "code": self.code,
            "name": self.name,
            "price": self.price,
            "open": self.open,
            "high": self.high,
            "low": self.low,
            "prev_close": self.prev_close,
            "change": self.change,
            "change_pct": self.change_pct,
            "volume": self.volume,
            "turnover": self.turnover,
            "turnover_rate": self.turnover_rate,
            "amplitude": self.amplitude,
            "volume_ratio": self.volume_ratio,
            "pe_ttm": self.pe_ttm,
            "pe_dynamic": self.pe_dynamic,
            "pb": self.pb,
            "total_mv": self.total_mv,
            "float_mv": self.float_mv,
            "timestamp": self.timestamp.isoformat() if self.timestamp else None,
            "update_time": self.update_time.isoformat() if self.update_time else None,
            "data_source": self.data_source.value,
        }


@dataclass
class StockKLine:
    """
    股票K线数据

    包含历史K线数据，支持多种周期。
    日线及以上周期每日更新，分钟级别实时更新。

    更新频率: DAILY (日线) / REALTIME (分钟线)
    """

    code: str  # 股票代码
    date: date  # 交易日期
    period: PeriodType = PeriodType.DAILY  # K线周期
    adjust: AdjustType = AdjustType.QFQ  # 复权类型

    # OHLCV数据
    open: float = 0.0  # 开盘价
    high: float = 0.0  # 最高价
    low: float = 0.0  # 最低价
    close: float = 0.0  # 收盘价
    volume: int = 0  # 成交量 (手)
    turnover: float = 0.0  # 成交额 (元)

    # 额外数据
    change_pct: Optional[float] = None  # 涨跌幅 (%)
    change: Optional[float] = None  # 涨跌额
    amplitude: Optional[float] = None  # 振幅 (%)
    turnover_rate: Optional[float] = None  # 换手率 (%)

    # 元数据
    update_time: Optional[datetime] = None
    data_source: DataSource = DataSource.AKSHARE

    def to_dict(self) -> Dict[str, Any]:
        return {
            "code": self.code,
            "date": self.date.isoformat(),
            "period": self.period.value,
            "adjust": self.adjust.value,
            "open": self.open,
            "high": self.high,
            "low": self.low,
            "close": self.close,
            "volume": self.volume,
            "turnover": self.turnover,
            "change_pct": self.change_pct,
            "change": self.change,
            "amplitude": self.amplitude,
            "turnover_rate": self.turnover_rate,
        }


@dataclass
class StockFinancial:
    """
    股票财务数据

    包含主要财务指标和报表数据。
    季度报表发布后更新。

    更新频率: QUARTERLY
    """

    code: str  # 股票代码
    report_date: date  # 报告期
    publish_date: Optional[date] = None  # 发布日期

    # 盈利能力
    eps: Optional[float] = None  # 每股收益 (元)
    eps_diluted: Optional[float] = None  # 稀释每股收益 (元)
    roe: Optional[float] = None  # 净资产收益率 (%)
    roa: Optional[float] = None  # 总资产收益率 (%)
    gross_margin: Optional[float] = None  # 毛利率 (%)
    net_margin: Optional[float] = None  # 净利率 (%)

    # 每股指标
    bps: Optional[float] = None  # 每股净资产 (元)
    cash_per_share: Optional[float] = None  # 每股现金流 (元)
    revenue_per_share: Optional[float] = None  # 每股营业收入 (元)

    # 利润表
    revenue: Optional[float] = None  # 营业收入 (元)
    revenue_yoy: Optional[float] = None  # 营收同比增长 (%)
    net_profit: Optional[float] = None  # 净利润 (元)
    net_profit_yoy: Optional[float] = None  # 净利润同比增长 (%)
    net_profit_deducted: Optional[float] = None  # 扣非净利润 (元)

    # 资产负债
    total_assets: Optional[float] = None  # 总资产 (元)
    total_liabilities: Optional[float] = None  # 总负债 (元)
    total_equity: Optional[float] = None  # 股东权益 (元)
    debt_ratio: Optional[float] = None  # 资产负债率 (%)

    # 现金流
    operating_cash_flow: Optional[float] = None  # 经营现金流 (元)
    investing_cash_flow: Optional[float] = None  # 投资现金流 (元)
    financing_cash_flow: Optional[float] = None  # 筹资现金流 (元)

    # 元数据
    update_time: Optional[datetime] = None
    data_source: DataSource = DataSource.AKSHARE

    def to_dict(self) -> Dict[str, Any]:
        return {
            "code": self.code,
            "report_date": self.report_date.isoformat(),
            "publish_date": (
                self.publish_date.isoformat() if self.publish_date else None
            ),
            "eps": self.eps,
            "eps_diluted": self.eps_diluted,
            "roe": self.roe,
            "roa": self.roa,
            "gross_margin": self.gross_margin,
            "net_margin": self.net_margin,
            "bps": self.bps,
            "revenue": self.revenue,
            "revenue_yoy": self.revenue_yoy,
            "net_profit": self.net_profit,
            "net_profit_yoy": self.net_profit_yoy,
            "debt_ratio": self.debt_ratio,
        }


@dataclass
class StockHolder:
    """
    股东信息

    包含十大股东、十大流通股东等信息。
    季度报表发布后更新。

    更新频率: QUARTERLY
    """

    code: str  # 股票代码
    report_date: date  # 报告期

    # 股东信息
    holder_name: str = ""  # 股东名称
    holder_type: str = ""  # 股东类型 (机构/个人)
    hold_num: Optional[float] = None  # 持股数量 (股)
    hold_ratio: Optional[float] = None  # 持股比例 (%)
    hold_change: Optional[float] = None  # 持股变动 (股)
    change_ratio: Optional[float] = None  # 变动比例 (%)
    rank: Optional[int] = None  # 股东排名

    # 股东类型标记
    is_top10: bool = False  # 是否十大股东
    is_top10_float: bool = False  # 是否十大流通股东
    is_institution: bool = False  # 是否机构
    is_fund: bool = False  # 是否基金

    # 元数据
    update_time: Optional[datetime] = None
    data_source: DataSource = DataSource.AKSHARE

    def to_dict(self) -> Dict[str, Any]:
        return {
            "code": self.code,
            "report_date": self.report_date.isoformat(),
            "holder_name": self.holder_name,
            "holder_type": self.holder_type,
            "hold_num": self.hold_num,
            "hold_ratio": self.hold_ratio,
            "hold_change": self.hold_change,
            "change_ratio": self.change_ratio,
            "rank": self.rank,
            "is_top10": self.is_top10,
            "is_top10_float": self.is_top10_float,
            "is_institution": self.is_institution,
            "is_fund": self.is_fund,
        }


@dataclass
class StockAnnouncement:
    """
    股票公告

    包含上市公司公告信息。
    实时更新。

    更新频率: REALTIME / HALF_DAY
    """

    code: str  # 股票代码
    title: str  # 公告标题
    url: str  # 公告链接
    publish_time: datetime  # 发布时间

    # 公告信息
    announcement_type: AnnouncementType = AnnouncementType.GENERAL  # 公告类型
    summary: Optional[str] = None  # 公告摘要
    content: Optional[str] = None  # 公告内容 (可选)

    # 关联信息
    related_codes: List[str] = field(default_factory=list)  # 关联股票代码

    # 元数据
    update_time: Optional[datetime] = None
    data_source: DataSource = DataSource.CNINFO

    def to_dict(self) -> Dict[str, Any]:
        return {
            "code": self.code,
            "title": self.title,
            "url": self.url,
            "publish_time": self.publish_time.isoformat(),
            "announcement_type": self.announcement_type.value,
            "summary": self.summary,
            "related_codes": self.related_codes,
        }


@dataclass
class StockFundFlow:
    """
    资金流向数据

    包含主力、散户、北向资金等流向数据。
    交易时段实时更新。

    更新频率: REALTIME (5秒) / DAILY
    """

    code: str  # 股票代码
    date: date  # 交易日期

    # 主力资金 (大单)
    main_net_inflow: Optional[float] = None  # 主力净流入 (元)
    main_net_inflow_pct: Optional[float] = None  # 主力净流入占比 (%)

    # 超大单
    super_large_inflow: Optional[float] = None  # 超大单流入 (元)
    super_large_outflow: Optional[float] = None  # 超大单流出 (元)
    super_large_net: Optional[float] = None  # 超大单净流入 (元)

    # 大单
    large_inflow: Optional[float] = None  # 大单流入 (元)
    large_outflow: Optional[float] = None  # 大单流出 (元)
    large_net: Optional[float] = None  # 大单净流入 (元)

    # 中单
    medium_inflow: Optional[float] = None  # 中单流入 (元)
    medium_outflow: Optional[float] = None  # 中单流出 (元)
    medium_net: Optional[float] = None  # 中单净流入 (元)

    # 小单
    small_inflow: Optional[float] = None  # 小单流入 (元)
    small_outflow: Optional[float] = None  # 小单流出 (元)
    small_net: Optional[float] = None  # 小单净流入 (元)

    # 元数据
    update_time: Optional[datetime] = None
    data_source: DataSource = DataSource.AKSHARE

    def to_dict(self) -> Dict[str, Any]:
        return {
            "code": self.code,
            "date": self.date.isoformat(),
            "main_net_inflow": self.main_net_inflow,
            "main_net_inflow_pct": self.main_net_inflow_pct,
            "super_large_net": self.super_large_net,
            "large_net": self.large_net,
            "medium_net": self.medium_net,
            "small_net": self.small_net,
        }


@dataclass
class StockIndustry:
    """
    行业分类信息

    包含股票的行业归属信息。
    行业调整时更新。

    更新频率: MONTHLY / 变动时
    """

    code: str  # 股票代码
    classification: IndustryClassification  # 分类标准

    # 行业层级
    industry_code: str = ""  # 行业代码
    industry_name: str = ""  # 行业名称
    level: int = 1  # 行业层级 (1/2/3)

    # 上级行业
    parent_code: Optional[str] = None  # 上级行业代码
    parent_name: Optional[str] = None  # 上级行业名称

    # 元数据
    effective_date: Optional[date] = None  # 生效日期
    update_time: Optional[datetime] = None
    data_source: DataSource = DataSource.AKSHARE

    def to_dict(self) -> Dict[str, Any]:
        return {
            "code": self.code,
            "classification": self.classification.value,
            "industry_code": self.industry_code,
            "industry_name": self.industry_name,
            "level": self.level,
            "parent_code": self.parent_code,
            "parent_name": self.parent_name,
            "effective_date": (
                self.effective_date.isoformat() if self.effective_date else None
            ),
        }


@dataclass
class StockTechnicalIndicator:
    """
    技术指标数据

    包含计算得出的技术分析指标。
    随K线数据更新而计算。

    更新频率: 跟随K线周期
    """

    code: str  # 股票代码
    date: date  # 日期
    period: PeriodType = PeriodType.DAILY  # K线周期

    # 均线指标
    ma5: Optional[float] = None  # 5日均线
    ma10: Optional[float] = None  # 10日均线
    ma20: Optional[float] = None  # 20日均线
    ma30: Optional[float] = None  # 30日均线
    ma60: Optional[float] = None  # 60日均线
    ma120: Optional[float] = None  # 120日均线
    ma250: Optional[float] = None  # 250日均线

    # MACD指标
    macd_dif: Optional[float] = None  # DIF
    macd_dea: Optional[float] = None  # DEA
    macd_hist: Optional[float] = None  # MACD柱

    # KDJ指标
    kdj_k: Optional[float] = None  # K值
    kdj_d: Optional[float] = None  # D值
    kdj_j: Optional[float] = None  # J值

    # RSI指标
    rsi_6: Optional[float] = None  # RSI(6)
    rsi_12: Optional[float] = None  # RSI(12)
    rsi_24: Optional[float] = None  # RSI(24)

    # 布林带
    boll_upper: Optional[float] = None  # 上轨
    boll_middle: Optional[float] = None  # 中轨
    boll_lower: Optional[float] = None  # 下轨

    # 其他指标
    cci: Optional[float] = None  # CCI
    atr: Optional[float] = None  # ATR
    obv: Optional[float] = None  # OBV

    # 元数据
    update_time: Optional[datetime] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "code": self.code,
            "date": self.date.isoformat(),
            "period": self.period.value,
            "ma5": self.ma5,
            "ma10": self.ma10,
            "ma20": self.ma20,
            "ma60": self.ma60,
            "macd_dif": self.macd_dif,
            "macd_dea": self.macd_dea,
            "macd_hist": self.macd_hist,
            "kdj_k": self.kdj_k,
            "kdj_d": self.kdj_d,
            "kdj_j": self.kdj_j,
            "rsi_6": self.rsi_6,
            "rsi_12": self.rsi_12,
            "boll_upper": self.boll_upper,
            "boll_middle": self.boll_middle,
            "boll_lower": self.boll_lower,
        }
