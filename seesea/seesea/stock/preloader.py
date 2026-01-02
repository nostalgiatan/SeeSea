#!/usr/bin/env python
"""
股票数据预加载模块

提供全量数据预加载功能，在初始化时加载所有非实时数据到缓存，
然后按照服务频率进行更新，数据处理系统直接从缓存获取数据。
"""

import asyncio
import logging
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Any, Set
import pandas as pd
from concurrent.futures import ThreadPoolExecutor
import akshare as ak

from .models import StockBasic, StockInfo
from .cache import CacheScope, CacheTTL, get_cache_manager
from .client import StockDataClient

logger = logging.getLogger(__name__)


class StockDataPreloader:
    """股票数据预加载器"""
    
    def __init__(self, client: StockDataClient, max_workers: int = 4):
        """
        初始化预加载器
        
        Args:
            client: 股票数据客户端
            max_workers: 最大工作线程数
        """
        self.client = client
        self.cache_manager = get_cache_manager()
        self.max_workers = max_workers
        self.executor = ThreadPoolExecutor(max_workers=max_workers)
        self._preloaded_codes: Set[str] = set()
        self._is_preloading = False
        
    async def preload_all_data(self, force_reload: bool = False) -> Dict[str, Any]:
        """
        预加载所有非实时数据
        
        Args:
            force_reload: 是否强制重新加载，忽略缓存
            
        Returns:
            Dict[str, Any]: 预加载结果统计
        """
        if self._is_preloading:
            logger.warning("预加载正在进行中，跳过本次请求")
            return {"status": "already_running"}
            
        self._is_preloading = True
        start_time = datetime.now()
        
        try:
            logger.info("开始预加载全量股票数据...")
            
            # 步骤1: 获取股票列表
            stock_list = await self._preload_stock_list()
            if not stock_list:
                logger.error("无法获取股票列表，预加载失败")
                return {"status": "failed", "error": "无法获取股票列表"}
            
            total_stocks = len(stock_list)
            logger.info(f"获取到 {total_stocks} 只股票")
            
            # 步骤2: 预加载基础信息数据
            basic_info_results = await self._preload_basic_info(stock_list, force_reload)
            
            # 步骤3: 预加载财务数据
            financial_results = await self._preload_financial_data(stock_list, force_reload)
            
            # 步骤4: 预加载K线历史数据（日线）
            kline_results = await self._preload_kline_data(stock_list, force_reload)
            
            # 步骤5: 预加载行业数据
            industry_results = await self._preload_industry_data(force_reload)
            
            # 步骤6: 预加载股东数据
            holder_results = await self._preload_holder_data(stock_list, force_reload)
            
            end_time = datetime.now()
            duration = (end_time - start_time).total_seconds()
            
            results = {
                "status": "success",
                "duration_seconds": duration,
                "total_stocks": total_stocks,
                "preloaded_codes": len(self._preloaded_codes),
                "basic_info": basic_info_results,
                "financial": financial_results,
                "kline": kline_results,
                "industry": industry_results,
                "holder": holder_results
            }
            
            logger.info(f"预加载完成，耗时 {duration:.2f} 秒，共处理 {len(self._preloaded_codes)} 只股票")
            return results
            
        except Exception as e:
            logger.error(f"预加载过程失败: {type(e).__name__}: {e}", exc_info=True)
            return {"status": "failed", "error": str(e)}
            
        finally:
            self._is_preloading = False
    
    async def _preload_stock_list(self) -> List[str]:
        """预加载股票列表"""
        cache_key = f"{CacheScope.STOCK_LIST}:all"
        
        # 检查缓存
        cached_data = await self.cache_manager.get(cache_key)
        if cached_data:
            logger.info("股票列表已从缓存获取")
            return cached_data
        
        try:
            # 获取A股列表
            df = ak.stock_zh_a_spot_em()
            if df.empty:
                logger.error("获取股票列表失败：数据为空")
                return []
            
            # 提取股票代码
            stock_codes = df['代码'].tolist()
            
            # 缓存股票列表（24小时）
            await self.cache_manager.set(
                cache_key, 
                stock_codes, 
                ttl=CacheTTL.STOCK_LIST
            )
            
            logger.info(f"股票列表预加载完成，共 {len(stock_codes)} 只股票")
            return stock_codes
            
        except Exception as e:
            logger.error(f"预加载股票列表失败: {type(e).__name__}: {e}")
            return []
    
    async def _preload_basic_info(self, stock_codes: List[str], force_reload: bool) -> Dict[str, Any]:
        """预加载基础信息数据"""
        logger.info("开始预加载基础信息数据...")
        success_count = 0
        failed_codes = []
        
        # 分批处理，避免单次请求过多
        batch_size = 50
        for i in range(0, len(stock_codes), batch_size):
            batch = stock_codes[i:i + batch_size]
            
            # 并发处理批次内的股票
            tasks = []
            for code in batch:
                task = self._preload_single_basic_info(code, force_reload)
                tasks.append(task)
            
            # 等待批次完成
            batch_results = await asyncio.gather(*tasks, return_exceptions=True)
            
            # 统计结果
            for code, result in zip(batch, batch_results):
                if isinstance(result, Exception):
                    failed_codes.append(code)
                    logger.warning(f"股票 {code} 基础信息预加载失败: {result}")
                elif result:
                    success_count += 1
                    self._preloaded_codes.add(code)
            
            logger.info(f"基础信息批次进度: {i+len(batch)}/{len(stock_codes)}")
        
        result = {
            "success_count": success_count,
            "failed_count": len(failed_codes),
            "failed_codes": failed_codes[:10]  # 只记录前10个失败的股票
        }
        
        logger.info(f"基础信息预加载完成: 成功 {success_count}, 失败 {len(failed_codes)}")
        return result
    
    async def _preload_single_basic_info(self, code: str, force_reload: bool) -> bool:
        """预加载单只股票的基础信息"""
        cache_key = f"{CacheScope.STOCK_INFO}:{code}"
        
        # 检查缓存
        if not force_reload:
            cached_data = await self.cache_manager.get(cache_key)
            if cached_data:
                return True
        
        try:
            # 获取股票基本信息
            df = ak.stock_individual_info_em(symbol=code)
            if df.empty:
                return False
            
            # 转换为字典格式
            info_dict = df.set_index('item')['value'].to_dict()
            
            # 缓存基础信息（24小时）
            await self.cache_manager.set(
                cache_key,
                info_dict,
                ttl=CacheTTL.STOCK_INFO
            )
            
            return True
            
        except Exception as e:
            logger.warning(f"获取股票 {code} 基础信息失败: {type(e).__name__}: {e}")
            return False
    
    async def _preload_financial_data(self, stock_codes: List[str], force_reload: bool) -> Dict[str, Any]:
        """预加载财务数据"""
        logger.info("开始预加载财务数据...")
        success_count = 0
        failed_codes = []
        
        # 只预加载部分重要股票的财务数据，避免数据量过大
        important_codes = [code for code in stock_codes if code.startswith(('60', '00', '30'))][:100]
        
        for code in important_codes:
            try:
                # 预加载主要财务指标
                cache_key = f"{CacheScope.STOCK_FINANCIAL}:{code}"
                
                if not force_reload:
                    cached_data = await self.cache_manager.get(cache_key)
                    if cached_data:
                        success_count += 1
                        continue
                
                # 获取财务指标
                df = ak.stock_financial_abstract_em(symbol=code)
                if not df.empty:
                    financial_dict = df.iloc[0].to_dict() if len(df) > 0 else {}
                    
                    await self.cache_manager.set(
                        cache_key,
                        financial_dict,
                        ttl=CacheTTL.FINANCIAL
                    )
                    success_count += 1
                
            except Exception as e:
                failed_codes.append(code)
                logger.warning(f"股票 {code} 财务数据预加载失败: {type(e).__name__}: {e}")
        
        result = {
            "success_count": success_count,
            "failed_count": len(failed_codes),
            "total_processed": len(important_codes)
        }
        
        logger.info(f"财务数据预加载完成: 成功 {success_count}, 失败 {len(failed_codes)}")
        return result
    
    async def _preload_kline_data(self, stock_codes: List[str], force_reload: bool) -> Dict[str, Any]:
        """预加载K线数据"""
        logger.info("开始预加载K线数据...")
        success_count = 0
        failed_codes = []
        
        # 只预加载部分活跃股票的K线数据
        sample_codes = stock_codes[:200]  # 取前200只股票作为样本
        
        for code in sample_codes:
            try:
                # 预加载日线数据（最近30天）
                cache_key = f"{CacheScope.STOCK_KLINE_DAILY}:{code}"
                
                if not force_reload:
                    cached_data = await self.cache_manager.get(cache_key)
                    if cached_data:
                        success_count += 1
                        continue
                
                # 获取日线数据
                end_date = datetime.now().strftime('%Y%m%d')
                start_date = (datetime.now() - timedelta(days=30)).strftime('%Y%m%d')
                
                df = ak.stock_zh_a_hist(symbol=code, period="daily", 
                                        start_date=start_date, end_date=end_date)
                
                if not df.empty:
                    # 转换为字典列表格式
                    kline_data = df.to_dict('records')
                    
                    await self.cache_manager.set(
                        cache_key,
                        kline_data,
                        ttl=CacheTTL.KLINE_DAILY
                    )
                    success_count += 1
                
            except Exception as e:
                failed_codes.append(code)
                logger.warning(f"股票 {code} K线数据预加载失败: {type(e).__name__}: {e}")
        
        result = {
            "success_count": success_count,
            "failed_count": len(failed_codes),
            "total_processed": len(sample_codes)
        }
        
        logger.info(f"K线数据预加载完成: 成功 {success_count}, 失败 {len(failed_codes)}")
        return result
    
    async def _preload_industry_data(self, force_reload: bool) -> Dict[str, Any]:
        """预加载行业数据"""
        logger.info("开始预加载行业数据...")
        
        try:
            cache_key = f"{CacheScope.STOCK_INDUSTRY}:all"
            
            if not force_reload:
                cached_data = await self.cache_manager.get(cache_key)
                if cached_data:
                    return {"success_count": 1, "failed_count": 0}
            
            # 获取行业分类数据
            df = ak.stock_industry_category_cninfo()
            
            if not df.empty:
                industry_data = df.to_dict('records')
                
                await self.cache_manager.set(
                    cache_key,
                    industry_data,
                    ttl=CacheTTL.INDUSTRY
                )
                
                logger.info("行业数据预加载完成")
                return {"success_count": 1, "failed_count": 0}
            else:
                logger.warning("行业数据为空")
                return {"success_count": 0, "failed_count": 1}
                
        except Exception as e:
            logger.error(f"行业数据预加载失败: {type(e).__name__}: {e}")
            return {"success_count": 0, "failed_count": 1}
    
    async def _preload_holder_data(self, stock_codes: List[str], force_reload: bool) -> Dict[str, Any]:
        """预加载股东数据"""
        logger.info("开始预加载股东数据...")
        success_count = 0
        failed_codes = []
        
        # 只预加载部分重要股票的股东数据
        sample_codes = [code for code in stock_codes if code.startswith(('60', '00'))][:50]
        
        for code in sample_codes:
            try:
                cache_key = f"{CacheScope.STOCK_HOLDER}:{code}"
                
                if not force_reload:
                    cached_data = await self.cache_manager.get(cache_key)
                    if cached_data:
                        success_count += 1
                        continue
                
                # 获取股东数据
                df = ak.stock_main_stock_holder(stock=code)
                
                if not df.empty:
                    holder_data