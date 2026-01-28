"""
SeeSea 股票SDK客户端

提供统一的股票数据访问接口，使用Rust核心库的PyStockClient。
"""

from typing import Dict, List, Optional, Any
import json

from ..base import BaseClient
from ..seesea_types.common_types import Result, Error

# 尝试导入 Rust 绑定
try:
    import seesea_core

    _STOCK_AVAILABLE = hasattr(seesea_core, "PyStockClient")
except ImportError:
    _STOCK_AVAILABLE = False


def _parse_json(data: str) -> Any:
    """解析 JSON 字符串"""
    try:
        return json.loads(data) if data else []
    except json.JSONDecodeError:
        return []


class StockClient(BaseClient[Dict[str, Any]]):
    """
    SeeSea 股票数据客户端

    提供高层次的股票数据访问功能，支持实时行情、历史数据、板块信息等。
    使用 Rust 核心库的 PyStockClient 进行数据获取。

    主要功能:
    - 股票列表和基础信息查询
    - 实时行情数据获取
    - 历史K线数据查询
    - 板块数据查询
    - 资金流向分析
    - 市场指数查询

    示例:
        >>> client = StockClient()
        >>> with client:
        ...     result = client.get_quote("000001")
        ...     if result.success:
        ...         quote = result.data
        ...         print(f"股价: {quote.get('price')}")
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """
        初始化股票客户端

        Args:
            config: 客户端配置
        """
        super().__init__(config)
        self._rust_client = None

        if not _STOCK_AVAILABLE:
            error = Error(
                code="StockClient.init",
                message="seesea_core.PyStockClient not available",
                source="StockClient",
            )
            self._set_error(error)

    def connect(self) -> Result[bool]:
        """连接到股票数据服务"""
        with self._handle_operation("connect"):
            if not _STOCK_AVAILABLE:
                return Result(
                    success=False,
                    error=Error(
                        code="StockClient.connect",
                        message="seesea_core.PyStockClient not available",
                    ),
                )

            try:
                self._rust_client = seesea_core.PyStockClient()
                self._is_connected = True
                return Result(success=True, data=True)

            except Exception as e:
                return Result(
                    success=False,
                    error=Error(
                        code="StockClient.connect",
                        message=f"Failed to create stock client: {e}",
                    ),
                )

    def disconnect(self) -> Result[bool]:
        """断开股票数据服务连接"""
        with self._handle_operation("disconnect"):
            self._rust_client = None
            self._is_connected = False
            return Result(success=True, data=True)

    def health_check(self) -> Result[Dict[str, Any]]:
        """股票数据服务健康检查"""
        if not self._is_connected or not self._rust_client:
            return Result(
                success=False,
                error=Error(
                    code="StockClient.health_check", message="Client not connected"
                ),
            )

        health_data = {
            "status": "healthy",
            "stock_available": _STOCK_AVAILABLE,
            "connected": self._is_connected,
        }
        return Result(success=True, data=health_data)

    def get_info(self) -> Result[Dict[str, Any]]:
        """获取股票客户端信息"""
        info = {
            "client_type": "StockClient",
            "version": "2.2.2",
            "stock_available": _STOCK_AVAILABLE,
            "connected": self._is_connected,
        }
        return Result(success=True, data=info)

    def _ensure_connected(self) -> Optional[Error]:
        """确保客户端已连接"""
        if not self._is_connected or not self._rust_client:
            return Error(
                code="StockClient",
                message="Client not connected. Call connect() first.",
            )
        return None

    # ==================== 股票列表和基础信息 ====================

    def get_stock_list(self, market: str = "a") -> Result[List[Dict[str, Any]]]:
        """
        获取股票列表

        Args:
            market: 市场代码 ("a", "b", "hk", "us")

        Returns:
            Result[List[Dict]]: 股票列表数据
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            if market == "a":
                data = (
                    self._rust_client.get_stock_info_a_code_name()
                    if self._rust_client
                    else ""
                )
            elif market == "b":
                data = (
                    self._rust_client.get_stock_zh_b_spot_em()
                    if self._rust_client
                    else "" if self._rust_client else ""
                )
            elif market == "hk":
                data = (
                    self._rust_client.get_stock_hk_spot_em()
                    if self._rust_client
                    else "" if self._rust_client else ""
                )
            elif market == "us":
                data = (
                    self._rust_client.get_stock_us_spot_em()
                    if self._rust_client
                    else "" if self._rust_client else ""
                )
            else:
                data = (
                    self._rust_client.get_stock_info_a_code_name()
                    if self._rust_client
                    else ""
                )
            return Result(success=True, data=_parse_json(data))
        except Exception as e:
            return Result(
                success=False,
                error=Error(code="StockClient.get_stock_list", message=str(e)),
            )

    def get_stock_info(self, symbol: str) -> Result[Dict[str, Any]]:
        """
        获取个股基础信息

        Args:
            symbol: 股票代码

        Returns:
            Result[Dict]: 股票基础信息
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            data = (
                self._rust_client.get_stock_individual_info(symbol)
                if self._rust_client
                else ""
            )
            return Result(success=True, data=_parse_json(data))
        except Exception as e:
            return Result(
                success=False,
                error=Error(code="StockClient.get_stock_info", message=str(e)),
            )

    # ==================== 实时行情 ====================

    def get_quote(self, symbol: str) -> Result[Dict[str, Any]]:
        """
        获取个股实时行情

        Args:
            symbol: 股票代码

        Returns:
            Result[Dict]: 实时行情数据
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            data = self._rust_client.get_by_code(symbol) if self._rust_client else ""
            if data:
                return Result(success=True, data=_parse_json(data))
            else:
                return Result(
                    success=False,
                    error=Error(
                        code="StockClient.get_quote", message=f"未找到股票: {symbol}"
                    ),
                )
        except Exception as e:
            return Result(
                success=False, error=Error(code="StockClient.get_quote", message=str(e))
            )

    def get_quotes(self, market: str = "a") -> Result[List[Dict[str, Any]]]:
        """
        获取市场全部实时行情

        Args:
            market: 市场代码

        Returns:
            Result[List[Dict]]: 全市场行情数据
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            if market == "a":
                data = (
                    self._rust_client.get_stock_zh_a_spot_em()
                    if self._rust_client
                    else ""
                )
            elif market == "b":
                data = (
                    self._rust_client.get_stock_zh_b_spot_em()
                    if self._rust_client
                    else ""
                )
            elif market == "hk":
                data = (
                    self._rust_client.get_stock_hk_spot_em()
                    if self._rust_client
                    else ""
                )
            elif market == "us":
                data = (
                    self._rust_client.get_stock_us_spot_em()
                    if self._rust_client
                    else ""
                )
            else:
                data = (
                    self._rust_client.get_stock_zh_a_spot_em()
                    if self._rust_client
                    else ""
                )
            return Result(success=True, data=_parse_json(data))
        except Exception as e:
            return Result(
                success=False,
                error=Error(code="StockClient.get_quotes", message=str(e)),
            )

    # ==================== 历史行情 ====================

    def get_kline(
        self,
        symbol: str,
        period: str = "daily",
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        adjust: str = "qfq",
    ) -> Result[List[Dict[str, Any]]]:
        """
        获取K线数据

        Args:
            symbol: 股票代码
            period: K线周期 ("daily", "weekly", "monthly")
            start_date: 开始日期 (YYYYMMDD)
            end_date: 结束日期 (YYYYMMDD)
            adjust: 复权类型 ("qfq"前复权, "hfq"后复权, ""不复权)

        Returns:
            Result[List[Dict]]: K线数据
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            data = (
                self._rust_client.get_stock_zh_a_hist(
                    symbol, period, start_date, end_date, adjust
                )
                if self._rust_client
                else ""
            )
            return Result(success=True, data=_parse_json(data))
        except Exception as e:
            return Result(
                success=False, error=Error(code="StockClient.get_kline", message=str(e))
            )

    def get_kline_hk(
        self,
        symbol: str,
        period: str = "daily",
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        adjust: str = "qfq",
    ) -> Result[List[Dict[str, Any]]]:
        """
        获取港股K线数据

        Args:
            symbol: 股票代码
            period: K线周期
            start_date: 开始日期
            end_date: 结束日期
            adjust: 复权类型

        Returns:
            Result[List[Dict]]: K线数据
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            data = (
                self._rust_client.get_stock_hk_hist(
                    symbol, period, start_date, end_date, adjust
                )
                if self._rust_client
                else ""
            )
            return Result(success=True, data=_parse_json(data))
        except Exception as e:
            return Result(
                success=False,
                error=Error(code="StockClient.get_kline_hk", message=str(e)),
            )

    # ==================== 板块数据 ====================

    def get_industry_list(self) -> Result[List[Dict[str, Any]]]:
        """
        获取行业板块列表

        Returns:
            Result[List[Dict]]: 行业板块列表
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            data = (
                self._rust_client.get_board_industry_name() if self._rust_client else ""
            )
            return Result(success=True, data=_parse_json(data))
        except Exception as e:
            return Result(
                success=False,
                error=Error(code="StockClient.get_industry_list", message=str(e)),
            )

    def get_concept_list(self) -> Result[List[Dict[str, Any]]]:
        """
        获取概念板块列表

        Returns:
            Result[List[Dict]]: 概念板块列表
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            data = (
                self._rust_client.get_board_concept_name() if self._rust_client else ""
            )
            return Result(success=True, data=_parse_json(data))
        except Exception as e:
            return Result(
                success=False,
                error=Error(code="StockClient.get_concept_list", message=str(e)),
            )

    def get_industry_stocks(self, symbol: str) -> Result[List[Dict[str, Any]]]:
        """
        获取行业板块成分股

        Args:
            symbol: 板块代码

        Returns:
            Result[List[Dict]]: 成分股列表
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            data = (
                self._rust_client.get_board_industry_cons(symbol)
                if self._rust_client
                else ""
            )
            return Result(success=True, data=_parse_json(data))
        except Exception as e:
            return Result(
                success=False,
                error=Error(code="StockClient.get_industry_stocks", message=str(e)),
            )

    def get_concept_stocks(self, symbol: str) -> Result[List[Dict[str, Any]]]:
        """
        获取概念板块成分股

        Args:
            symbol: 板块代码

        Returns:
            Result[List[Dict]]: 成分股列表
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            data = (
                self._rust_client.get_board_concept_cons(symbol)
                if self._rust_client
                else ""
            )
            return Result(success=True, data=_parse_json(data))
        except Exception as e:
            return Result(
                success=False,
                error=Error(code="StockClient.get_concept_stocks", message=str(e)),
            )

    # ==================== 指数数据 ====================

    def get_index_list(self) -> Result[List[Dict[str, Any]]]:
        """
        获取指数列表

        Returns:
            Result[List[Dict]]: 指数列表
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            data = self._rust_client.get_index_spot() if self._rust_client else ""
            return Result(success=True, data=_parse_json(data))
        except Exception as e:
            return Result(
                success=False,
                error=Error(code="StockClient.get_index_list", message=str(e)),
            )

    # ==================== 资金流向 ====================

    def get_market_fund_flow(self) -> Result[List[Dict[str, Any]]]:
        """
        获取大盘资金流向

        Returns:
            Result[List[Dict]]: 资金流向数据
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            data = self._rust_client.get_market_fund_flow() if self._rust_client else ""
            return Result(success=True, data=_parse_json(data))
        except Exception as e:
            return Result(
                success=False,
                error=Error(code="StockClient.get_market_fund_flow", message=str(e)),
            )

    # ==================== 涨跌停 ====================

    def get_zt_pool(self, date: Optional[str] = None) -> Result[List[Dict[str, Any]]]:
        """
        获取涨停板数据

        Args:
            date: 日期 (YYYYMMDD)

        Returns:
            Result[List[Dict]]: 涨停板数据
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            data = self._rust_client.get_zt_pool(date) if self._rust_client else ""
            return Result(success=True, data=_parse_json(data))
        except Exception as e:
            return Result(
                success=False,
                error=Error(code="StockClient.get_zt_pool", message=str(e)),
            )

    def get_dt_pool(self, date: Optional[str] = None) -> Result[List[Dict[str, Any]]]:
        """
        获取跌停板数据

        Args:
            date: 日期 (YYYYMMDD)

        Returns:
            Result[List[Dict]]: 跌停板数据
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            data = self._rust_client.get_dt_pool(date) if self._rust_client else ""
            return Result(success=True, data=_parse_json(data))
        except Exception as e:
            return Result(
                success=False,
                error=Error(code="StockClient.get_dt_pool", message=str(e)),
            )

    # ==================== 搜索 ====================

    def search(
        self, keyword: str, limit: Optional[int] = None
    ) -> Result[List[Dict[str, Any]]]:
        """
        搜索股票

        Args:
            keyword: 搜索关键词（代码、名称、拼音）
            limit: 返回结果数量限制

        Returns:
            Result[List[Dict]]: 匹配的股票列表
        """
        if err := self._ensure_connected():
            return Result(success=False, error=err)

        try:
            data = self._rust_client.search(keyword, limit) if self._rust_client else ""
            return Result(success=True, data=_parse_json(data))
        except Exception as e:
            return Result(
                success=False, error=Error(code="StockClient.search", message=str(e))
            )
