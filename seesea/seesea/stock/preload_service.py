#!/usr/bin/env python
"""
股票数据预加载服务

负责在系统启动时预加载所有非实时数据到缓存，以及后续的增量更新
"""

import asyncio
import logging
from datetime import datetime, timedelta
from typing import List, Dict, Any, Optional
from dataclasses import asdict
import pandas as pd
import akshare as ak

from .cache import CacheScope, CacheTTL, StockCacheManager
from .models import Stock, StockInfo, StockFinancial, StockQuote
from .exchange import IndexQuote
from .client import StockDataClient

logger = logging.getLogger(__name__)


class StockPreloadService:
    """股票数据预加载服务"""
    
    def __init__(self, cache_manager: StockCacheManager, client: StockDataClient):
        """
        初始化预加载服务
        
        Args:
            cache_manager: 缓存管理器
            client: 数据客户端（用于获取数据）
        """
        self._cache = cache_manager
        self._client = client
        self._preload_status = {}
        self._preload_complete = False  # 预加载完成标记
        
    async def preload_market_data_priority(self) -> bool:
        """
        优先预加载市场数据以支持搜索功能
        
        Returns:
            是否成功加载市场数据
        """
        logger.info("优先预加载市场数据以支持搜索功能...")
        
        # 检查股票列表是否已存在且有效
        if await self._cache.is_cache_fresh(CacheScope.STOCK_LIST, "list:cn_a", CacheTTL.STOCK_LIST):
            cache_age = await self._cache.get_cache_age(CacheScope.STOCK_LIST, "list:cn_a")
            logger.info(f"股票列表缓存仍然有效（{cache_age}秒前更新），跳过重新加载")
            return True
        
        try:
            # 优先加载股票列表
            logger.info("预加载股票列表...")
            stock_list = await self._client.get_stock_list()
            if stock_list:
                stock_dicts = [asdict(stock) for stock in stock_list]
                await self._cache.set(CacheScope.STOCK_LIST, "list:cn_a", stock_dicts, CacheTTL.STOCK_LIST)
                logger.info(f"股票列表预加载成功：{len(stock_list)} 只股票")
                return True
            else:
                logger.warning("股票列表预加载失败：无数据")
                return False
        except Exception as e:
            logger.error(f"股票列表预加载失败：{e}")
            return False
    
    async def preload_all(self) -> Dict[str, bool]:
        """
        预加载所有非实时数据（异步版本）
        
        Returns:
            预加载状态字典
        """
        logger.info("开始预加载所有股票数据...")
        start_time = datetime.now()
        
        # 第一步：优先加载市场数据以支持搜索功能
        market_data_loaded = await self.preload_market_data_priority()
        
        # 如果市场数据加载失败，仍然继续加载其他数据
        if not market_data_loaded:
            logger.warning("市场数据加载失败，但继续加载其他数据")
        
        # 第二阶段：基础数据（必须成功）
        status1 = await self._preload_basic_data()
        
        # 第三阶段：财务和行业数据
        status2 = await self._preload_financial_data()
        
        # 第四阶段：行情数据（可延迟）
        status3 = await self._preload_quote_data()
        
        # 合并状态
        self._preload_status["market_data"] = market_data_loaded
        self._preload_status.update(status1)
        self._preload_status.update(status2)
        self._preload_status.update(status3)
        
        elapsed = (datetime.now() - start_time).total_seconds()
        success_count = sum(1 for v in self._preload_status.values() if v)
        total_count = len(self._preload_status)
        
        logger.info(f"预加载完成：{success_count}/{total_count} 成功，耗时 {elapsed:.2f} 秒")
        logger.info(f"预加载状态：{self._preload_status}")
        
        # 标记预加载完成
        self._preload_complete = True
        
        return self._preload_status
    
    def _get_preload_progress(self) -> Dict[str, Any]:
        """
        获取当前预加载进度状态
        
        Returns:
            预加载进度信息
        """
        return {
            "market_data": self._preload_status.get("market_data", False),
            "stock_list": self._preload_status.get("stock_list", False),
            "stock_info": self._preload_status.get("stock_info", False),
            "financial_data": self._preload_status.get("financial_data", False),
            "index_data": self._preload_status.get("index_data", False),
            "complete": self._preload_complete,
            "timestamp": datetime.now().isoformat()
        }
    
    def _should_resume_preload(self, scope: str, key: str, max_age: int = None) -> bool:
        """
        判断是否应该继续预加载（断点续服务）
        
        Args:
            scope: 缓存作用域
            key: 缓存键
            max_age: 最大允许年龄
            
        Returns:
            是否应该继续加载
        """
        try:
            import asyncio
            
            # 创建新的事件循环
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            
            try:
                # 检查缓存是否存在且有效
                exists = loop.run_until_complete(self._cache.exists(scope, key))
                if not exists:
                    return True  # 缓存不存在，需要加载
                
                # 检查缓存是否仍然新鲜
                is_fresh = loop.run_until_complete(
                    self._cache.is_cache_fresh(scope, key, max_age)
                )
                
                if is_fresh:
                    cache_age = loop.run_until_complete(
                        self._cache.get_cache_age(scope, key)
                    )
                    logger.info(f"缓存 {scope}:{key} 仍然有效（{cache_age}秒前更新），跳过重新加载")
                    return False
                else:
                    logger.info(f"缓存 {scope}:{key} 已过期，需要重新加载")
                    return True
                    
            finally:
                loop.close()
                
        except Exception as e:
            logger.warning(f"检查缓存状态时出错 {scope}:{key}: {e}，默认需要重新加载")
            return True
    
    def preload_market_data_priority_sync(self, loop=None) -> bool:
        """
        同步版本：优先预加载市场数据以支持搜索功能
        
        Args:
            loop: 可选的外部事件循环，如果为None则创建新循环
        
        Returns:
            是否成功加载市场数据
        """
        logger.info("优先预加载市场数据以支持搜索功能（同步版本）...")
        
        try:
            import asyncio
            
            # 使用传入的事件循环或创建新的事件循环
            own_loop = loop is None
            if own_loop:
                loop = asyncio.new_event_loop()
                asyncio.set_event_loop(loop)
            
            try:
                # 检查事件循环是否已经在运行
                is_running = loop.is_running()
                
                if is_running:
                    # 事件循环已经在运行，不能使用run_until_complete
                    # 这种情况下，我们需要创建一个新的事件循环来执行同步操作
                    logger.warning("传入的事件循环已在运行，创建新的事件循环执行同步操作")
                    sync_loop = asyncio.new_event_loop()
                    asyncio.set_event_loop(sync_loop)
                    try:
                        # 检查股票列表是否已存在且有效
                        is_fresh = sync_loop.run_until_complete(
                            self._cache.is_cache_fresh(CacheScope.STOCK_LIST, "list:cn_a", CacheTTL.STOCK_LIST)
                        )
                        
                        if is_fresh:
                            cache_age = sync_loop.run_until_complete(
                                self._cache.get_cache_age(CacheScope.STOCK_LIST, "list:cn_a")
                            )
                            logger.info(f"股票列表缓存仍然有效（{cache_age}秒前更新），跳过重新加载")
                            return True
                        
                        # 优先加载股票列表
                        logger.info("预加载股票列表...")
                        logger.info(f"缓存访问 - scope: '{CacheScope.STOCK_LIST}', key: 'list:cn_a', TTL: {CacheTTL.STOCK_LIST}s")
                        stock_list = sync_loop.run_until_complete(self._client.get_stock_list())
                        if stock_list:
                            stock_dicts = [asdict(stock) for stock in stock_list]
                            logger.info(f"设置缓存 - scope: '{CacheScope.STOCK_LIST}', key: 'list:cn_a', 数据量: {len(stock_dicts)} 条")
                            
                            # 添加详细的数据预览和缓存设置日志
                            logger.info(f"缓存设置前数据预览 - 前2条数据: {stock_dicts[:2] if stock_dicts else '无数据'}")
                            
                            try:
                                result = sync_loop.run_until_complete(
                                    self._cache.set(CacheScope.STOCK_LIST, "list:cn_a", stock_dicts, CacheTTL.STOCK_LIST)
                                )
                                logger.info(f"缓存设置结果: {result}")
                                
                                # 立即验证缓存是否设置成功
                                verify_result = sync_loop.run_until_complete(
                                    self._cache.get(CacheScope.STOCK_LIST, "list:cn_a")
                                )
                                logger.info(f"缓存验证结果 - 数据存在: {verify_result is not None}, 数据长度: {len(verify_result) if verify_result else 0}")
                                
                            except Exception as cache_error:
                                logger.error(f"缓存设置失败: {cache_error}")
                                logger.error(f"缓存设置异常类型: {type(cache_error)}")
                                return False
                            
                            logger.info(f"股票列表预加载成功：{len(stock_list)} 只股票")
                            self._preload_status["market_data"] = True
                            return True
                        else:
                            logger.warning("股票列表预加载失败：无数据")
                            self._preload_status["market_data"] = False
                            return False
                    finally:
                        sync_loop.close()
                else:
                    # 事件循环未运行，可以正常使用run_until_complete
                    # 检查股票列表是否已存在且有效
                    is_fresh = loop.run_until_complete(
                        self._cache.is_cache_fresh(CacheScope.STOCK_LIST, "list:cn_a", CacheTTL.STOCK_LIST)
                    )
                    
                    if is_fresh:
                        cache_age = loop.run_until_complete(
                            self._cache.get_cache_age(CacheScope.STOCK_LIST, "list:cn_a")
                        )
                        logger.info(f"股票列表缓存仍然有效（{cache_age}秒前更新），跳过重新加载")
                        self._preload_status["market_data"] = True
                        return True
                    
                    # 优先加载股票列表
                    logger.info("预加载股票列表...")
                    logger.info(f"缓存访问 - scope: '{CacheScope.STOCK_LIST}', key: 'list:cn_a', TTL: {CacheTTL.STOCK_LIST}s")
                    stock_list = loop.run_until_complete(self._client.get_stock_list())
                    if stock_list:
                        stock_dicts = [asdict(stock) for stock in stock_list]
                        logger.info(f"设置缓存 - scope: '{CacheScope.STOCK_LIST}', key: 'list:cn_a', 数据量: {len(stock_dicts)} 条")
                        
                        # 添加详细的数据预览和缓存设置日志
                        logger.info(f"缓存设置前数据预览 - 前2条数据: {stock_dicts[:2] if stock_dicts else '无数据'}")
                        
                        try:
                            result = loop.run_until_complete(
                                self._cache.set(CacheScope.STOCK_LIST, "list:cn_a", stock_dicts, CacheTTL.STOCK_LIST)
                            )
                            logger.info(f"缓存设置结果: {result}")
                            
                            # 立即验证缓存是否设置成功
                            verify_result = loop.run_until_complete(
                                self._cache.get(CacheScope.STOCK_LIST, "list:cn_a")
                            )
                            logger.info(f"缓存验证结果 - 数据存在: {verify_result is not None}, 数据长度: {len(verify_result) if verify_result else 0}")
                            
                        except Exception as cache_error:
                            logger.error(f"缓存设置失败: {cache_error}")
                            logger.error(f"缓存设置异常类型: {type(cache_error)}")
                            self._preload_status["market_data"] = False
                            return False
                        
                        logger.info(f"股票列表预加载成功：{len(stock_list)} 只股票")
                        self._preload_status["market_data"] = True
                        return True
                    else:
                        logger.warning("股票列表预加载失败：无数据")
                        self._preload_status["market_data"] = False
                        return False
                    
            finally:
                # 只关闭自己创建的事件循环
                if own_loop:
                    loop.close()
                
        except Exception as e:
            logger.error(f"股票列表预加载失败：{e}")
            return False
    
    def preload_all_sync(self) -> Dict[str, bool]:
        """
        预加载所有非实时数据（同步版本，支持断点续服务）
        
        Returns:
            预加载状态字典
        """
        logger.info("开始同步预加载所有股票数据（支持断点续服务）...")
        start_time = datetime.now()
        
        # 获取当前进度
        progress = self._get_preload_progress()
        logger.info(f"当前预加载进度：{progress}")
        
        try:
            import asyncio
            
            # 创建新的事件循环
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            
            try:
                # 第一步：优先加载市场数据以支持搜索功能
                if not progress.get("market_data", False):
                    logger.info("市场数据未加载，开始加载...")
                    market_data_loaded = self.preload_market_data_priority_sync()
                else:
                    logger.info("市场数据已加载，跳过")
                    market_data_loaded = True
                
                # 如果市场数据加载失败，仍然继续加载其他数据
                if not market_data_loaded:
                    logger.warning("市场数据加载失败，但继续加载其他数据")
                
                # 第二阶段：基础数据（必须成功）
                if not progress.get("stock_list", False) or not progress.get("stock_info", False):
                    logger.info("基础数据未完全加载，开始加载...")
                    status1 = loop.run_until_complete(self._preload_basic_data())
                else:
                    logger.info("基础数据已加载，跳过")
                    status1 = {"stock_list": True, "stock_info": True}
                
                # 第三阶段：财务和行业数据
                if not progress.get("financial_data", False):
                    logger.info("财务数据未加载，开始加载...")
                    status2 = loop.run_until_complete(self._preload_financial_data())
                else:
                    logger.info("财务数据已加载，跳过")
                    status2 = {"financial_data": True}
                
                # 第四阶段：行情数据（可延迟）
                if not progress.get("index_data", False):
                    logger.info("指数数据未加载，开始加载...")
                    status3 = loop.run_until_complete(self._preload_quote_data())
                else:
                    logger.info("指数数据已加载，跳过")
                    status3 = {"index_data": True}
                
                # 合并状态
                self._preload_status["market_data"] = market_data_loaded
                self._preload_status.update(status1)
                self._preload_status.update(status2)
                self._preload_status.update(status3)
                
                elapsed = (datetime.now() - start_time).total_seconds()
                success_count = sum(1 for v in self._preload_status.values() if v)
                total_count = len(self._preload_status)
                
                logger.info(f"同步预加载完成：{success_count}/{total_count} 成功，耗时 {elapsed:.2f} 秒")
                logger.info(f"预加载状态：{self._preload_status}")
                
                # 标记预加载完成
                self._preload_complete = True
                
                return self._preload_status
                
            finally:
                loop.close()
                
        except Exception as e:
            logger.error(f"同步预加载失败: {e}")
            self._preload_complete = True  # 即使失败也标记为完成，避免卡住
            return self._preload_status
    
    async def _preload_basic_data(self) -> Dict[str, bool]:
        """预加载基础数据"""
        status = {}
        
        try:
            # 1. 股票列表
            logger.info("预加载股票列表...")
            logger.info(f"缓存设置 - scope: '{CacheScope.STOCK_LIST}', key: 'list:cn_a'")
            stock_list = await self._client.get_stock_list()
            if stock_list:
                stock_dicts = [asdict(stock) for stock in stock_list]
                await self._cache.set(CacheScope.STOCK_LIST, "list:cn_a", stock_dicts, CacheTTL.STOCK_LIST)
                status["stock_list"] = True
                logger.info(f"股票列表预加载成功：{len(stock_list)} 只股票，已缓存到 scope: '{CacheScope.STOCK_LIST}', key: 'list:cn_a'")
            else:
                status["stock_list"] = False
                logger.warning("股票列表预加载失败：无数据")
        except Exception as e:
            status["stock_list"] = False
            logger.error(f"股票列表预加载失败：{e}")
        
        try:
            # 2. 股票基本信息（并行加载，分批处理）
            logger.info("预加载股票基本信息...")
            if stock_list:  # 使用上面获取的股票列表
                # 启动后台任务，分批并行加载所有股票信息
                import asyncio
                
                batch_size = 50  # 每批处理50只股票
                total_stocks = len(stock_list)
                
                logger.info(f"将并行预加载 {total_stocks} 只股票的基本信息，每批 {batch_size} 只")
                
                # 先快速加载前100只作为基础数据
                quick_load_count = min(100, total_stocks)
                quick_stock_infos = []
                
                async def load_stock_info_batch(stocks_batch, batch_num):
                    """加载一批股票信息"""
                    batch_infos = []
                    for stock in stocks_batch:
                        try:
                            info = await self._client.get_stock_info(stock.code)
                            if info:
                                batch_infos.append(info.to_dict())
                        except Exception as e:
                            logger.warning(f"获取股票 {stock.code} 基本信息失败：{e}")
                    
                    logger.info(f"第 {batch_num} 批完成：加载了 {len(batch_infos)}/{len(stocks_batch)} 只股票信息")
                    return batch_infos
                
                # 快速加载前100只（分2批，每批50只）
                quick_tasks = []
                for i in range(0, quick_load_count, batch_size):
                    batch = stock_list[i:i+batch_size]
                    batch_num = (i // batch_size) + 1
                    task = load_stock_info_batch(batch, batch_num)
                    quick_tasks.append(task)
                
                # 并行执行快速加载
                quick_results = await asyncio.gather(*quick_tasks, return_exceptions=True)
                
                # 收集快速加载结果
                for result in quick_results:
                    if isinstance(result, list):
                        quick_stock_infos.extend(result)
                
                # 缓存快速加载结果
                if quick_stock_infos:
                    await self._cache.set(CacheScope.STOCK_INFO, "quick", quick_stock_infos, CacheTTL.STOCK_INFO)
                    status["stock_info"] = True
                    logger.info(f"快速预加载完成：{len(quick_stock_infos)} 只股票基本信息")
                
                # 启动后台任务加载剩余股票信息
                remaining_stocks = stock_list[quick_load_count:]
                if remaining_stocks:
                    logger.info(f"启动后台任务加载剩余 {len(remaining_stocks)} 只股票信息")
                    
                    async def background_load():
                        """后台加载剩余股票信息"""
                        try:
                            background_infos = []
                            background_tasks = []
                            
                            for i in range(0, len(remaining_stocks), batch_size):
                                batch = remaining_stocks[i:i+batch_size]
                                batch_num = (i // batch_size) + 3  # 从第3批开始编号
                                task = load_stock_info_batch(batch, batch_num)
                                background_tasks.append(task)
                            
                            # 分批执行，避免一次性创建太多任务
                            for j in range(0, len(background_tasks), 5):  # 每5批一起执行
                                batch_tasks = background_tasks[j:j+5]
                                batch_results = await asyncio.gather(*batch_tasks, return_exceptions=True)
                                
                                for result in batch_results:
                                    if isinstance(result, list):
                                        background_infos.extend(result)
                                
                                logger.info(f"后台加载进度：{j+len(batch_tasks)}/{len(background_tasks)} 批完成")
                                await asyncio.sleep(1)  # 短暂间隔，避免过载
                            
                            # 合并所有信息并更新缓存
                            all_infos = quick_stock_infos + background_infos
                            if all_infos:
                                await self._cache.set(CacheScope.STOCK_INFO, "all", all_infos, CacheTTL.STOCK_INFO)
                                logger.info(f"后台加载完成：总共 {len(all_infos)} 只股票基本信息")
                            
                        except Exception as e:
                            logger.error(f"后台加载股票基本信息失败：{e}")
                    
                    # 启动后台任务，不等待完成
                    asyncio.create_task(background_load())
                    
            else:
                status["stock_info"] = False
                logger.warning("股票基本信息预加载跳过：无股票列表")
        except Exception as e:
            status["stock_info"] = False
            logger.error(f"股票基本信息预加载失败：{e}")
        
        return status
    
    async def _preload_financial_data(self) -> Dict[str, bool]:
        """预加载财务数据（并行加载）"""
        status = {}
        
        try:
            # 获取股票列表
            logger.info(f"获取股票列表 - scope: '{CacheScope.STOCK_LIST}', key: 'list:cn_a'")
            stock_list_data = await self._cache.get(CacheScope.STOCK_LIST, "list:cn_a")
            if not stock_list_data:
                logger.warning("财务数据预加载跳过：无股票列表缓存")
                return status
            
            # 1. 财务指标（并行加载前200只作为示例）
            stock_codes = [stock["code"] for stock in stock_list_data[:200]]
            
            logger.info(f"并行预加载财务指标数据，共 {len(stock_codes)} 只股票...")
            
            async def load_financial_data_batch(codes_batch, batch_num):
                """加载一批财务数据"""
                batch_data = []
                for code in codes_batch:
                    try:
                        financial = await self._client.get_financial_data(code)
                        if financial:
                            batch_data.append(financial.to_dict())
                    except Exception as e:
                        logger.warning(f"获取股票 {code} 财务数据失败：{e}")
                
                logger.info(f"财务数据第 {batch_num} 批完成：{len(batch_data)}/{len(codes_batch)} 只股票")
                return batch_data
            
            # 分批并行加载
            batch_size = 50  # 每批50只股票
            financial_tasks = []
            
            for i in range(0, len(stock_codes), batch_size):
                batch = stock_codes[i:i+batch_size]
                batch_num = (i // batch_size) + 1
                task = load_financial_data_batch(batch, batch_num)
                financial_tasks.append(task)
            
            # 并行执行所有任务
            financial_results = await asyncio.gather(*financial_tasks, return_exceptions=True)
            
            # 收集结果
            financial_data = []
            for result in financial_results:
                if isinstance(result, list):
                    financial_data.extend(result)
            
            if financial_data:
                await self._cache.set(CacheScope.STOCK_FINANCIAL, "latest", financial_data, CacheTTL.FINANCIAL)
                status["financial_data"] = True
                logger.info(f"财务数据预加载成功：{len(financial_data)} 只股票")
            else:
                status["financial_data"] = False
                logger.warning("财务数据预加载失败：无数据")
                
        except Exception as e:
            status["financial_data"] = False
            logger.error(f"财务数据预加载失败：{e}")
        
        return status
    
    async def _preload_quote_data(self) -> Dict[str, bool]:
        """预加载行情数据"""
        status = {}
        
        try:
            # 获取主要指数
            logger.info("预加载指数数据...")
            indices = ["000001", "399001", "399006"]  # 上证指数、深证成指、创业板指
            index_quotes = []
            
            for symbol in indices:
                try:
                    index_data = await self._client.get_index_data(symbol)
                    if index_data:
                        index_quotes.append(index_data.to_dict())
                except Exception as e:
                    logger.warning(f"获取指数 {symbol} 数据失败：{e}")
            
            if index_quotes:
                await self._cache.set(CacheScope.STOCK_INDEX, "main", index_quotes, CacheTTL.INDEX)
                status["index_data"] = True
                logger.info(f"指数数据预加载成功：{len(index_quotes)} 个指数")
            else:
                status["index_data"] = False
                
        except Exception as e:
            status["index_data"] = False
            logger.error(f"指数数据预加载失败：{e}")
        
        return status
    
    async def get_preload_status(self) -> Dict[str, bool]:
        """获取预加载状态"""
        return self._preload_status.copy()
    
    def is_preload_complete(self) -> bool:
        """检查预加载是否已完成
        
        快速模式：只要基础数据（股票列表和快速股票信息）加载完成就认为预加载完成
        完整模式：所有数据都加载完成
        """
        if not self._preload_complete:
            return False
            
        # 检查基础数据是否完成
        basic_complete = (
            self._preload_status.get("stock_list", False) and 
            self._preload_status.get("stock_info", False)
        )
        
        return basic_complete