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
巨潮资讯数据提供者

基于巨潮资讯网站数据，主要用于获取公告信息和公司详细资料。
使用 crawl4ai 进行网页抓取和解析。
"""

import re
from datetime import date, datetime
from typing import Optional, List, Dict, Any
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
    PeriodType,
    AdjustType,
    DataSource,
    AnnouncementType,
)
from seesea.models.exchange import IndexQuote, MarketStatus

logger = logging.getLogger(__name__)

# 尝试导入 crawl4ai
try:
    from crawl4ai import AsyncWebCrawler

    CRAWL4AI_AVAILABLE = True
except ImportError:
    CRAWL4AI_AVAILABLE = False
    logger.warning("crawl4ai 未安装，请运行: pip install crawl4ai")


class CNInfoProvider(BaseProvider):
    """
    巨潮资讯数据提供者

    主要用于获取:
    - 上市公司公告
    - 公司详细资料
    - 股东变动信息
    - 公开信息（龙虎榜等）
    """

    def __init__(self):
        """初始化巨潮资讯提供者"""
        self._crawler: Optional[AsyncWebCrawler] = None

    @property
    def name(self) -> str:
        return "cninfo"

    @property
    def supported_markets(self) -> List[str]:
        return ["cn_a"]

    async def _get_crawler(self) -> AsyncWebCrawler:
        """获取爬虫实例"""
        if not CRAWL4AI_AVAILABLE:
            raise ImportError("crawl4ai 未安装，请运行: pip install crawl4ai")

        if self._crawler is None:
            self._crawler = AsyncWebCrawler()
        return self._crawler

    async def close(self):
        """关闭爬虫"""
        if self._crawler:
            await self._crawler.close()
            self._crawler = None

    # ==================== 股票列表 (不支持) ====================

    async def get_stock_list(self, market: str = "cn_a") -> List[Stock]:
        """巨潮资讯不提供股票列表，使用 AKShare"""
        return []

    async def get_stock_info(self, code: str) -> Optional[Stock]:
        """获取公司详细信息"""
        try:
            crawler = await self._get_crawler()

            # 使用个股F10页面
            url = f"http://www.cninfo.com.cn/new/disclosure/stock?stockCode={code}"

            result = await crawler.arun(
                url=url, extractor_kwargs={"extract_images": False}
            )

            if not result or not result.markdown:
                return None

            # 解析 markdown 提取公司信息
            markdown = result.markdown

            # 提取公司名称
            name_pattern = r"([^\s]+)\s*\[(\d{6})\]"
            name_match = re.search(name_pattern, markdown)

            if name_match:
                return Stock(
                    code=code,
                    name=name_match.group(1),
                    full_code=(
                        f"{code}.SZ" if code.startswith(("0", "3")) else f"{code}.SH"
                    ),
                    data_source=DataSource.CNINFO,
                )

            return None

        except Exception as e:
            logger.error(f"获取公司信息失败 {code}: {e}")
            return None

    # ==================== 实时行情 (不支持) ====================

    async def get_realtime_quotes(
        self, codes: List[str] | None = None
    ) -> List[StockQuote]:
        """巨潮资讯不提供实时行情，使用 AKShare"""
        return []

    async def get_quote(self, code: str) -> Optional[StockQuote]:
        """巨潮资讯不提供实时行情，使用 AKShare"""
        return None

    # ==================== 历史K线 (不支持) ====================

    async def get_klines(
        self,
        code: str,
        period: PeriodType = PeriodType.DAILY,
        adjust: AdjustType = AdjustType.QFQ,
        start_date: Optional[date] = None,
        end_date: Optional[date] = None,
        limit: int = 1000,
    ) -> List[StockKLine]:
        """巨潮资讯不提供K线数据，使用 AKShare"""
        return []

    # ==================== 财务数据 ====================

    async def get_financial(
        self,
        code: str,
        report_type: str = "all",
    ) -> List[StockFinancial]:
        """获取财务数据"""
        try:
            crawler = await self._get_crawler()

            url = f"http://www.cninfo.com.cn/new/disclosure/stock?stockCode={code}&tabName=data"

            result = await crawler.arun(
                url=url, extractor_kwargs={"extract_images": False}
            )

            if not result or not result.markdown:
                return []

            # 解析财务数据
            markdown = result.markdown
            financials = []

            # 提取财务指标
            patterns = {
                "debt_ratio": r"负债率：\s*([\d,\.]+%)",
                "net_profit": r"净利润：\s*([\d,\.]+亿)",
                "main_income": r"主营收入：\s*([\d,\.]+亿)",
                "roe": r"ROE：\s*([\d,\.]+%)",
            }

            financial_data = {}
            for key, pattern in patterns.items():
                match = re.search(pattern, markdown)
                if match:
                    financial_data[key] = match.group(1)

            if financial_data:
                financial = StockFinancial(
                    code=code,
                    report_date=date.today(),
                    roe=(
                        float(financial_data.get("roe", "0").replace("%", ""))
                        if financial_data.get("roe")
                        else None
                    ),
                    debt_ratio=(
                        float(financial_data.get("debt_ratio", "0").replace("%", ""))
                        if financial_data.get("debt_ratio")
                        else None
                    ),
                    update_time=datetime.now(),
                    data_source=DataSource.CNINFO,
                )
                financials.append(financial)

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
        # 简化实现，可后续扩展
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
        try:
            crawler = await self._get_crawler()

            # 构建搜索URL
            if code:
                url = f"http://www.cninfo.com.cn/new/fulltextSearch?keyWord={code}&searchType=2&pageNum=1&pageSize={limit}"
            else:
                url = f"http://www.cninfo.com.cn/new/fulltextSearch?searchType=2&pageNum=1&pageSize={limit}"

            result = await crawler.arun(
                url=url, extractor_kwargs={"extract_images": False}
            )

            if not result or not result.markdown:
                return []

            # 解析公告列表
            markdown = result.markdown
            announcements = []

            # 匹配公告链接格式
            pattern = r"\[(.*?)\]\((https?://[^)]+)\)\s*\|\s*(\d{4}-\d{2}-\d{2}(?:\s+\d{2}:\d{2})?)"
            matches = re.findall(pattern, markdown)

            for match in matches[:limit]:
                title = match[0].strip()
                url = match[1].strip()
                publish_time_str = match[2].strip()

                if not title or not url:
                    continue

                # 解析发布时间
                try:
                    if " " in publish_time_str:
                        publish_time = datetime.strptime(
                            publish_time_str, "%Y-%m-%d %H:%M"
                        )
                    else:
                        publish_time = datetime.strptime(publish_time_str, "%Y-%m-%d")
                except (ValueError, TypeError):
                    publish_time = datetime.now()

                # 判断公告类型
                announcement_type = AnnouncementType.GENERAL
                if "年报" in title or "季报" in title or "半年报" in title:
                    announcement_type = AnnouncementType.FINANCIAL_REPORT
                elif "分红" in title or "派息" in title:
                    announcement_type = AnnouncementType.DIVIDEND
                elif "股东" in title:
                    announcement_type = AnnouncementType.SHAREHOLDER
                elif "高管" in title or "董事" in title:
                    announcement_type = AnnouncementType.EXECUTIVE
                elif "风险" in title:
                    announcement_type = AnnouncementType.RISK_WARNING

                announcement = StockAnnouncement(
                    code=code or "",
                    title=title,
                    url=url,
                    publish_time=publish_time,
                    announcement_type=announcement_type,
                    update_time=datetime.now(),
                    data_source=DataSource.CNINFO,
                )
                announcements.append(announcement)

            return announcements

        except Exception as e:
            logger.error(f"获取公告信息失败: {e}")
            return []

    # ==================== 资金流向 (不支持) ====================

    async def get_fund_flow(
        self,
        code: str,
        period: str = "daily",
    ) -> List[StockFundFlow]:
        """巨潮资讯不提供资金流向，使用 AKShare"""
        return []

    # ==================== 行业分类 (不支持) ====================

    async def get_industry(
        self,
        code: str,
        classification: str = "sw",
    ) -> Optional[StockIndustry]:
        """巨潮资讯不提供行业分类，使用 AKShare"""
        return None

    # ==================== 指数数据 (不支持) ====================

    async def get_index_quotes(self) -> List[IndexQuote]:
        """巨潮资讯不提供指数数据，使用 AKShare"""
        return []

    # ==================== 市场状态 (不支持) ====================

    async def get_market_status(self, exchange: str = "sse") -> Optional[MarketStatus]:
        """巨潮资讯不提供市场状态，使用 AKShare"""
        return None

    # ==================== 公开信息 ====================

    async def get_public_info(
        self,
        trade_date: Optional[date] = None,
    ) -> List[Dict[str, Any]]:
        """获取公开信息（龙虎榜等）"""
        try:
            crawler = await self._get_crawler()

            # 构建URL
            if trade_date:
                date_str = trade_date.strftime("%Y-%m-%d")
                url = f"http://www.cninfo.com.cn/new/commonUrl?url=data/public-information&date={date_str},{date_str}"
            else:
                url = (
                    "http://www.cninfo.com.cn/new/commonUrl?url=data/public-information"
                )

            result = await crawler.arun(
                url=url, extractor_kwargs={"extract_images": False}
            )

            if not result or not result.markdown:
                return []

            # 解析公开信息
            # 这里返回原始markdown供上层处理
            return [{"markdown": result.markdown, "url": url}]

        except Exception as e:
            logger.error(f"获取公开信息失败: {e}")
            return []
