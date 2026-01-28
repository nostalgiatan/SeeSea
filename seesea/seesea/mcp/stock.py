"""
SeeSea MCP Stock 工具

直接定义股票相关的 MCP 工具。
"""

from typing import Optional, Dict, Any
from fastmcp import FastMCP
from ..sdk.stock.client import StockClient

# 初始化股票客户端
_stock_client = StockClient()


async def get_stock_list(market: str = "a") -> Dict[str, Any]:
    """
    获取股票列表

    Args:
        market: 市场代码 ("a", "b", "hk", "us")

    Returns:
        股票列表数据
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        list_result = _stock_client.get_stock_list(market)
        _stock_client.disconnect()

        if not list_result.success:
            return {
                "success": False,
                "error": (
                    list_result.error.message
                    if list_result.error
                    else "获取股票列表失败"
                ),
            }

        return {"success": True, "stocks": list_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取股票列表失败: {str(e)}"}


async def get_stock_info(symbol: str) -> Dict[str, Any]:
    """
    获取个股基础信息

    Args:
        symbol: 股票代码

    Returns:
        股票基础信息
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        info_result = _stock_client.get_stock_info(symbol)
        _stock_client.disconnect()

        if not info_result.success:
            return {
                "success": False,
                "error": (
                    info_result.error.message
                    if info_result.error
                    else "获取股票信息失败"
                ),
            }

        return {"success": True, "info": info_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取股票信息失败: {str(e)}"}


async def get_quote(symbol: str) -> Dict[str, Any]:
    """
    获取个股实时行情

    Args:
        symbol: 股票代码

    Returns:
        实时行情数据
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        quote_result = _stock_client.get_quote(symbol)
        _stock_client.disconnect()

        if not quote_result.success:
            return {
                "success": False,
                "error": (
                    quote_result.error.message if quote_result.error else "获取行情失败"
                ),
            }

        return {"success": True, "quote": quote_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取行情失败: {str(e)}"}


async def get_quotes(market: str = "a") -> Dict[str, Any]:
    """
    获取市场全部实时行情

    Args:
        market: 市场代码

    Returns:
        全市场行情数据
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        quotes_result = _stock_client.get_quotes(market)
        _stock_client.disconnect()

        if not quotes_result.success:
            return {
                "success": False,
                "error": (
                    quotes_result.error.message
                    if quotes_result.error
                    else "获取行情失败"
                ),
            }

        return {"success": True, "quotes": quotes_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取行情失败: {str(e)}"}


async def get_kline(
    symbol: str,
    period: str = "daily",
    start_date: Optional[str] = None,
    end_date: Optional[str] = None,
    adjust: str = "qfq",
) -> Dict[str, Any]:
    """
    获取K线数据

    Args:
        symbol: 股票代码
        period: K线周期 ("daily", "weekly", "monthly")
        start_date: 开始日期 (YYYYMMDD)
        end_date: 结束日期 (YYYYMMDD)
        adjust: 复权类型 ("qfq"前复权, "hfq"后复权, ""不复权)

    Returns:
        K线数据
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        kline_result = _stock_client.get_kline(
            symbol, period, start_date, end_date, adjust
        )
        _stock_client.disconnect()

        if not kline_result.success:
            return {
                "success": False,
                "error": (
                    kline_result.error.message if kline_result.error else "获取K线失败"
                ),
            }

        return {"success": True, "kline": kline_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取K线失败: {str(e)}"}


async def get_kline_hk(
    symbol: str,
    period: str = "daily",
    start_date: Optional[str] = None,
    end_date: Optional[str] = None,
    adjust: str = "qfq",
) -> Dict[str, Any]:
    """
    获取港股K线数据

    Args:
        symbol: 股票代码
        period: K线周期
        start_date: 开始日期
        end_date: 结束日期
        adjust: 复权类型

    Returns:
        K线数据
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        kline_result = _stock_client.get_kline_hk(
            symbol, period, start_date, end_date, adjust
        )
        _stock_client.disconnect()

        if not kline_result.success:
            return {
                "success": False,
                "error": (
                    kline_result.error.message if kline_result.error else "获取K线失败"
                ),
            }

        return {"success": True, "kline": kline_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取K线失败: {str(e)}"}


async def get_industry_list() -> Dict[str, Any]:
    """
    获取行业板块列表

    Returns:
        行业板块列表
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        list_result = _stock_client.get_industry_list()
        _stock_client.disconnect()

        if not list_result.success:
            return {
                "success": False,
                "error": (
                    list_result.error.message
                    if list_result.error
                    else "获取行业列表失败"
                ),
            }

        return {"success": True, "industries": list_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取行业列表失败: {str(e)}"}


async def get_concept_list() -> Dict[str, Any]:
    """
    获取概念板块列表

    Returns:
        概念板块列表
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        list_result = _stock_client.get_concept_list()
        _stock_client.disconnect()

        if not list_result.success:
            return {
                "success": False,
                "error": (
                    list_result.error.message
                    if list_result.error
                    else "获取概念列表失败"
                ),
            }

        return {"success": True, "concepts": list_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取概念列表失败: {str(e)}"}


async def get_industry_stocks(symbol: str) -> Dict[str, Any]:
    """
    获取行业板块成分股

    Args:
        symbol: 板块代码

    Returns:
        成分股列表
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        stocks_result = _stock_client.get_industry_stocks(symbol)
        _stock_client.disconnect()

        if not stocks_result.success:
            return {
                "success": False,
                "error": (
                    stocks_result.error.message
                    if stocks_result.error
                    else "获取成分股失败"
                ),
            }

        return {"success": True, "stocks": stocks_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取成分股失败: {str(e)}"}


async def get_concept_stocks(symbol: str) -> Dict[str, Any]:
    """
    获取概念板块成分股

    Args:
        symbol: 板块代码

    Returns:
        成分股列表
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        stocks_result = _stock_client.get_concept_stocks(symbol)
        _stock_client.disconnect()

        if not stocks_result.success:
            return {
                "success": False,
                "error": (
                    stocks_result.error.message
                    if stocks_result.error
                    else "获取成分股失败"
                ),
            }

        return {"success": True, "stocks": stocks_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取成分股失败: {str(e)}"}


async def get_index_list() -> Dict[str, Any]:
    """
    获取指数列表

    Returns:
        指数列表
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        list_result = _stock_client.get_index_list()
        _stock_client.disconnect()

        if not list_result.success:
            return {
                "success": False,
                "error": (
                    list_result.error.message
                    if list_result.error
                    else "获取指数列表失败"
                ),
            }

        return {"success": True, "indices": list_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取指数列表失败: {str(e)}"}


async def get_market_fund_flow() -> Dict[str, Any]:
    """
    获取大盘资金流向

    Returns:
        资金流向数据
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        flow_result = _stock_client.get_market_fund_flow()
        _stock_client.disconnect()

        if not flow_result.success:
            return {
                "success": False,
                "error": (
                    flow_result.error.message
                    if flow_result.error
                    else "获取资金流向失败"
                ),
            }

        return {"success": True, "flow": flow_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取资金流向失败: {str(e)}"}


async def get_zt_pool(date: Optional[str] = None) -> Dict[str, Any]:
    """
    获取涨停板数据

    Args:
        date: 日期 (YYYYMMDD)

    Returns:
        涨停板数据
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        pool_result = _stock_client.get_zt_pool(date)
        _stock_client.disconnect()

        if not pool_result.success:
            return {
                "success": False,
                "error": (
                    pool_result.error.message if pool_result.error else "获取涨停板失败"
                ),
            }

        return {"success": True, "zt_pool": pool_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取涨停板失败: {str(e)}"}


async def get_dt_pool(date: Optional[str] = None) -> Dict[str, Any]:
    """
    获取跌停板数据

    Args:
        date: 日期 (YYYYMMDD)

    Returns:
        跌停板数据
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        pool_result = _stock_client.get_dt_pool(date)
        _stock_client.disconnect()

        if not pool_result.success:
            return {
                "success": False,
                "error": (
                    pool_result.error.message if pool_result.error else "获取跌停板失败"
                ),
            }

        return {"success": True, "dt_pool": pool_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取跌停板失败: {str(e)}"}


async def search_stock(keyword: str, limit: Optional[int] = None) -> Dict[str, Any]:
    """
    搜索股票

    Args:
        keyword: 搜索关键词（代码、名称、拼音）
        limit: 返回结果数量限制

    Returns:
        匹配的股票列表
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        search_result = _stock_client.search(keyword, limit)
        _stock_client.disconnect()

        if not search_result.success:
            return {
                "success": False,
                "error": (
                    search_result.error.message
                    if search_result.error
                    else "搜索股票失败"
                ),
            }

        return {"success": True, "stocks": search_result.data}

    except Exception as e:
        return {"success": False, "error": f"搜索股票失败: {str(e)}"}


async def get_stock_client_info() -> Dict[str, Any]:
    """
    获取股票客户端信息

    Returns:
        客户端信息
    """
    try:
        result = _stock_client.connect()
        if not result.success:
            return {
                "success": False,
                "error": result.error.message if result.error else "连接失败",
            }

        info_result = _stock_client.get_info()
        _stock_client.disconnect()

        if not info_result.success:
            return {
                "success": False,
                "error": (
                    info_result.error.message if info_result.error else "获取信息失败"
                ),
            }

        return {"success": True, "info": info_result.data}

    except Exception as e:
        return {"success": False, "error": f"获取客户端信息失败: {str(e)}"}


def register_tools(mcp: FastMCP) -> None:
    """注册股票工具到 MCP 服务器"""
    mcp.tool()(get_stock_list)
    mcp.tool()(get_stock_info)
    mcp.tool()(get_quote)
    mcp.tool()(get_quotes)
    mcp.tool()(get_kline)
    mcp.tool()(get_kline_hk)
    mcp.tool()(get_industry_list)
    mcp.tool()(get_concept_list)
    mcp.tool()(get_industry_stocks)
    mcp.tool()(get_concept_stocks)
    mcp.tool()(get_index_list)
    mcp.tool()(get_market_fund_flow)
    mcp.tool()(get_zt_pool)
    mcp.tool()(get_dt_pool)
    mcp.tool()(search_stock)
    mcp.tool()(get_stock_client_info)
