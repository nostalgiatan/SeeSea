#!/usr/bin/env python
"""
股票数据客户端模块

提供基于AkShare的股票数据获取功能，支持缓存和错误处理。
"""

import logging
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Any, Union
import pandas as pd
import akshare as ak

from .models import (
    StockQuote, StockKLine, StockInfo, StockBasic, StockFinancial, StockAnnouncement,
    StockFundFlow, StockValuation, StockMargin, StockLHB, StockBalanceSheet,
    StockIncomeStatement, StockCashFlow, StockShareholder, StockDividend,
    StockResearchReport, StockPledge, StockRestricted, StockResearchVisit,
    StockRepurchase, StockMarginList, PeriodType, ReportType
)

logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)


class StockDataClient:
    """股票数据客户端"""
    
    def __init__(self, cache_ttl_minutes: int = 15):
        """初始化客户端
        
        Args:
            cache_ttl_minutes: 缓存过期时间（分钟）
        """
        self._cache: Dict[str, tuple[Any, datetime]] = {}
        self._cache_ttl = timedelta(minutes=cache_ttl_minutes)
        
    def _get_cache_key(self, prefix: str, **kwargs) -> str:
        """生成缓存键"""
        key_parts = [prefix]
        for k, v in sorted(kwargs.items()):
            key_parts.append(f"{k}={v}")
        return ":".join(key_parts)
        
    def _get_from_cache(self, cache_key: str) -> Optional[Any]:
        """从缓存获取数据（集成新缓存机制）"""
        try:
            # 使用简单的内存缓存机制
            if hasattr(self, '_temp_cache') and cache_key in self._temp_cache:
                cache_entry = self._temp_cache[cache_key]
                
                # 检查TTL
                age = (datetime.now() - cache_entry['timestamp']).total_seconds()
                if age < cache_entry['ttl']:
                    logger.info(f"临时缓存获取成功: {cache_key}")
                    return cache_entry['data']
                else:
                    # 过期，删除缓存
                    del self._temp_cache[cache_key]
                    logger.info(f"临时缓存已过期: {cache_key}")
            
            logger.info(f"临时缓存未找到: {cache_key}")
            return None
        except Exception as e:
            logger.error(f"缓存获取失败 {cache_key}: {type(e).__name__}: {e}", exc_info=True)
            return None
    
    def _set_cache(self, cache_key: str, data: Any, ttl: Optional[int] = None):
        """设置缓存（集成新缓存机制）"""
        try:
            # 直接使用完整的缓存键，不强制要求冒号格式
            # 存储在内存中（使用实例变量作为临时缓存）
            if not hasattr(self, '_temp_cache'):
                self._temp_cache = {}
            
            self._temp_cache[cache_key] = {
                'data': data,
                'timestamp': datetime.now(),
                'ttl': ttl or 300  # 默认5分钟
            }
            
            logger.info(f"临时缓存设置成功: {cache_key}")
            return True
            
        except Exception as e:
            logger.error(f"缓存设置失败 {cache_key}: {type(e).__name__}: {e}", exc_info=True)
            return False
    
    def _get_period_param(self, period_type: Union[PeriodType, str]) -> str:
        """获取周期参数
        
        Args:
            period_type: 周期类型
            
        Returns:
            str: AkShare API参数
        """
        if isinstance(period_type, PeriodType):
            mapping = {
                PeriodType.DAILY: "101",
                PeriodType.WEEKLY: "102", 
                PeriodType.MONTHLY: "103"
            }
            return mapping.get(period_type, "101")
        return "101"
    
    def _get_report_type_param(self, report_type: Union[ReportType, str]) -> str:
        """获取报表类型参数
        
        Args:
            report_type: 报表类型
            
        Returns:
            str: AkShare API参数
        """
        if isinstance(report_type, ReportType):
            mapping = {
                ReportType.BALANCE_SHEET: "balance_sheet",
                ReportType.INCOME_STATEMENT: "income_statement",
                ReportType.CASH_FLOW_STATEMENT: "cash_flow_statement"
            }
            return mapping.get(report_type, "balance_sheet")
        return "balance_sheet"
    
    def _validate_code_format(self, code: str) -> bool:
        """验证股票代码格式
        
        Args:
            code: 股票代码
            
        Returns:
            bool: 是否有效
            
        Raises:
            TypeError: 当code不是字符串类型时
        """
        if not isinstance(code, str):
            if code is None:
                raise TypeError("股票代码不能为None")
            return False
            
        if not code:
            return False
        
        # 检查是否包含空格（拒绝包含空格的代码）
        if code != code.strip():
            return False
        
        # 移除空格
        code = code.strip()
        
        # 检查长度
        if len(code) != 6:
            return False
        
        # 检查是否全为数字
        if not code.isdigit():
            return False
        
        # 检查前缀是否符合A股规则
        valid_prefixes = ['000', '001', '002', '003', '300', '301', '600', '601', '603', '605', '688']
        prefix = code[:3]
        
        return prefix in valid_prefixes
        
    def _handle_df_result(self, df: pd.DataFrame) -> pd.DataFrame:
        """处理DataFrame结果"""
        logger.info(f"处理DataFrame结果，输入: {type(df)}")
        if df is None:
            logger.warning("DataFrame为None，返回空DataFrame")
            return pd.DataFrame()
        if df.empty:
            logger.warning("DataFrame为空，返回空DataFrame")
            return pd.DataFrame()
        logger.info(f"DataFrame形状: {df.shape}, 列: {df.columns.tolist()}")
        return df.copy()
    
    def _safe_float_convert(self, value: Any, field_name: str = "") -> float:
        """安全转换为浮点数
        
        Args:
            value: 需要转换的值
            field_name: 字段名称，用于错误日志
            
        Returns:
            float: 转换后的浮点数，转换失败时返回0.0
        """
        try:
            if value is None or value == "" or str(value).strip() == "":
                return 0.0
            
            # 处理pandas的NaN值
            if pd.isna(value):
                return 0.0
                
            # 尝试直接转换为float
            float_value = float(value)
            
            # 验证数值有效性
            if pd.isna(float_value) or float_value == float('inf') or float_value == float('-inf'):
                logger.warning(f"字段 {field_name} 包含无效数值: {value}")
                return 0.0
                
            return float_value
            
        except (ValueError, TypeError) as e:
            logger.warning(f"转换字段 {field_name} 失败: {value}, 错误: {e}")
            return 0.0
            
    def _safe_int_convert(self, value: Any, field_name: str = "") -> int:
        """安全转换为整数
        
        Args:
            value: 需要转换的值
            field_name: 字段名称，用于错误日志
            
        Returns:
            int: 转换后的整数，转换失败时返回0
        """
        try:
            if value is None or value == "" or str(value).strip() == "":
                return 0
                
            # 处理pandas的NaN值
            if pd.isna(value):
                return 0
                
            # 先转换为float，再转换为int
            float_value = float(value)
            
            # 验证数值有效性
            if pd.isna(float_value) or float_value == float('inf') or float_value == float('-inf'):
                logger.warning(f"字段 {field_name} 包含无效数值: {value}")
                return 0
                
            return int(float_value)
            
        except (ValueError, TypeError) as e:
            logger.warning(f"转换字段 {field_name} 失败: {value}, 错误: {e}")
            return 0
    
    async def get_realtime_quote(self, code: str) -> Optional[StockQuote]:
        """获取实时行情
        
        Args:
            code: 股票代码，格式如 "000001" 或 "000001.SZ"
            
        Returns:
            StockQuote: 股票实时行情数据，如果获取失败则返回None
        """
        try:
            # 验证股票代码格式
            if not code or not isinstance(code, str):
                logger.error(f"无效的股票代码格式: {code}")
                return None
                
            # 标准化股票代码格式
            code = code.strip().upper()
            if '.' in code:
                # 移除交易所后缀，保持纯数字格式
                code = code.split('.')[0]
                
            if not code.isdigit() or len(code) != 6:
                logger.error(f"股票代码格式错误，应为6位数字: {code}")
                return None
                
            cache_key = self._get_cache_key("quote", code=code)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取实时行情数据
            df = ak.stock_zh_a_spot_em()
            df = self._handle_df_result(df)
            
            if df.empty:
                logger.warning(f"获取实时行情数据为空")
                return None
                
            # 验证必要字段存在
            required_fields = ['代码', '名称', '最新价']
            missing_fields = [field for field in required_fields if field not in df.columns]
            if missing_fields:
                logger.error(f"实时行情数据缺少必要字段: {missing_fields}")
                return None
                
            # 查找指定股票
            stock_df = df[df['代码'] == code]
            if stock_df.empty:
                logger.warning(f"未找到股票代码 {code} 的实时行情数据")
                return None
                
            row = stock_df.iloc[0]
            
            # 构建行情数据对象
            quote = StockQuote(
                code=code,
                name=str(row['名称']) if pd.notna(row.get('名称')) else code,
                price=self._safe_float_convert(row.get('最新价'), '最新价'),
                change=self._safe_float_convert(row.get('涨跌额'), '涨跌额'),
                change_pct=self._safe_float_convert(row.get('涨跌幅'), '涨跌幅'),
                volume=self._safe_int_convert(row.get('成交量'), '成交量'),
                amount=self._safe_float_convert(row.get('成交额'), '成交额'),
                open=self._safe_float_convert(row.get('今开'), '今开'),
                high=self._safe_float_convert(row.get('最高'), '最高'),
                low=self._safe_float_convert(row.get('最低'), '最低'),
                close=self._safe_float_convert(row.get('最新价'), '最新价'),  # 使用最新价作为收盘价
                prev_close=self._safe_float_convert(row.get('昨收'), '昨收'),
                time=datetime.now()
            )
            
            # 验证关键数据有效性
            if quote.price <= 0:
                logger.error(f"股票价格数据异常: {quote.price}")
                return None
                
            self._set_cache(cache_key, quote)
            logger.info(f"成功获取股票 {code} 实时行情数据")
            return quote
            
        except pd.errors.EmptyDataError as e:
            logger.error(f"实时行情数据解析失败 {code}: {e}")
            return None
        except KeyError as e:
            logger.error(f"实时行情数据字段缺失 {code}: {e}")
            return None
        except Exception as e:
            logger.error(f"获取实时行情失败 {code}: {e}", exc_info=True)
            return None
    
    async def get_realtime_quotes_batch(self, codes: List[str]) -> List[StockQuote]:
        """批量获取实时行情数据
        
        Args:
            codes: 股票代码列表，格式如 ["000001", "000002"]
            
        Returns:
            List[StockQuote]: 股票实时行情数据列表，获取失败的股票将不包含在结果中
        """
        if not codes:
            return []
            
        # 标准化股票代码格式
        normalized_codes = []
        for code in codes:
            if not code or not isinstance(code, str):
                continue
                
            code = code.strip().upper()
            if '.' in code:
                code = code.split('.')[0]
                
            if code.isdigit() and len(code) == 6:
                normalized_codes.append(code)
        
        if not normalized_codes:
            logger.warning("没有有效的股票代码")
            return []
        
        try:
            # 先检查缓存，筛选出需要实时获取的股票
            quotes_to_fetch = []
            cached_quotes = {}
            
            for code in normalized_codes:
                cache_key = self._get_cache_key("quote", code=code)
                cached = self._get_from_cache(cache_key)
                if cached:
                    cached_quotes[code] = cached
                else:
                    quotes_to_fetch.append(code)
            
            # 如果需要获取实时数据
            if quotes_to_fetch:
                logger.info(f"批量获取 {len(quotes_to_fetch)} 只股票的实时行情数据")
                
                # 获取所有实时行情数据
                df = ak.stock_zh_a_spot_em()
                df = self._handle_df_result(df)
                
                if df.empty:
                    logger.warning("获取实时行情数据为空")
                    return list(cached_quotes.values())
                
                # 验证必要字段存在
                required_fields = ['代码', '名称', '最新价']
                missing_fields = [field for field in required_fields if field not in df.columns]
                if missing_fields:
                    logger.error(f"实时行情数据缺少必要字段: {missing_fields}")
                    return list(cached_quotes.values())
                
                # 处理需要获取的股票
                for code in quotes_to_fetch:
                    stock_df = df[df['代码'] == code]
                    if stock_df.empty:
                        logger.warning(f"未找到股票代码 {code} 的实时行情数据")
                        continue
                    
                    row = stock_df.iloc[0]
                    
                    # 构建行情数据对象
                    quote = StockQuote(
                        code=code,
                        name=str(row['名称']) if pd.notna(row.get('名称')) else code,
                        price=self._safe_float_convert(row.get('最新价'), '最新价'),
                        change=self._safe_float_convert(row.get('涨跌额'), '涨跌额'),
                        change_pct=self._safe_float_convert(row.get('涨跌幅'), '涨跌幅'),
                        volume=self._safe_int_convert(row.get('成交量'), '成交量'),
                        amount=self._safe_float_convert(row.get('成交额'), '成交额'),
                        open=self._safe_float_convert(row.get('今开'), '今开'),
                        high=self._safe_float_convert(row.get('最高'), '最高'),
                        low=self._safe_float_convert(row.get('最低'), '最低'),
                        close=self._safe_float_convert(row.get('昨收'), '昨收'),
                        prev_close=self._safe_float_convert(row.get('昨收'), '昨收'),
                        time=datetime.now()
                    )
                    
                    # 验证关键数据有效性
                    if quote.price <= 0:
                        logger.error(f"股票价格数据异常: {quote.price}")
                        continue
                        
                    # 添加到缓存
                    cache_key = self._get_cache_key("quote", code=code)
                    self._set_cache(cache_key, quote)
                    cached_quotes[code] = quote
            
            # 返回所有股票的行情数据（保持原始顺序）
            result = []
            for code in normalized_codes:
                if code in cached_quotes:
                    result.append(cached_quotes[code])
            
            logger.info(f"成功获取 {len(result)} 只股票的实时行情数据")
            return result
            
        except Exception as e:
            logger.error(f"批量获取实时行情数据失败: {e}")
            return list(cached_quotes.values())
        
    async def get_kline_data(
        self, 
        code: str, 
        period: PeriodType = PeriodType.DAILY,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        adjust: str = "",
        limit: Optional[int] = None
    ) -> List[StockKLine]:
        """获取K线数据
        
        Args:
            code: 股票代码
            period: 周期类型 (DAILY, WEEKLY, MONTHLY)
            start_date: 开始日期，格式为YYYYMMDD，默认为90天前
            end_date: 结束日期，格式为YYYYMMDD，默认为今天
            adjust: 复权类型 ("": 不复权, "qfq": 前复权, "hfq": 后复权)
            limit: 返回数据条数限制，如果指定则优先使用此参数
        """
        try:
            # 默认获取最近90天数据，如果指定了limit则根据limit调整
            if limit:
                # 如果指定了limit，获取最近limit天的数据
                end_date = datetime.now().strftime('%Y%m%d')
                start_date = (datetime.now() - timedelta(days=limit)).strftime('%Y%m%d')
            else:
                # 默认获取最近90天数据
                if not end_date:
                    end_date = datetime.now().strftime('%Y%m%d')
                if not start_date:
                    start_date = (datetime.now() - timedelta(days=90)).strftime('%Y%m%d')
                
            cache_key = self._get_cache_key("kline", code=code, period=period.value, 
                                          start=start_date, end=end_date, adjust=adjust)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取K线数据
            period_map = {
                PeriodType.DAILY: 'daily',
                PeriodType.WEEKLY: 'weekly', 
                PeriodType.MONTHLY: 'monthly'
            }
            
            df = ak.stock_zh_a_hist(symbol=code, period=period_map[period], 
                                  start_date=start_date, end_date=end_date, adjust=adjust)
            df = self._handle_df_result(df)
            
            if df.empty:
                return []
                
            # 验证必要字段存在
            required_fields = ['日期', '开盘', '收盘', '最高', '最低', '成交量']
            missing_fields = [field for field in required_fields if field not in df.columns]
            if missing_fields:
                logger.error(f"K线数据缺少必要字段: {missing_fields}")
                return []
                
            klines = []
            for _, row in df.iterrows():
                try:
                    kline = StockKLine(
                        date=pd.to_datetime(row['日期']).date(),
                        open=self._safe_float_convert(row['开盘'], '开盘'),
                        high=self._safe_float_convert(row['最高'], '最高'),
                        low=self._safe_float_convert(row['最低'], '最低'),
                        close=self._safe_float_convert(row['收盘'], '收盘'),
                        volume=self._safe_int_convert(row['成交量'], '成交量'),
                        amount=self._safe_float_convert(row.get('成交额'), '成交额'),
                        change=self._safe_float_convert(row.get('涨跌额'), '涨跌额'),
                        change_pct=self._safe_float_convert(row.get('涨跌幅'), '涨跌幅')
                    )
                    klines.append(kline)
                except Exception as e:
                    logger.warning(f"处理K线数据失败 {code}: {e}")
                    continue
            
            self._set_cache(cache_key, klines)
            logger.info(f"成功获取股票 {code} 的 {len(klines)} 条K线数据")
            return klines
            
        except pd.errors.EmptyDataError as e:
            logger.error(f"K线数据解析失败 {code}: {e}")
            return []
        except KeyError as e:
            logger.error(f"K线数据字段缺失 {code}: {e}")
            return []
        except Exception as e:
            logger.error(f"获取K线数据失败 {code}: {e}", exc_info=True)
            return []
    
    async def get_index_data(self, symbol: str, period: PeriodType = PeriodType.DAILY,
                           start_date: Optional[str] = None, end_date: Optional[str] = None) -> List[StockKLine]:
        """获取指数数据
        
        Args:
            symbol: 指数代码，如 "000001" (上证指数)
            period: 周期类型 (DAILY, WEEKLY, MONTHLY)
            start_date: 开始日期，格式为YYYYMMDD，默认为90天前
            end_date: 结束日期，格式为YYYYMMDD，默认为今天
            
        Returns:
            List[StockKLine]: 指数K线数据列表
        """
        try:
            # 默认获取最近90天数据
            if not end_date:
                end_date = datetime.now().strftime('%Y%m%d')
            if not start_date:
                start_date = (datetime.now() - timedelta(days=90)).strftime('%Y%m%d')
                
            cache_key = self._get_cache_key("index", symbol=symbol, period=period.value, 
                                          start=start_date, end=end_date)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取指数数据
            period_map = {
                PeriodType.DAILY: 'daily',
                PeriodType.WEEKLY: 'weekly', 
                PeriodType.MONTHLY: 'monthly'
            }
            
            df = ak.index_zh_a_hist(symbol=symbol, period=period_map[period], 
                                  start_date=start_date, end_date=end_date)
            df = self._handle_df_result(df)
            
            if df.empty:
                logger.warning(f"获取指数 {symbol} 数据为空")
                return []
                
            # 验证必要字段存在
            required_fields = ['日期', '开盘', '收盘', '最高', '最低', '成交量']
            missing_fields = [field for field in required_fields if field not in df.columns]
            if missing_fields:
                logger.error(f"指数数据缺少必要字段: {missing_fields}")
                return []
                
            klines = []
            for _, row in df.iterrows():
                try:
                    kline = StockKLine(
                        date=pd.to_datetime(row['日期']),
                        open=self._safe_float_convert(row['开盘'], '开盘'),
                        high=self._safe_float_convert(row['最高'], '最高'),
                        low=self._safe_float_convert(row['最低'], '最低'),
                        close=self._safe_float_convert(row['收盘'], '收盘'),
                        volume=self._safe_int_convert(row['成交量'], '成交量'),
                        amount=self._safe_float_convert(row.get('成交额'), '成交额'),
                        change=self._safe_float_convert(row.get('涨跌额'), '涨跌额'),
                        change_pct=self._safe_float_convert(row.get('涨跌幅'), '涨跌幅')
                    )
                    klines.append(kline)
                except Exception as e:
                    logger.warning(f"处理指数数据失败 {symbol}: {e}")
                    continue
            
            self._set_cache(cache_key, klines)
            logger.info(f"成功获取指数 {symbol} 的 {len(klines)} 条数据")
            return klines
            
        except Exception as e:
            logger.error(f"获取指数数据失败 {symbol}: {e}", exc_info=True)
            return []
    
    async def get_stock_basic(self, code: str) -> Optional[StockBasic]:
        """获取股票基本信息
        
        Args:
            code: 股票代码，格式如 "000001"
            
        Returns:
            StockBasic: 股票基本信息，如果获取失败则返回None
        """
        try:
            # 验证股票代码格式
            if not code or not isinstance(code, str):
                logger.error(f"无效的股票代码格式: {code}")
                return None
                
            # 标准化股票代码格式
            code = code.strip().upper()
            if '.' in code:
                code = code.split('.')[0]
                
            if not code.isdigit() or len(code) != 6:
                logger.error(f"股票代码格式错误，应为6位数字: {code}")
                return None
                
            cache_key = self._get_cache_key("basic", code=code)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取股票基本信息
            df = ak.stock_individual_info_em(symbol=code)
            df = self._handle_df_result(df)
            
            if df.empty:
                logger.warning(f"获取股票 {code} 基本信息为空")
                return None
                
            # 验证必要字段存在
            required_fields = ['股票简称']
            missing_fields = [field for field in required_fields if field not in df.columns]
            if missing_fields:
                logger.error(f"股票基本信息缺少必要字段: {missing_fields}")
                return None
                
            row = df.iloc[0]
            
            def safe_float_convert(value):
                try:
                    return float(value) if value else 0.0
                except (ValueError, TypeError):
                    return 0.0
            
            def safe_date_convert(date_str):
                try:
                    return pd.to_datetime(date_str)
                except (ValueError, TypeError):
                    return datetime.now()
            
            basic = StockBasic(
                code=code,
                name=str(row.get('股票简称', code)),
                industry=str(row.get('所属行业', '')),
                area=str(row.get('地区', '')),
                pe=safe_float_convert(row.get('市盈率(TTM)')),
                pb=safe_float_convert(row.get('市净率')),
                total_share=safe_float_convert(row.get('总股本')),
                float_share=safe_float_convert(row.get('流通股')),
                total_assets=safe_float_convert(row.get('总资产')),
                liquid_assets=safe_float_convert(row.get('流动资产')),
                fixed_assets=safe_float_convert(row.get('固定资产')),
                reserved=safe_float_convert(row.get('公积金')),
                reserved_per_share=safe_float_convert(row.get('每股公积金')),
                eps=safe_float_convert(row.get('每股收益')),
                bvps=safe_float_convert(row.get('每股净资产')),
                listing_date=safe_date_convert(row.get('上市日期'))
            )
            
            self._set_cache(cache_key, basic)
            return basic
            
        except Exception as e:
            logger.error(f"获取股票基本信息失败 {code}: {e}")
            return None
    
    async def get_announcements(self, code: str, date_str: str) -> List[StockAnnouncement]:
        """获取公告信息"""
        try:
            cache_key = self._get_cache_key("announcement", code=code, date=date_str)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取公告数据
            df = ak.stock_announcement(symbol=code, date=date_str)
            df = self._handle_df_result(df)
            
            announcements = []
            for _, row in df.iterrows():
                announcement = StockAnnouncement(
                    code=code,
                    name=str(row.get('股票名称', code)),
                    title=str(row.get('标题', '')),
                    type=str(row.get('公告类型', '')),
                    date=pd.to_datetime(row.get('公告日期', datetime.now())),
                    url=str(row.get('公告链接', ''))
                )
                announcements.append(announcement)
            
            self._set_cache(cache_key, announcements)
            return announcements
            
        except Exception as e:
            logger.error(f"获取公告失败 {code}: {e}")
            return []
    
    async def get_fund_flow(self, code: str) -> Optional[StockFundFlow]:
        """获取资金流向"""
        try:
            cache_key = self._get_cache_key("fund_flow", code=code)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取资金流向数据
            df = ak.stock_individual_fund_flow(symbol=code)
            df = self._handle_df_result(df)
            
            if df.empty:
                return None
                
            row = df.iloc[0]
            
            def safe_float(value):
                try:
                    return float(value) if value else 0.0
                except (ValueError, TypeError):
                    return 0.0
            
            fund_flow = StockFundFlow(
                code=code,
                name=str(row.get('股票简称', code)),
                date=pd.to_datetime(row.get('日期', datetime.now())),
                main_inflow=safe_float(row.get('主力净流入-净额')),
                main_outflow=safe_float(row.get('主力净流入-净占比')),
                net_inflow=safe_float(row.get('净额')),
                small_inflow=safe_float(row.get('小单净流入')),
                small_outflow=safe_float(row.get('小单净流出')),
                net_small_inflow=safe_float(row.get('小单净额')),
                large_inflow=safe_float(row.get('大单净流入')),
                large_outflow=safe_float(row.get('大单净流出'))
            )
            
            self._set_cache(cache_key, fund_flow)
            return fund_flow
            
        except Exception as e:
            logger.error(f"获取资金流向失败 {code}: {e}")
            return None
    
    async def get_batch_quotes(self, codes: List[str]) -> Dict[str, StockQuote]:
        """批量获取实时行情（使用优化的批量处理方法）
        
        Args:
            codes: 股票代码列表，格式如 ["000001", "000002"]
            
        Returns:
            Dict[str, StockQuote]: 股票代码到行情数据的映射，获取失败的股票将不包含在结果中
        """
        # 使用新的批量处理方法
        quotes = await self.get_realtime_quotes_batch(codes)
        
        # 转换为字典格式
        result = {}
        for quote in quotes:
            result[quote.code] = quote
            
        return result
    
    async def get_stock_valuation(self, code: str) -> Optional[StockValuation]:
        """获取股票估值分析"""
        try:
            cache_key = self._get_cache_key("valuation", code=code)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取估值分析数据
            df = ak.stock_value_em(symbol=code)
            df = self._handle_df_result(df)
            
            if df.empty:
                return None
                
            row = df.iloc[0]
            
            def safe_float(value, default=0.0):
                try:
                    return float(value) if value else default
                except (ValueError, TypeError):
                    return default
            
            valuation = StockValuation(
                code=code,
                name=str(row.get('股票简称', code)),
                date=pd.to_datetime(row.get('数据日期', datetime.now())),
                close_price=safe_float(row.get('当日收盘价')),
                change_rate=safe_float(row.get('当日涨跌幅')),
                total_market_cap=safe_float(row.get('总市值')),
                float_market_cap=safe_float(row.get('流通市值')),
                total_shares=safe_float(row.get('总股本')),
                float_shares=safe_float(row.get('流通股本')),
                pe_ttm=safe_float(row.get('PE(TTM)')),
                pe_static=safe_float(row.get('PE(静)')),
                pb=safe_float(row.get('市净率'))
            )
            
            self._set_cache(cache_key, valuation)
            return valuation
            
        except Exception as e:
            logger.error(f"获取股票估值分析失败 {code}: {e}")
            return None
    
    async def get_margin_data(self, code: str) -> Optional[StockMargin]:
        """获取融资融券数据"""
        try:
            cache_key = self._get_cache_key("margin", code=code)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取融资融券数据
            df = ak.stock_margin_detail_sse(symbol=code)
            df = self._handle_df_result(df)
            
            if df.empty:
                # 如果没有数据，尝试获取最近的历史数据
                df = ak.stock_margin_history_em(symbol=code)
                df = self._handle_df_result(df)
                if df.empty:
                    return None
            
            # 获取最新数据
            latest_row = df.iloc[0]
            
            def safe_float(value):
                try:
                    return float(value) if value else 0.0
                except (ValueError, TypeError):
                    return 0.0
            
            margin = StockMargin(
                code=code,
                name=str(latest_row.get('股票名称', code)),
                date=pd.to_datetime(latest_row.get('交易日期', datetime.now())),
                margin_buy=safe_float(latest_row.get('融资买入额')),
                margin_sell=safe_float(latest_row.get('融资偿还额')),
                margin_balance=safe_float(latest_row.get('融资余额')),
                margin_ratio=safe_float(latest_row.get('融资余额/流通市值'))
            )
            
            self._set_cache(cache_key, margin)
            return margin
            
        except Exception as e:
            logger.error(f"获取融资融券数据失败 {code}: {e}")
            return None
    
    async def get_lhb_data(self, code: str, start_date: str, end_date: str) -> List[StockLHB]:
        """获取龙虎榜数据"""
        try:
            cache_key = self._get_cache_key("lhb", code=code, start=start_date, end=end_date)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取龙虎榜数据
            df = ak.stock_lhb_detail_em(symbol=code, start_date=start_date, end_date=end_date)
            df = self._handle_df_result(df)
            
            if df.empty:
                return []
                
            lhb_list = []
            for _, row in df.iterrows():
                def safe_float(value, default=0.0):
                    try:
                        return float(value) if value else default
                    except (ValueError, TypeError):
                        return default
                
                lhb = StockLHB(
                    code=code,
                    name=str(row.get('股票简称', code)),
                    date=pd.to_datetime(row.get('上榜日期', datetime.now())),
                    close_price=safe_float(row.get('收盘价')),
                    change_pct=safe_float(row.get('涨跌幅')),
                    turnover_rate=safe_float(row.get('换手率')),
                    amount=safe_float(row.get('总成交金额')),
                    lhb_net_buy=safe_float(row.get('龙虎榜净买入额')),
                    lhb_buy_amount=safe_float(row.get('龙虎榜买入额')),
                    lhb_sell_amount=safe_float(row.get('龙虎榜卖出额')),
                    lhb_total_amount=safe_float(row.get('龙虎榜成交额')),
                    reason=str(row.get('上榜原因', ''))
                )
                lhb_list.append(lhb)
            
            self._set_cache(cache_key, lhb_list)
            return lhb_list
            
        except Exception as e:
            logger.error(f"获取龙虎榜数据失败 {code}: {e}")
            return []
    
    async def get_balance_sheet(self, code: str) -> Optional[StockBalanceSheet]:
        """获取资产负债表"""
        try:
            cache_key = self._get_cache_key("balance", code=code)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取资产负债表数据
            df = ak.stock_financial_report_sina(symbol=code, symbol_type="资产负债表")
            df = self._handle_df_result(df)
            
            if df.empty:
                return None
                
            # 获取最新报表
            df = df.sort_values('report_date', ascending=False)
            row = df.iloc[0]
            
            def safe_float(value):
                try:
                    return float(value) if value else 0.0
                except (ValueError, TypeError):
                    return 0.0
            
            balance = StockBalanceSheet(
                code=code,
                name=str(row.get('股票简称', code)),
                report_date=pd.to_datetime(row.get('report_date')),
                total_assets=safe_float(row.get('资产总计')),
                total_liability=safe_float(row.get('负债合计')),
                total_equity=safe_float(row.get('股东权益合计')),
                current_assets=safe_float(row.get('流动资产合计')),
                current_liability=safe_float(row.get('流动负债合计')),
                inventory=safe_float(row.get('存货')),
                accounts_receivable=safe_float(row.get('应收账款')),
                cash_and_equivalents=safe_float(row.get('货币资金'))
            )
            
            self._set_cache(cache_key, balance)
            return balance
            
        except Exception as e:
            logger.error(f"获取资产负债表失败 {code}: {e}")
            return None
    
    async def get_income_statement(self, code: str) -> Optional[StockIncomeStatement]:
        """获取利润表"""
        try:
            cache_key = self._get_cache_key("income", code=code)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取利润表数据
            df = ak.stock_financial_report_sina(symbol=code, symbol_type="利润表")
            df = self._handle_df_result(df)
            
            if df.empty:
                return None
                
            # 获取最新报表
            df = df.sort_values('report_date', ascending=False)
            row = df.iloc[0]
            
            def safe_float(value):
                try:
                    return float(value) if value else 0.0
                except (ValueError, TypeError):
                    return 0.0
            
            income = StockIncomeStatement(
                code=code,
                name=str(row.get('股票简称', code)),
                report_date=pd.to_datetime(row.get('report_date')),
                total_revenue=safe_float(row.get('营业总收入')),
                operating_revenue=safe_float(row.get('营业收入')),
                operating_cost=safe_float(row.get('营业成本')),
                operating_profit=safe_float(row.get('营业利润')),
                total_profit=safe_float(row.get('利润总额')),
                net_profit=safe_float(row.get('净利润')),
                eps=safe_float(row.get('基本每股收益'))
            )
            
            self._set_cache(cache_key, income)
            return income
            
        except Exception as e:
            logger.error(f"获取利润表失败 {code}: {e}")
            return None
    
    async def get_cash_flow(self, code: str) -> Optional[StockCashFlow]:
        """获取现金流量表"""
        try:
            cache_key = self._get_cache_key("cashflow", code=code)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取现金流量表数据
            df = ak.stock_financial_report_sina(symbol=code, symbol_type="现金流量表")
            df = self._handle_df_result(df)
            
            if df.empty:
                return None
                
            # 获取最新报表
            df = df.sort_values('report_date', ascending=False)
            row = df.iloc[0]
            
            def safe_float(value):
                try:
                    return float(value) if value else 0.0
                except (ValueError, TypeError):
                    return 0.0
            
            cash_flow = StockCashFlow(
                code=code,
                name=str(row.get('股票简称', code)),
                report_date=pd.to_datetime(row.get('report_date')),
                operating_cash_flow=safe_float(row.get('经营活动产生的现金流量净额')),
                investing_cash_flow=safe_float(row.get('投资活动产生的现金流量净额')),
                financing_cash_flow=safe_float(row.get('筹资活动产生的现金流量净额')),
                net_cash_flow=safe_float(row.get('现金及现金等价物净增加额')),
                cash_at_end=safe_float(row.get('期末现金及现金等价物余额'))
            )
            
            self._set_cache(cache_key, cash_flow)
            return cash_flow
            
        except Exception as e:
            logger.error(f"获取现金流量表失败 {code}: {e}")
            return None
    
    async def get_shareholder_data(self, code: str) -> Optional[StockShareholder]:
        """获取股东户数数据"""
        try:
            cache_key = self._get_cache_key("shareholder", code=code)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取股东户数数据
            df = ak.stock_gdfh_em()
            df = self._handle_df_result(df)
            
            if df.empty:
                return None
                
            # 筛选指定股票的数据
            stock_data = df[df['代码'] == code]
            if stock_data.empty:
                return None
                
            row = stock_data.iloc[0]
            
            def safe_int(value, default=0):
                try:
                    return int(value) if value else default
                except (ValueError, TypeError):
                    return default
            
            def safe_float(value, default=0.0):
                try:
                    return float(value) if value else default
                except (ValueError, TypeError):
                    return default
            
            shareholder = StockShareholder(
                code=code,
                name=str(row.get('名称', code)),
                end_date=pd.to_datetime(row.get('统计日期', datetime.now())),
                holder_num=safe_int(row.get('股东户数')),
                holder_num_change=safe_int(row.get('较上期变动')),
                holder_num_ratio=safe_float(row.get('户均持股')),
                avg_hold_num=safe_float(row.get('户均持股')),
                avg_market_cap=safe_float(row.get('户均持股市值'))
            )
            
            self._set_cache(cache_key, shareholder)
            return shareholder
            
        except Exception as e:
            logger.error(f"获取股东户数数据失败 {code}: {e}")
            return None
    
    async def get_dividend_data(self, code: str) -> Optional[StockDividend]:
        """"获取分红送配数据"""""
        try:
            cache_key = self._get_cache_key("dividend", code=code)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取分红送配数据
            df = ak.stock_dividend_cninfo(symbol=code)
            df = self._handle_df_result(df)
            
            if df.empty:
                return None
                
            # 筛选指定股票的数据
            stock_data = df[df['证券代码'] == code]
            if stock_data.empty:
                return None
                
            row = stock_data.iloc[0]
            
            def safe_float(value, default=0.0):
                try:
                    return float(value) if value else default
                except (ValueError, TypeError):
                    return default
            
            dividend = StockDividend(
                code=code,
                name=str(row.get('证券简称', code)),
                report_date=pd.to_datetime(row.get('最新公告日期', datetime.now())),
                dividend_per_share=safe_float(row.get('分红(元)')),
                bonus_per_share=safe_float(row.get('送转(元)')),
                transfer_per_share=safe_float(row.get('公积金转增(股)')),
                total_dividend=safe_float(row.get('总分红额')),
                dividend_date=pd.to_datetime(row.get('除权除息日', datetime.now())),
                equity_record_date=pd.to_datetime(row.get('股权登记日', datetime.now())),
                ex_dividend_date=pd.to_datetime(row.get('除息日', datetime.now()))
            )
            
            self._set_cache(cache_key, dividend)
            return dividend
            
        except Exception as e:
            logger.error(f"获取分红送配数据失败 {code}: {e}")
            return None
    
    async def get_research_report(self, code: str, limit: int = 10) -> List[StockResearchReport]:
        """获取研报数据"""
        try:
            cache_key = self._get_cache_key("research", code=code, limit=limit)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                    
            # 获取研报数据
            df = ak.stock_research_report_em(symbol=code, limit=limit)
            df = self._handle_df_result(df)
            
            if df.empty:
                return []
                    
            data = df[df['股票代码'] == code]
            if data.empty:
                return []
                
            reports = []
            for _, row in data.iterrows():
                def safe_float(value, default=0.0):
                    try:
                        return float(value) if value else default
                    except (ValueError, TypeError):
                        return default
                
                report = StockResearchReport(
                    code=code,
                    name=str(row.get('股票简称', code)),
                    report_date=pd.to_datetime(row.get('发布日期', datetime.now())),
                    title=str(row.get('标题', '')),
                    author=str(row.get('机构', '')),
                    organization=str(row.get('分析师')),
                    rating=str(row.get('评级', '')),
                    rating_change=str(row.get('评级变动', '')),
                    target_price=safe_float(row.get('目标价')),
                    content=str(row.get('研报摘要', ''))
                )
                reports.append(report)
            
            self._set_cache(cache_key, reports)
            return reports
            
        except Exception as e:
            logger.error(f"获取研报数据失败 {code}: {e}")
            return []
    
    async def get_pledge_data(self, code: str) -> List[StockPledge]:
        """"获取股权质押数据"""
        try:
            cache_key = self._get_cache_key("pledge", code=code)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取股权质押数据
            df = ak.stock_gpzy_profile_em()
            df = self._handle_df_result(df)
            
            if df.empty:
                return []
                
            # 筛选指定股票的数据
            stock_data = df[df['股票代码'] == code]
            if stock_data.empty:
                return []
                
            pledges = []
            for _, row in stock_data.iterrows():
                def safe_float(value, default=0.0):
                    try:
                        return float(value) if value else default
                    except (ValueError, TypeError):
                        return default
                
                pledge = StockPledge(
                    code=code,
                    name=str(row.get('股票简称', code)),
                    pledge_date=pd.to_datetime(row.get('质押日期', datetime.now())),
                    pledgor=str(row.get('出质人', '')),
                    pledgee=str(row.get('质权人', '')),
                    pledge_num=safe_float(row.get('质押数量')),
                    pledge_ratio=safe_float(row.get('质押比例')),
                    pledge_amount=safe_float(row.get('质押金额')),
                    start_date=pd.to_datetime(row.get('质押开始日期', datetime.now())),
                    end_date=pd.to_datetime(row.get('质押结束日期', datetime.now())),
                    status=str(row.get('质押状态', ''))
                )
                pledges.append(pledge)
            
            self._set_cache(cache_key, pledges)
            return pledges
            
        except Exception as e:
            logger.error(f"获取股权质押数据失败 {code}: {e}")
            return []
    
    async def get_restricted_data(self, code: str, start_date: str, end_date: str) -> List[StockRestricted]:
        """获取限售股解禁数据"""
        try:
            cache_key = self._get_cache_key("restricted", code=code, start=start_date, end=end_date)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                    
            # 获取限售股解禁数据
            df = ak.stock_restricted_release_summary_em(symbol="全部股票", start_date=start_date, end_date=end_date)
            df = self._handle_df_result(df)
            
            if df.empty:
                return []
                
            # 筛选指定股票的数据
            stock_data = df[df['股票代码'] == code]
            if stock_data.empty:
                return []
                    
            
            restricted_list = []
            for _, row in stock_data.iterrows():
                def safe_float(value, default=0.0):
                    try:
                        return float(value) if value else default
                    except (ValueError, TypeError):
                        return default
                
                restricted = StockRestricted(
                    code=code,
                    name=str(row.get('股票简称', code)),
                    release_date=pd.to_datetime(row.get('解禁日期', datetime.now())),
                    release_num=safe_float(row.get('解禁数量')),
                    release_ratio=safe_float(row.get('解禁比例')),
                    release_market_cap=safe_float(row.get('解禁市值')),
                    shareholder=str(row.get('股东名称', '')),
                    shareholder_type=str(row.get('股东类型', '')),
                    lock_reason=str(row.get('限售原因', ''))
                )
                restricted_list.append(restricted)
            
            self._set_cache(cache_key, restricted_list)
            return restricted_list
            
        except Exception as e:
            logger.error(f"获取限售股解禁数据失败 {code}: {e}")
            return []
    
    async def get_stock_list(self) -> List[StockBasic]:
        """获取A股股票列表
        
        Returns:
            List[StockBasic]: A股股票基本信息列表
        """
        logger.info("开始获取A股股票列表")
        
        try:
            cache_key = self._get_cache_key("stock_list")
            logger.info(f"缓存键: {cache_key}")
            
            cached = self._get_from_cache(cache_key)
            if cached:
                logger.info(f"从缓存获取到 {len(cached)} 只股票信息")
                return cached
            else:
                logger.info("缓存中没有股票列表数据")
                
            # 获取A股股票列表
            logger.info("调用 ak.stock_zh_a_spot_em() 获取股票列表")
            df = ak.stock_zh_a_spot_em()
            logger.info(f"ak.stock_zh_a_spot_em() 返回数据类型: {type(df)}")
            
            if df is not None:
                logger.info(f"原始数据形状: {df.shape if hasattr(df, 'shape') else '无shape属性'}")
                logger.info(f"原始数据列: {df.columns.tolist() if hasattr(df, 'columns') else '无columns属性'}")
                logger.info(f"原始数据前几行:\n{df.head() if hasattr(df, 'head') else '无head方法'}")
            else:
                logger.warning("ak.stock_zh_a_spot_em() 返回 None")
            
            df = self._handle_df_result(df)
            
            if df.empty:
                logger.warning("获取A股股票列表为空")
                return []
                
            # 验证必要字段存在
            required_fields = ['代码', '名称']
            missing_fields = [field for field in required_fields if field not in df.columns]
            if missing_fields:
                logger.error(f"股票列表数据缺少必要字段: {missing_fields}")
                logger.error(f"实际字段: {df.columns.tolist()}")
                return []
                
            stock_list = []
            logger.info(f"开始处理 {len(df)} 条股票数据")
            for _, row in df.iterrows():
                try:
                    stock = StockBasic(
                        code=str(row['代码']),
                        name=str(row['名称']),
                        industry=str(row.get('所属行业', '')),
                        area=str(row.get('地区', '')),
                        pe=self._safe_float_convert(row.get('市盈率'), '市盈率'),
                        pb=self._safe_float_convert(row.get('市净率'), '市净率'),
                        total_share=self._safe_float_convert(row.get('总股本'), '总股本'),
                        float_share=self._safe_float_convert(row.get('流通股'), '流通股'),
                        total_assets=self._safe_float_convert(row.get('总资产'), '总资产'),
                        liquid_assets=self._safe_float_convert(row.get('流动资产'), '流动资产'),
                        fixed_assets=self._safe_float_convert(row.get('固定资产'), '固定资产'),
                        reserved=self._safe_float_convert(row.get('公积金'), '公积金'),
                        reserved_per_share=self._safe_float_convert(row.get('每股公积金'), '每股公积金'),
                        eps=self._safe_float_convert(row.get('每股收益'), '每股收益'),
                        bvps=self._safe_float_convert(row.get('每股净资产'), '每股净资产'),
                        listing_date=pd.to_datetime(row.get('上市时间', datetime.now()))
                    )
                    stock_list.append(stock)
                    logger.debug(f"成功处理股票: {stock.code} - {stock.name}")
                except Exception as e:
                    logger.warning(f"处理股票数据失败 {row.get('代码', 'unknown')}: {e}")
                    continue
            
            logger.info(f"成功处理 {len(stock_list)} 只股票数据")
            self._set_cache(cache_key, stock_list)
            logger.info(f"已将 {len(stock_list)} 只股票信息写入缓存")
            return stock_list
            
        except Exception as e:
            logger.error(f"获取A股股票列表失败: {e}", exc_info=True)
            return []
    
    async def get_stock_info(self, code: str) -> Optional[StockInfo]:
        """获取股票信息
        
        Args:
            code: 股票代码，格式如 "000001" 或 "000001.SZ"
            
        Returns:
            StockInfo: 股票信息数据，如果获取失败则返回None
        """
        try:
            # 验证股票代码格式
            if not code or not isinstance(code, str):
                logger.error(f"无效的股票代码格式: {code}")
                return None
                
            # 标准化股票代码格式
            code = code.strip().upper()
            if '.' in code:
                # 移除交易所后缀，保持纯数字格式
                code = code.split('.')[0]
                
            cache_key = self._get_cache_key("stock_info", code=code)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取个股信息
            df = ak.stock_individual_info_em(symbol=code)
            df = self._handle_df_result(df)
            
            if df.empty:
                logger.warning(f"获取股票信息为空: {code}")
                return None
                
            # 验证必要字段存在
            if 'item' not in df.columns or 'value' not in df.columns:
                logger.error(f"股票信息数据格式错误: {df.columns.tolist()}")
                return None
                
            # 创建字段映射
            field_mapping = {
                '总市值': 'total_market_cap',
                '流通市值': 'circulation_market_cap', 
                '行业': 'industry',
                '市盈率': 'pe_ratio',
                '市净率': 'pb_ratio',
                '股息率': 'dividend_rate'
            }
            
            # 从数据中提取字段值
            info_dict = {}
            for _, row in df.iterrows():
                item = str(row.get('item', ''))
                value = str(row.get('value', ''))
                if item in field_mapping:
                    info_dict[field_mapping[item]] = value
            
            # 获取股票名称
            name = info_dict.get('name', '')
            if not name:
                # 尝试从其他字段获取名称
                for _, row in df.iterrows():
                    if str(row.get('item', '')) == '股票名称':
                        name = str(row.get('value', ''))
                        break
            
            # 创建StockInfo对象
            stock_info = StockInfo(
                code=code,
                name=name or f"股票{code}",
                total_market_cap=info_dict.get('total_market_cap', ''),
                circulation_market_cap=info_dict.get('circulation_market_cap', ''),
                industry=info_dict.get('industry', ''),
                pe_ratio=info_dict.get('pe_ratio', ''),
                pb_ratio=info_dict.get('pb_ratio', ''),
                dividend_rate=info_dict.get('dividend_rate', '')
            )
            
            self._set_cache(cache_key, stock_info)
            logger.info(f"成功获取股票信息: {code}")
            return stock_info
            
        except Exception as e:
            logger.error(f"获取股票信息失败 {code}: {e}", exc_info=True)
            return None
    
    async def get_research_visit_data(self, code: str, date: str) -> List[StockResearchVisit]:
        try:
            cache_key = self._get_cache_key("research_visit", code=code, date=date)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取机构调研数据
            df = ak.stock_jgdy_tj_em(date=date)
            df = self._handle_df_result(df)
            
            if df.empty:
                return []                
            # 筛选指定股票的数据
            stock_data = df[df['股票代码'] == code]
            if stock_data.empty:
                return []
                
            visits = []
            for _, row in stock_data.iterrows():
                def safe_int(value, default=0):
                    try:
                        return int(value) if value else default
                    except (ValueError, TypeError):
                        return default
                
                visit = StockResearchVisit(
                    code=code,
                    name=str(row.get('股票简称', code)),
                    visit_date=pd.to_datetime(row.get('调研日期', datetime.now())),
                    visit_num=safe_int(row.get('调研机构数量')),
                    visit_org=str(row.get('调研机构')),
                    visit_type=str(row.get('调研方式', '')),
                    visit_content=str(row.get('调研内容', '')),
                    visit_summary=str(row.get('调研摘要', '')),
                    receive_org=str(row.get('接待机构', '')),
                    receive_people=str(row.get('接待人员', ''))
                )
                visits.append(visit)
            
            self._set_cache(cache_key, visits)
            return visits
            
        except Exception as e:
            logger.error(f"获取机构调研数据失败 {code}: {e}")
            return []
    
    async def get_repurchase_data(self, code: str) -> List[StockRepurchase]:
        """"获取股票回购数据"""
        try:
            cache_key = self._get_cache_key("repurchase", code=code)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取股票回购数据
            df = ak.stock_repurchase_em()
            df = self._handle_df_result(df)
            
            if df.empty:
                return []
                
            # 筛选指定股票的数据
            stock_data = df[df['股票代码'] == code]
            if stock_data.empty:
                return []
                    
            repurchases = []
            for _, row in stock_data.iterrows():
                def safe_float(value, default=0.0):
                    try:
                        return float(value) if value else default
                    except (ValueError, TypeError):
                        return default
                    
                repurchase = StockRepurchase(
                    code=code,
                    name=str(row.get('股票简称', code)),
                    announce_date=pd.to_datetime(row.get('公告日期', datetime.now())),
                    plan_type=str(row.get('回购方案类型', '')),
                    price_range=str(row.get('回购价格区间', '')),
                    num_lower=safe_float(row.get('回购数量下限')),
                    num_upper=safe_float(row.get('回购数量上限')),
                    ratio_lower=safe_float(row.get('占总股本比例下限')),
                    ratio_upper=safe_float(row.get('占总股本比例上限')),
                    amount_lower=safe_float(row.get('回购金额下限')),
                    amount_upper=safe_float(row.get('回购金额上限')),
                    purpose=str(row.get('回购用途', '')),
                    status=str(row.get('回购状态', ''))
                )
                repurchases.append(repurchase)
            
            self._set_cache(cache_key, repurchases)
            return repurchases
            
        except Exception as e:
            logger.error(f"获取股票回购数据失败 {code}: {e}")
            return []
    
    async def get_margin_list_data(self, margin_type: str = "融资") -> List[StockMarginList]:
        """获取融资融券标的列表"""
        try:
            cache_key = self._get_cache_key("margin_list", margin_type=margin_type)
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached
                
            # 获取融资融券标的列表
            # 这里使用融资数据作为参考，实际需要专门的接口
            df = ak.stock_margin_detail_sse(date=datetime.now().strftime("%Y%m%d"))
            df = self._handle_df_result(df)
            
            if df.empty:
                return []
                
            margin_list = []
            for _, row in df.iterrows():
                margin = StockMarginList(
                    code=str(row.get('股票代码', '')),
                    name=str(row.get('股票简称', '')),
                    list_date=pd.to_datetime(row.get('交易日期', datetime.now())),
                    margin_type=margin_type,
                    market=str(row.get('市场', '')),
                    status="正常"
                )
                margin_list.append(margin)
            
            self._set_cache(cache_key, margin_list)
            return margin_list
            
        except Exception as e:
            logger.error(f"获取融资融券标的列表失败 {margin_type}: {e}")
            return []