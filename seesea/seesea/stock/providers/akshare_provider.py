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
AKShare 数据提供者

基于 AKShare 库实现的数据提供者，提供 A股实时行情、历史数据、财务数据等。

AKShare 主要接口：
- stock_zh_a_spot_em: 沪深京A股实时行情
- stock_zh_a_hist: 历史K线数据
- stock_zh_index_spot_em: 指数实时行情
- stock_financial_abstract: 财务摘要
- stock_fund_flow_individual: 个股资金流
- stock_lhb_detail_daily_sina: 龙虎榜
- stock_board_industry_name_em: 行业板块
- stock_board_concept_name_em: 概念板块
"""

import asyncio
from datetime import date, datetime, timedelta
from typing import Optional, List, Dict, Any, cast
import logging

from .base import BaseProvider
from seesea.models import (
    Stock,
    StockQuote,
    StockKLine,
    StockFinancial,
    StockHolder,
    StockAnnouncement,
    StockFundFlow,
    StockIndustry,
    Market,
    Exchange,
    BoardType,
    PeriodType,
    AdjustType,
    DataSource,
    IndustryClassification,
)
from seesea.models.exchange import IndexQuote, MarketStatus

logger = logging.getLogger(__name__)

# 尝试导入 akshare
try:
    import akshare as ak
    import pandas as pd

    AKSHARE_AVAILABLE = True
except ImportError:
    AKSHARE_AVAILABLE = False
    logger.warning("akshare 未安装，请运行: pip install akshare")


class StockNameMapping:
    """
    股票名称到代码的映射管理器

    提供按名称/拼音/简称查找股票代码的功能，支持：
    - 精确名称匹配
    - 股票代码匹配
    - 拼音首字母匹配（完整拼音支持）
    - 拼音全拼匹配
    - 模糊搜索
    """

    def __init__(self):
        """初始化映射管理器"""

        self._name_to_code: Dict[str, str] = {}  # 名称 -> 代码
        self._pinyin_to_codes: Dict[str, List[str]] = {}  # 拼音首字母 -> 代码列表
        self._full_pinyin_to_codes: Dict[str, List[str]] = {}  # 全拼 -> 代码列表
        self._code_to_name: Dict[str, str] = {}  # 代码 -> 名称
        self._code_to_pinyin: Dict[str, str] = {}  # 代码 -> 拼音首字母
        self._code_to_full_pinyin: Dict[str, str] = {}  # 代码 -> 全拼
        self._initialized = False
        self._init_time: Optional[datetime] = None
        self._cache_ttl = timedelta(hours=4)  # 映射缓存4小时
        self._pinyin_module_loaded = True

    def is_stale(self) -> bool:
        """检查缓存是否过期"""
        if not self._initialized or self._init_time is None:
            return True
        # 明确告诉 mypy 此处 _init_time 已不为 None
        assert self._init_time is not None
        # 强制转换为 bool，以防 mypy 将比较表达式视为 Any
        return bool((datetime.now() - self._init_time) > self._cache_ttl)

    def clear(self):
        """清除缓存"""
        self._name_to_code.clear()
        self._pinyin_to_codes.clear()
        self._full_pinyin_to_codes.clear()
        self._code_to_name.clear()
        self._code_to_pinyin.clear()
        self._code_to_full_pinyin.clear()
        self._initialized = False
        self._init_time = None

    def build_from_stocks(self, stocks: List[Stock]):
        """从股票列表构建映射"""
        self.clear()

        for stock in stocks:
            # 名称 -> 代码
            self._name_to_code[stock.name] = stock.code
            self._code_to_name[stock.code] = stock.name

            # 生成拼音首字母
            pinyin_initials = self._get_pinyin_initials(stock.name)
            if pinyin_initials:
                self._code_to_pinyin[stock.code] = pinyin_initials
                if pinyin_initials not in self._pinyin_to_codes:
                    self._pinyin_to_codes[pinyin_initials] = []
                self._pinyin_to_codes[pinyin_initials].append(stock.code)

            # 生成全拼
            full_pinyin = self._get_full_pinyin(stock.name)
            if full_pinyin:
                self._code_to_full_pinyin[stock.code] = full_pinyin
                if full_pinyin not in self._full_pinyin_to_codes:
                    self._full_pinyin_to_codes[full_pinyin] = []
                self._full_pinyin_to_codes[full_pinyin].append(stock.code)

        self._initialized = True
        self._init_time = datetime.now()
        logger.info(
            f"股票映射构建完成: {len(self._name_to_code)} 只股票, "
            f"{len(self._pinyin_to_codes)} 个拼音索引"
        )

    def _get_pinyin_initials(self, name: str) -> str:
        """
        获取名称的拼音首字母

        Args:
            name: 股票名称

        Returns:
            拼音首字母（大写），如 "平安银行" -> "PAYH"
        """
        from pypinyin import lazy_pinyin, Style

        try:
            # 获取每个字的拼音首字母
            initials = lazy_pinyin(name, style=Style.FIRST_LETTER)
            return "".join(initials).upper()
        except Exception as e:
            logger.warning(f"获取拼音首字母失败 {name}: {e}")
            return ""

    def _get_full_pinyin(self, name: str) -> str:
        """
        获取名称的全拼

        Args:
            name: 股票名称

        Returns:
            全拼（小写无空格），如 "平安银行" -> "pinganyinhang"
        """
        from pypinyin import lazy_pinyin, Style

        try:
            pinyin_list = lazy_pinyin(name, style=Style.NORMAL)
            return "".join(pinyin_list).lower()
        except Exception as e:
            logger.warning(f"获取全拼失败 {name}: {e}")
            return ""

    def get_code_by_name(self, name: str) -> Optional[str]:
        """
        根据股票名称获取代码

        Args:
            name: 股票名称

        Returns:
            股票代码，如果未找到返回 None
        """
        return cast(Optional[str], self._name_to_code.get(name))

    def get_name_by_code(self, code: str) -> Optional[str]:
        """
        根据股票代码获取名称

        Args:
            code: 股票代码

        Returns:
            股票名称，如果未找到返回 None
        """
        return cast(Optional[str], self._code_to_name.get(code))

    def get_pinyin_by_code(self, code: str) -> Optional[str]:
        """根据代码获取拼音首字母"""
        return self._code_to_pinyin.get(code)

    def get_full_pinyin_by_code(self, code: str) -> Optional[str]:
        """根据代码获取全拼"""
        return self._code_to_full_pinyin.get(code)

    def get_codes_by_pinyin(self, pinyin: str) -> List[str]:
        """根据拼音首字母获取代码列表"""
        return self._pinyin_to_codes.get(pinyin.upper(), [])

    def get_codes_by_full_pinyin(self, pinyin: str) -> List[str]:
        """根据全拼获取代码列表"""
        return self._full_pinyin_to_codes.get(pinyin.lower(), [])

    def search(self, query: str, limit: int = 20) -> List[Dict[str, Any]]:
        """
        智能搜索股票

        支持:
        - 完整名称匹配（最高优先）
        - 股票代码精确匹配
        - 拼音首字母精确匹配
        - 全拼精确匹配
        - 拼音首字母前缀匹配
        - 全拼前缀匹配
        - 名称模糊匹配
        - 代码前缀匹配

        Args:
            query: 搜索关键词
            limit: 最大返回数量

        Returns:
            搜索结果列表，每项包含:
            - code: 股票代码
            - name: 股票名称
            - pinyin: 拼音首字母
            - match_type: 匹配类型
            - score: 匹配分数（越高越匹配）
        """
        results = []
        query_lower = query.lower()
        query_upper = query.upper()
        seen_codes = set()

        def add_result(code: str, match_type: str, score: int):
            """添加结果（去重）"""
            if code in seen_codes:
                return
            seen_codes.add(code)
            results.append(
                {
                    "code": code,
                    "name": self._code_to_name.get(code, ""),
                    "pinyin": self._code_to_pinyin.get(code, ""),
                    "full_pinyin": self._code_to_full_pinyin.get(code, ""),
                    "match_type": match_type,
                    "score": score,
                }
            )

        # 1. 精确代码匹配（最高分 100）
        if query in self._code_to_name:
            add_result(query, "code_exact", 100)

        # 2. 精确名称匹配（分数 95）
        if query in self._name_to_code:
            add_result(self._name_to_code[query], "name_exact", 95)

        # 3. 拼音首字母精确匹配（分数 90）
        pinyin_codes = self._pinyin_to_codes.get(query_upper, [])
        for code in pinyin_codes:
            add_result(code, "pinyin_exact", 90)

        # 4. 全拼精确匹配（分数 85）
        full_pinyin_codes = self._full_pinyin_to_codes.get(query_lower, [])
        for code in full_pinyin_codes:
            add_result(code, "full_pinyin_exact", 85)

        # 5. 拼音首字母前缀匹配（分数 70-80）
        if len(results) < limit:
            for pinyin, codes in self._pinyin_to_codes.items():
                if pinyin.startswith(query_upper):
                    score = 80 - (len(pinyin) - len(query_upper))
                    for code in codes:
                        add_result(code, "pinyin_prefix", max(70, score))
                        if len(results) >= limit:
                            break
                if len(results) >= limit:
                    break

        # 6. 全拼前缀匹配（分数 60-70）
        if len(results) < limit:
            for pinyin, codes in self._full_pinyin_to_codes.items():
                if pinyin.startswith(query_lower):
                    score = 70 - min(10, len(pinyin) - len(query_lower))
                    for code in codes:
                        add_result(code, "full_pinyin_prefix", max(60, score))
                        if len(results) >= limit:
                            break
                if len(results) >= limit:
                    break

        # 7. 名称包含匹配（分数 50）
        if len(results) < limit:
            for name, code in self._name_to_code.items():
                if query in name:
                    add_result(code, "name_contains", 50)
                    if len(results) >= limit:
                        break

        # 8. 代码前缀匹配（分数 40）
        if len(results) < limit:
            for code in self._code_to_name.keys():
                if code.startswith(query):
                    add_result(code, "code_prefix", 40)
                    if len(results) >= limit:
                        break

        # 按分数排序
        # 将可能为任意类型的 score 显式转换为 float，确保可比较（mypy 对 dict 值类型严格）
        from typing import Any, Dict

        def _score_key(item: Dict[str, Any]) -> float:
            v = item.get("score", 0)
            try:
                return float(v)  # type: ignore[arg-type]
            except (TypeError, ValueError):
                try:
                    return float(str(v))
                except Exception:
                    return 0.0

        results.sort(key=_score_key, reverse=True)

        return results[:limit]

    @property
    def name_to_code(self) -> Dict[str, str]:
        """获取名称到代码的映射字典"""
        return self._name_to_code.copy()

    @property
    def code_to_name(self) -> Dict[str, str]:
        """获取代码到名称的映射字典"""
        return self._code_to_name.copy()

    @property
    def pinyin_to_codes(self) -> Dict[str, List[str]]:
        """获取拼音首字母到代码的映射字典"""
        return {k: v.copy() for k, v in self._pinyin_to_codes.items()}

    def __len__(self) -> int:
        return len(self._name_to_code)

    def to_dict(self) -> Dict[str, Any]:
        """导出为字典格式"""
        return {
            "name_to_code": self._name_to_code,
            "code_to_name": self._code_to_name,
            "code_to_pinyin": self._code_to_pinyin,
            "stock_count": len(self._name_to_code),
            "pinyin_index_count": len(self._pinyin_to_codes),
            "initialized": self._initialized,
            "init_time": self._init_time.isoformat() if self._init_time else None,
        }


# 全局股票映射实例
_stock_mapping = StockNameMapping()


def get_stock_mapping() -> StockNameMapping:
    """获取全局股票映射实例"""
    return _stock_mapping


class AKShareProvider(BaseProvider):
    """
    AKShare 数据提供者

    使用 AKShare 获取 A股市场数据。
    """

    def __init__(self):
        """初始化 AKShare 提供者"""
        if not AKSHARE_AVAILABLE:
            raise ImportError("akshare 未安装，请运行: pip install akshare")

        self._stock_list_cache: Dict[str, List[Stock]] = {}
        self._stock_list_cache_time: Dict[str, datetime] = {}
        self._cache_ttl = timedelta(hours=1)  # 股票列表缓存1小时
        self._stock_mapping = get_stock_mapping()  # 使用全局映射

        # 为访问东方财富的接口添加节流器，保证最小间隔为1秒
        # 避免频繁请求被对方限流或封禁
        self._eastmoney_lock = asyncio.Lock()
        self._last_eastmoney_call = 0.0

    @property
    def name(self) -> str:
        return "akshare"

    @property
    def supported_markets(self) -> List[str]:
        return ["cn_a", "cn_index", "hk"]

    def _run_sync(self, func, *args, **kwargs):
        """在事件循环中运行同步函数"""
        loop = asyncio.get_event_loop()
        return loop.run_in_executor(None, lambda: func(*args, **kwargs))

    def _parse_exchange_from_code(self, code: str) -> Exchange:
        """根据股票代码判断交易所"""
        if code.startswith(("60", "68")):
            return Exchange.SSE
        elif code.startswith(("00", "30")):
            return Exchange.SZSE
        elif code.startswith(("8", "4")):
            return Exchange.BSE
        return Exchange.SZSE

    def _parse_board_from_code(self, code: str) -> BoardType:
        """根据股票代码判断板块"""
        if code.startswith("688"):
            return BoardType.STAR
        elif code.startswith("60"):
            return BoardType.MAIN_SSE
        elif code.startswith("300"):
            return BoardType.GEM
        elif code.startswith("00"):
            return BoardType.MAIN_SZSE
        elif code.startswith(("8", "4")):
            return BoardType.BSE_MAIN
        return BoardType.MAIN_SZSE

    # ==================== 股票列表 ====================

    async def get_stock_list(self, market: str = "cn_a") -> List[Stock]:
        """获取A股股票列表"""
        # 检查缓存
        cache_key = market
        if cache_key in self._stock_list_cache:
            cache_time = self._stock_list_cache_time.get(cache_key)
            if cache_time and datetime.now() - cache_time < self._cache_ttl:
                return self._stock_list_cache[cache_key]

        try:
            # 使用节流调用东方财富接口
            df = await self._call_eastmoney(ak.stock_zh_a_spot_em)

            stocks = []
            for _, row in df.iterrows():
                code = str(row["代码"])
                exchange = self._parse_exchange_from_code(code)
                board = self._parse_board_from_code(code)

                stock = Stock(
                    code=code,
                    name=str(row["名称"]),
                    full_code=f"{code}.{'SH' if exchange == Exchange.SSE else 'SZ'}",
                    market=Market.CN_A,
                    exchange=exchange,
                    board=board,
                    data_source=DataSource.AKSHARE,
                )
                stocks.append(stock)

            # 更新缓存
            self._stock_list_cache[cache_key] = stocks
            self._stock_list_cache_time[cache_key] = datetime.now()

            # 更新股票映射（如果过期或未初始化）
            if self._stock_mapping.is_stale():
                self._stock_mapping.build_from_stocks(stocks)

            return stocks

        except Exception as e:
            logger.error(f"获取股票列表失败: {e}")
            return []

    def get_code_by_name(self, name: str) -> Optional[str]:
        """
        根据股票名称获取代码

        Args:
            name: 股票名称

        Returns:
            股票代码，如果未找到返回 None
        """
        return cast(Optional[str], self._stock_mapping.get_code_by_name(name))

    def get_name_by_code(self, code: str) -> Optional[str]:
        """
        根据股票代码获取名称

        Args:
            code: 股票代码

        Returns:
            股票名称，如果未找到返回 None
        """
        return cast(Optional[str], self._stock_mapping.get_name_by_code(code))

    def search_stocks(self, query: str) -> List[Dict[str, Any]]:
        """
        搜索股票

        支持名称、代码、拼音首字母匹配

        Args:
            query: 搜索关键词

        Returns:
            匹配结果列表
        """
        # _stock_mapping.search 可能被 mypy 视为 untyped Any，显式 cast 到期望类型
        return cast(List[Dict[str, Any]], self._stock_mapping.search(query))

    async def get_stock_info(self, code: str) -> Optional[Stock]:
        """获取单只股票信息"""
        try:
            # 获取个股资料
            df = await self._run_sync(ak.stock_individual_info_em, symbol=code)

            if df is None or df.empty:
                return None

            # 解析数据
            info_dict = {}
            for _, row in df.iterrows():
                key = str(row["item"])
                value = row["value"]
                info_dict[key] = value

            exchange = self._parse_exchange_from_code(code)
            board = self._parse_board_from_code(code)

            # 解析上市日期
            list_date = None
            if "上市时间" in info_dict:
                try:
                    list_date = datetime.strptime(
                        str(info_dict["上市时间"]), "%Y%m%d"
                    ).date()
                except (ValueError, TypeError):
                    pass

            stock = Stock(
                code=code,
                name=info_dict.get("股票简称", ""),
                full_code=f"{code}.{'SH' if exchange == Exchange.SSE else 'SZ'}",
                market=Market.CN_A,
                exchange=exchange,
                board=board,
                list_date=list_date,
                total_share=(
                    float(info_dict.get("总股本", 0)) / 1e8
                    if info_dict.get("总股本")
                    else None
                ),
                float_share=(
                    float(info_dict.get("流通股", 0)) / 1e8
                    if info_dict.get("流通股")
                    else None
                ),
                industry_sw_l1=info_dict.get("行业"),
                data_source=DataSource.AKSHARE,
            )

            return stock

        except Exception as e:
            logger.error(f"获取股票信息失败 {code}: {e}")
            return None

    # ==================== 实时行情 ====================

    async def get_realtime_quotes(
        self, codes: List[str] | None = None
    ) -> List[StockQuote]:
        """获取实时行情"""
        try:
            df = await self._run_sync(ak.stock_zh_a_spot_em)

            if df is None or df.empty:
                return []

            # 过滤指定股票
            if codes:
                df = df[df["代码"].isin(codes)]

            quotes = []
            for _, row in df.iterrows():
                try:
                    quote = StockQuote(
                        code=str(row["代码"]),
                        name=str(row["名称"]),
                        price=float(row["最新价"]) if pd.notna(row["最新价"]) else 0.0,
                        open=float(row["今开"]) if pd.notna(row["今开"]) else 0.0,
                        high=float(row["最高"]) if pd.notna(row["最高"]) else 0.0,
                        low=float(row["最低"]) if pd.notna(row["最低"]) else 0.0,
                        prev_close=float(row["昨收"]) if pd.notna(row["昨收"]) else 0.0,
                        change=float(row["涨跌额"]) if pd.notna(row["涨跌额"]) else 0.0,
                        change_pct=(
                            float(row["涨跌幅"]) if pd.notna(row["涨跌幅"]) else 0.0
                        ),
                        volume=int(row["成交量"]) if pd.notna(row["成交量"]) else 0,
                        turnover=(
                            float(row["成交额"]) if pd.notna(row["成交额"]) else 0.0
                        ),
                        turnover_rate=(
                            float(row["换手率"])
                            if pd.notna(row.get("换手率"))
                            else None
                        ),
                        amplitude=(
                            float(row["振幅"]) if pd.notna(row.get("振幅")) else None
                        ),
                        volume_ratio=(
                            float(row["量比"]) if pd.notna(row.get("量比")) else None
                        ),
                        pe_dynamic=(
                            float(row["市盈率-动态"])
                            if pd.notna(row.get("市盈率-动态"))
                            else None
                        ),
                        pb=(
                            float(row["市净率"])
                            if pd.notna(row.get("市净率"))
                            else None
                        ),
                        total_mv=(
                            float(row["总市值"])
                            if pd.notna(row.get("总市值"))
                            else None
                        ),
                        float_mv=(
                            float(row["流通市值"])
                            if pd.notna(row.get("流通市值"))
                            else None
                        ),
                        update_time=datetime.now(),
                        data_source=DataSource.AKSHARE,
                    )
                    quotes.append(quote)
                except Exception as e:
                    logger.warning(f"解析行情数据失败: {e}")
                    continue

            return quotes

        except Exception as e:
            logger.error(f"获取实时行情失败: {e}")
            return []

    async def get_quote(self, code: str) -> Optional[StockQuote]:
        """获取单只股票实时行情"""
        quotes = await self.get_realtime_quotes([code])
        return quotes[0] if quotes else None

    # ==================== 历史K线 ====================

    async def get_klines(
        self,
        code: str,
        period: PeriodType = PeriodType.DAILY,
        adjust: AdjustType = AdjustType.QFQ,
        start_date: Optional[date] = None,
        end_date: Optional[date] = None,
        limit: int = 1000,
    ) -> List[StockKLine]:
        """获取历史K线数据"""
        try:
            # 转换周期参数
            period_map = {
                PeriodType.DAILY: "daily",
                PeriodType.WEEKLY: "weekly",
                PeriodType.MONTHLY: "monthly",
            }
            ak_period = period_map.get(period, "daily")

            # 转换复权参数
            adjust_map = {
                AdjustType.NONE: "",
                AdjustType.QFQ: "qfq",
                AdjustType.HFQ: "hfq",
            }
            ak_adjust = adjust_map.get(adjust, "qfq")

            # 日期处理
            if not start_date:
                start_date = date.today() - timedelta(days=365)
            if not end_date:
                end_date = date.today()

            start_str = start_date.strftime("%Y%m%d")
            end_str = end_date.strftime("%Y%m%d")

            # 调用 akshare
            df = await self._run_sync(
                ak.stock_zh_a_hist,
                symbol=code,
                period=ak_period,
                start_date=start_str,
                end_date=end_str,
                adjust=ak_adjust,
            )

            if df is None or df.empty:
                return []

            klines = []
            for _, row in df.iterrows():
                try:
                    kline = StockKLine(
                        code=code,
                        date=pd.to_datetime(row["日期"]).date(),
                        period=period,
                        adjust=adjust,
                        open=float(row["开盘"]),
                        high=float(row["最高"]),
                        low=float(row["最低"]),
                        close=float(row["收盘"]),
                        volume=int(row["成交量"]),
                        turnover=float(row["成交额"]),
                        change_pct=(
                            float(row["涨跌幅"])
                            if pd.notna(row.get("涨跌幅"))
                            else None
                        ),
                        change=(
                            float(row["涨跌额"])
                            if pd.notna(row.get("涨跌额"))
                            else None
                        ),
                        amplitude=(
                            float(row["振幅"]) if pd.notna(row.get("振幅")) else None
                        ),
                        turnover_rate=(
                            float(row["换手率"])
                            if pd.notna(row.get("换手率"))
                            else None
                        ),
                        update_time=datetime.now(),
                        data_source=DataSource.AKSHARE,
                    )
                    klines.append(kline)
                except Exception as e:
                    logger.warning(f"解析K线数据失败: {e}")
                    continue

            # 限制返回数量
            if limit and len(klines) > limit:
                klines = klines[-limit:]

            return klines

        except Exception as e:
            logger.error(f"获取K线数据失败 {code}: {e}")
            return []

    # ==================== 财务数据 ====================

    async def get_financial(
        self,
        code: str,
        report_type: str = "all",
    ) -> List[StockFinancial]:
        """获取财务数据"""
        try:
            df = await self._run_sync(ak.stock_financial_abstract, symbol=code)

            if df is None or df.empty:
                return []

            financials = []
            for _, row in df.iterrows():
                try:
                    report_date_str = str(row.get("报告期", ""))
                    if not report_date_str:
                        continue

                    report_date = datetime.strptime(report_date_str, "%Y%m%d").date()

                    financial = StockFinancial(
                        code=code,
                        report_date=report_date,
                        eps=(
                            float(row.get("基本每股收益", 0))
                            if pd.notna(row.get("基本每股收益"))
                            else None
                        ),
                        roe=(
                            float(row.get("净资产收益率", 0))
                            if pd.notna(row.get("净资产收益率"))
                            else None
                        ),
                        bps=(
                            float(row.get("每股净资产", 0))
                            if pd.notna(row.get("每股净资产"))
                            else None
                        ),
                        revenue=(
                            float(row.get("营业总收入", 0))
                            if pd.notna(row.get("营业总收入"))
                            else None
                        ),
                        revenue_yoy=(
                            float(row.get("营业总收入同比增长", 0))
                            if pd.notna(row.get("营业总收入同比增长"))
                            else None
                        ),
                        net_profit=(
                            float(row.get("净利润", 0))
                            if pd.notna(row.get("净利润"))
                            else None
                        ),
                        net_profit_yoy=(
                            float(row.get("净利润同比增长", 0))
                            if pd.notna(row.get("净利润同比增长"))
                            else None
                        ),
                        update_time=datetime.now(),
                        data_source=DataSource.AKSHARE,
                    )
                    financials.append(financial)
                except Exception as e:
                    logger.warning(f"解析财务数据失败: {e}")
                    continue

            return financials

        except Exception as e:
            logger.error(f"获取财务数据失败 {code}: {e}")
            return []

    # ==================== 股东信息 ====================

    async def get_holders(
        self,
        code: str,
        holder_type: str = "top10",
    ) -> List[StockHolder]:
        """获取股东信息"""
        try:
            # 获取十大股东
            if holder_type in ("top10", "all"):
                df = await self._run_sync(ak.stock_gdfx_top_10_em, symbol=code)
            else:
                df = await self._run_sync(ak.stock_gdfx_free_top_10_em, symbol=code)

            if df is None or df.empty:
                return []

            holders = []
            for _, row in df.iterrows():
                try:
                    report_date_str = str(row.get("报告期", ""))
                    if not report_date_str:
                        continue

                    report_date = datetime.strptime(report_date_str, "%Y-%m-%d").date()

                    holder = StockHolder(
                        code=code,
                        report_date=report_date,
                        holder_name=str(row.get("股东名称", "")),
                        hold_num=(
                            float(row.get("持股数量", 0))
                            if pd.notna(row.get("持股数量"))
                            else None
                        ),
                        hold_ratio=(
                            float(row.get("持股比例", 0))
                            if pd.notna(row.get("持股比例"))
                            else None
                        ),
                        hold_change=(
                            float(row.get("增减", 0))
                            if pd.notna(row.get("增减"))
                            else None
                        ),
                        rank=(
                            int(row.get("序号", 0))
                            if pd.notna(row.get("序号"))
                            else None
                        ),
                        is_top10=holder_type == "top10",
                        is_top10_float=holder_type == "top10_float",
                        update_time=datetime.now(),
                        data_source=DataSource.AKSHARE,
                    )
                    holders.append(holder)
                except Exception as e:
                    logger.warning(f"解析股东数据失败: {e}")
                    continue

            return holders

        except Exception as e:
            logger.error(f"获取股东数据失败 {code}: {e}")
            return []

    # ==================== 公告信息 ====================

    async def get_announcements(
        self,
        code: str | None = None,
        start_date: Optional[date] = None,
        end_date: Optional[date] = None,
        limit: int = 100,
    ) -> List[StockAnnouncement]:
        """获取公告信息"""
        # AKShare 公告接口需要特殊处理，此处简化返回空
        # 实际可使用 CNInfoProvider 获取更详细的公告
        return []

    # ==================== 资金流向 ====================

    async def get_fund_flow(
        self,
        code: str,
        period: str = "daily",
    ) -> List[StockFundFlow]:
        """获取资金流向"""
        try:
            exchange = self._parse_exchange_from_code(code)
            market = "sh" if exchange == Exchange.SSE else "sz"

            df = await self._run_sync(
                ak.stock_individual_fund_flow,
                stock=code,
                market=market,
            )

            if df is None or df.empty:
                return []

            fund_flows = []
            for _, row in df.iterrows():
                try:
                    date_str = str(row.get("日期", ""))
                    if not date_str:
                        continue

                    flow_date = pd.to_datetime(date_str).date()

                    fund_flow = StockFundFlow(
                        code=code,
                        date=flow_date,
                        main_net_inflow=(
                            float(row.get("主力净流入-净额", 0))
                            if pd.notna(row.get("主力净流入-净额"))
                            else None
                        ),
                        main_net_inflow_pct=(
                            float(row.get("主力净流入-净占比", 0))
                            if pd.notna(row.get("主力净流入-净占比"))
                            else None
                        ),
                        super_large_net=(
                            float(row.get("超大单净流入-净额", 0))
                            if pd.notna(row.get("超大单净流入-净额"))
                            else None
                        ),
                        large_net=(
                            float(row.get("大单净流入-净额", 0))
                            if pd.notna(row.get("大单净流入-净额"))
                            else None
                        ),
                        medium_net=(
                            float(row.get("中单净流入-净额", 0))
                            if pd.notna(row.get("中单净流入-净额"))
                            else None
                        ),
                        small_net=(
                            float(row.get("小单净流入-净额", 0))
                            if pd.notna(row.get("小单净流入-净额"))
                            else None
                        ),
                        update_time=datetime.now(),
                        data_source=DataSource.AKSHARE,
                    )
                    fund_flows.append(fund_flow)
                except Exception as e:
                    logger.warning(f"解析资金流向数据失败: {e}")
                    continue

            return fund_flows

        except Exception as e:
            logger.error(f"获取资金流向失败 {code}: {e}")
            return []

    # ==================== 行业分类 ====================

    async def get_industry(
        self,
        code: str,
        classification: str = "sw",
    ) -> Optional[StockIndustry]:
        """获取行业分类"""
        try:
            # 获取股票信息中的行业
            stock = await self.get_stock_info(code)

            if stock and stock.industry_sw_l1:
                return StockIndustry(
                    code=code,
                    classification=IndustryClassification.SW,
                    industry_name=stock.industry_sw_l1,
                    level=1,
                    update_time=datetime.now(),
                    data_source=DataSource.AKSHARE,
                )

            return None

        except Exception as e:
            logger.error(f"获取行业分类失败 {code}: {e}")
            return None

    # ==================== 指数数据 ====================

    async def get_index_quotes(self) -> List[IndexQuote]:
        """获取主要指数行情"""
        try:
            df = await self._call_eastmoney(ak.stock_zh_index_spot_em)

            if df is None or df.empty:
                return []

            # 主要指数代码
            main_indices = ["000001", "399001", "399006", "000016", "000300", "000905"]
            df = df[df["代码"].isin(main_indices)]

            indices = []
            for _, row in df.iterrows():
                try:
                    index = IndexQuote(
                        code=str(row["代码"]),
                        name=str(row["名称"]),
                        price=float(row["最新价"]) if pd.notna(row["最新价"]) else 0.0,
                        change=float(row["涨跌额"]) if pd.notna(row["涨跌额"]) else 0.0,
                        change_pct=(
                            float(row["涨跌幅"]) if pd.notna(row["涨跌幅"]) else 0.0
                        ),
                        volume=(
                            float(row["成交量"]) / 1e8
                            if pd.notna(row.get("成交量"))
                            else 0.0
                        ),
                        turnover=(
                            float(row["成交额"]) / 1e8
                            if pd.notna(row.get("成交额"))
                            else 0.0
                        ),
                        update_time=datetime.now(),
                    )
                    indices.append(index)
                except Exception as e:
                    logger.warning(f"解析指数数据失败: {e}")
                    continue

            return indices

        except Exception as e:
            logger.error(f"获取指数行情失败: {e}")
            return []

    # ==================== 市场状态 ====================

    async def get_market_status(self, exchange: str = "sse") -> Optional[MarketStatus]:
        """获取市场状态"""
        try:
            # 获取全部股票统计涨跌
            quotes = await self.get_realtime_quotes()

            if not quotes:
                return None

            up_count = sum(1 for q in quotes if q.change_pct > 0)
            down_count = sum(1 for q in quotes if q.change_pct < 0)
            flat_count = sum(1 for q in quotes if q.change_pct == 0)
            limit_up_count = sum(1 for q in quotes if q.change_pct >= 9.9)
            limit_down_count = sum(1 for q in quotes if q.change_pct <= -9.9)

            total_volume = sum(q.volume for q in quotes) / 1e8
            total_turnover = sum(q.turnover for q in quotes) / 1e8

            from seesea.models.exchange import TradingSession

            return MarketStatus(
                exchange=Exchange.SSE if exchange == "sse" else Exchange.SZSE,
                is_trading_day=True,  # 简化处理
                current_session=TradingSession.MORNING,  # 简化处理
                total_stocks=len(quotes),
                trading_stocks=len(quotes),
                up_count=up_count,
                down_count=down_count,
                flat_count=flat_count,
                limit_up_count=limit_up_count,
                limit_down_count=limit_down_count,
                total_volume=total_volume,
                total_turnover=total_turnover,
                update_time=datetime.now(),
            )

        except Exception as e:
            logger.error(f"获取市场状态失败: {e}")
            return None

    # ==================== 龙虎榜数据 ====================

    async def get_lhb_data(
        self,
        trade_date: Optional[date] = None,
    ) -> List[Dict[str, Any]]:
        """获取龙虎榜数据"""
        try:
            if not trade_date:
                trade_date = date.today()

            date_str = trade_date.strftime("%Y%m%d")
            df = await self._call_eastmoney(
                ak.stock_lhb_detail_em, start_date=date_str, end_date=date_str
            )

            if df is None or df.empty:
                return []

            # pandas 的 to_dict 返回 untyped Any；明确转换为预期类型以满足 mypy
            return cast(List[Dict[str, Any]], df.to_dict("records"))

        except Exception as e:
            logger.error(f"获取龙虎榜数据失败: {e}")
            return []

    # ==================== 板块数据 ====================

    async def get_sector_list(
        self, sector_type: str = "industry"
    ) -> List[Dict[str, Any]]:
        """获取板块列表"""
        try:
            if sector_type == "industry":
                df = await self._call_eastmoney(ak.stock_board_industry_name_em)
            elif sector_type == "concept":
                df = await self._call_eastmoney(ak.stock_board_concept_name_em)
            else:
                return []

            if df is None or df.empty:
                return []

            return cast(List[Dict[str, Any]], df.to_dict("records"))

        except Exception as e:
            logger.error(f"获取板块列表失败: {e}")
            return []

    async def get_sector_stocks(self, sector_code: str) -> List[str]:
        """获取板块成分股"""
        try:
            df = await self._call_eastmoney(
                ak.stock_board_industry_cons_em,
                symbol=sector_code,
            )

            if df is None or df.empty:
                return []

            # 将 pandas 返回值显式转换为字符串列表，避免 Any 返回
            return [str(x) for x in list(df["代码"].tolist())]

        except Exception as e:
            logger.error(f"获取板块成分股失败: {e}")
            return []

    async def _call_eastmoney(self, func, *args, **kwargs):
        """调用 akshare 中访问东方财富的函数时使用的节流器。

        会确保两次对东方财富的网络请求间隔至少为1秒。
        func 应该是 akshare 的同步函数（本方法会在线程池中运行它）。
        """
        import time

        # 使用异步锁串行化对东方财富的访问
        async with self._eastmoney_lock:
            now = time.time()
            elapsed = now - getattr(self, "_last_eastmoney_call", 0.0)
            wait = 1.0 - elapsed
            if wait > 0:
                await asyncio.sleep(wait)

            # 在线程池中执行同步请求函数
            result = await self._run_sync(func, *args, **kwargs)
            # 更新最后调用时间
            self._last_eastmoney_call = time.time()
            return result
