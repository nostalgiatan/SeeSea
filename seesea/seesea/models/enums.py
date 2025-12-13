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
股票相关枚举定义

提供股票市场、交易所、板块类型等枚举。
"""

from enum import Enum


class Market(str, Enum):
    """
    市场类型枚举
    """

    CN_A = "cn_a"  # A股市场
    CN_B = "cn_b"  # B股市场
    HK = "hk"  # 港股市场
    US = "us"  # 美股市场
    CN_FUND = "cn_fund"  # 中国基金
    CN_BOND = "cn_bond"  # 中国债券
    CN_INDEX = "cn_index"  # 中国指数


class Exchange(str, Enum):
    """
    交易所枚举
    """

    # 中国大陆
    SSE = "sse"  # 上海证券交易所
    SZSE = "szse"  # 深圳证券交易所
    BSE = "bse"  # 北京证券交易所

    # 港股
    HKEX = "hkex"  # 香港交易所

    # 美股
    NYSE = "nyse"  # 纽约证券交易所
    NASDAQ = "nasdaq"  # 纳斯达克
    AMEX = "amex"  # 美国证券交易所


class BoardType(str, Enum):
    """
    板块类型枚举
    """

    # 上交所
    MAIN_SSE = "main_sse"  # 沪市主板
    STAR = "star"  # 科创板 (688xxx)

    # 深交所
    MAIN_SZSE = "main_szse"  # 深市主板
    SME = "sme"  # 中小板 (已并入主板)
    GEM = "gem"  # 创业板 (300xxx)

    # 北交所
    BSE_MAIN = "bse_main"  # 北交所

    # B股
    B_SSE = "b_sse"  # 沪B
    B_SZSE = "b_szse"  # 深B


class StockStatus(str, Enum):
    """
    股票状态枚举
    """

    NORMAL = "normal"  # 正常交易
    SUSPENDED = "suspended"  # 停牌
    DELISTING = "delisting"  # 退市警告
    DELISTED = "delisted"  # 已退市
    IPO = "ipo"  # 新股
    ST = "st"  # ST
    ST_STAR = "st_star"  # *ST


class AdjustType(str, Enum):
    """
    复权类型枚举
    """

    NONE = "none"  # 不复权
    QFQ = "qfq"  # 前复权
    HFQ = "hfq"  # 后复权


class PeriodType(str, Enum):
    """
    K线周期枚举
    """

    # 分钟级别
    MIN_1 = "1"  # 1分钟
    MIN_5 = "5"  # 5分钟
    MIN_15 = "15"  # 15分钟
    MIN_30 = "30"  # 30分钟
    MIN_60 = "60"  # 60分钟

    # 日级别及以上
    DAILY = "daily"  # 日线
    WEEKLY = "weekly"  # 周线
    MONTHLY = "monthly"  # 月线
    QUARTERLY = "quarterly"  # 季线
    YEARLY = "yearly"  # 年线


class FundFlowDirection(str, Enum):
    """
    资金流向枚举
    """

    INFLOW = "inflow"  # 流入
    OUTFLOW = "outflow"  # 流出
    NET_INFLOW = "net_inflow"  # 净流入
    NET_OUTFLOW = "net_outflow"  # 净流出


class AnnouncementType(str, Enum):
    """
    公告类型枚举
    """

    GENERAL = "general"  # 一般公告
    FINANCIAL_REPORT = "financial_report"  # 财务报告
    DIVIDEND = "dividend"  # 分红派息
    STOCK_CHANGE = "stock_change"  # 股本变动
    MAJOR_EVENT = "major_event"  # 重大事项
    ACQUISITION = "acquisition"  # 收购兼并
    RELATED_PARTY = "related_party"  # 关联交易
    SHAREHOLDER = "shareholder"  # 股东变动
    EXECUTIVE = "executive"  # 高管变动
    BOND = "bond"  # 债券公告
    RISK_WARNING = "risk_warning"  # 风险提示
    CLARIFICATION = "clarification"  # 澄清公告


class IndustryClassification(str, Enum):
    """
    行业分类标准枚举
    """

    CSRC = "csrc"  # 证监会行业分类
    SW = "sw"  # 申万行业分类
    WIND = "wind"  # Wind行业分类
    THS = "ths"  # 同花顺行业分类
    EASTMONEY = "em"  # 东方财富行业分类


class UpdateFrequency(str, Enum):
    """
    数据更新频率枚举
    """

    REALTIME = "realtime"  # 实时 (5秒)
    MINUTE = "minute"  # 分钟级
    HOURLY = "hourly"  # 小时级
    HALF_DAY = "half_day"  # 半天
    DAILY = "daily"  # 每日
    WEEKLY = "weekly"  # 每周
    MONTHLY = "monthly"  # 每月
    QUARTERLY = "quarterly"  # 每季度
    YEARLY = "yearly"  # 每年
    STATIC = "static"  # 静态数据


class DataSource(str, Enum):
    """
    数据源枚举
    """

    AKSHARE = "akshare"  # AKShare
    CNINFO = "cninfo"  # 巨潮资讯
    EASTMONEY = "eastmoney"  # 东方财富
    SINA = "sina"  # 新浪财经
    THS = "ths"  # 同花顺
    SSE = "sse"  # 上交所官网
    SZSE = "szse"  # 深交所官网
    BSE = "bse"  # 北交所官网
