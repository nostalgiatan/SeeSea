"""
股票数据请求事件处理器
处理来自Rust端的股票数据请求事件
"""

import asyncio
import json
import logging
from typing import Optional, Dict, Any
from datetime import datetime

logger = logging.getLogger(__name__)


class StockDataRequestHandler:
    """股票数据请求事件处理器"""
    
    def __init__(self, task_queue, cache_manager, client):
        """
        初始化事件处理器
        
        Args:
            task_queue: 任务队列实例
            cache_manager: 缓存管理器实例
            client: 股票数据客户端实例
        """
        self._task_queue = task_queue
        self._cache = cache_manager
        self._client = client
        self._is_running = False
        self._listener_task: Optional[asyncio.Task] = None
        self._pending_requests: Dict[str, datetime] = {}  # 记录正在处理的请求
        
    async def start(self):
        """启动事件监听器"""
        if self._is_running:
            logger.warning("事件监听器已在运行")
            return
        
        self._is_running = True
        self._listener_task = asyncio.create_task(self._listen_for_requests())
        logger.info("股票数据请求事件监听器已启动")
        
    async def stop(self):
        """停止事件监听器"""
        if not self._is_running:
            return
        
        self._is_running = False
        
        if self._listener_task:
            self._listener_task.cancel()
            try:
                await self._listener_task
            except asyncio.CancelledError:
                pass
        
        logger.info("股票数据请求事件监听器已停止")
        
    async def _listen_for_requests(self):
        """监听股票数据请求事件"""
        try:
            # 尝试导入事件系统
            try:
                from seesea_core import on_string_event
                has_event_system = True
            except ImportError:
                logger.warning("未找到事件系统，事件监听器将无法工作")
                has_event_system = False
            
            if not has_event_system:
                return
            
            # 注册事件处理器
            try:
                await on_string_event(
                    "stock.data.request",
                    self._handle_event
                )
                logger.info("已注册股票数据请求事件处理器")
            except Exception as e:
                logger.error(f"注册事件处理器失败: {e}")
                self._is_running = False
                
        except asyncio.CancelledError:
            logger.info("事件监听器被取消")
            self._is_running = False
        except Exception as e:
            logger.error(f"事件监听器发生严重错误: {e}")
            self._is_running = False
            
    def _handle_event(self, event_type: str, data: str):
        """
        事件处理回调函数（同步，用于注册）
        
        Args:
            event_type: 事件类型
            data: 事件数据（JSON字符串）
        """
        try:
            # 解析事件数据
            import json
            event_data = json.loads(data)
            
            # 创建异步任务来处理事件
            asyncio.create_task(self._handle_request(event_data))
            
            # 返回空响应
            return ""
            
        except Exception as e:
            logger.error(f"处理事件回调时发生错误: {e}")
            return ""
            
    async def _handle_request(self, event_data: Dict[str, Any]):
        """
        处理股票数据请求事件
        
        Args:
            event_data: 事件数据，包含股票代码等信息
        """
        try:
            # 解析事件数据
            code = event_data.get("code")
            data_type = event_data.get("data_type", "info")  # info, financial, etc.
            
            if not code:
                logger.warning("收到无效的股票数据请求事件: 缺少code字段")
                return
            
            logger.info(f"收到股票数据请求事件: code={code}, data_type={data_type}")
            
            # 检查是否已经在处理中
            if code in self._pending_requests:
                logger.info(f"股票 {code} 的数据已在处理中，跳过")
                return
            
            # 记录请求时间
            self._pending_requests[code] = datetime.now()
            
            # 根据数据类型创建相应的任务
            if data_type == "info":
                # 股票基本信息
                await self._task_queue.add_task(
                    task_type="stock_info",
                    code=code,
                    priority=0  # 最高优先级
                )
            elif data_type == "financial":
                # 财务数据
                await self._task_queue.add_task(
                    task_type="financial_data",
                    code=code,
                    priority=0  # 最高优先级
                )
            elif data_type == "industry":
                # 行业数据
                await self._task_queue.add_task(
                    task_type="industry_data",
                    code=code,
                    priority=0  # 最高优先级
                )
            elif data_type == "quote":
                # 股票行情数据
                await self._task_queue.add_task(
                    task_type="quote_data",
                    code=code,
                    priority=0  # 最高优先级
                )
            elif data_type == "kline":
                # K线数据
                await self._task_queue.add_task(
                    task_type="kline_data",
                    code=code,
                    priority=0  # 最高优先级
                )
            elif data_type == "holders":
                # 股东数据
                await self._task_queue.add_task(
                    task_type="holders_data",
                    code=code,
                    priority=0  # 最高优先级
                )
            elif data_type == "announcements":
                # 公告数据
                await self._task_queue.add_task(
                    task_type="announcements_data",
                    code=code,
                    priority=0  # 最高优先级
                )
            else:
                logger.warning(f"未知的数据类型: {data_type}")
                return
            
            logger.info(f"已将股票 {code} 的 {data_type} 数据获取任务提升为最高优先级")
            
            # 监听任务完成
            asyncio.create_task(self._wait_for_completion(code, data_type))
            
        except Exception as e:
            logger.error(f"处理股票数据请求事件时发生错误: {e}")
            # 从待处理列表中移除
            if code in self._pending_requests:
                del self._pending_requests[code]
                
    async def _wait_for_completion(self, code: str, data_type: str):
        """
        等待任务完成并发送done事件
        
        Args:
            code: 股票代码
            data_type: 数据类型
        """
        try:
            # 等待任务完成（轮询检查缓存）
            max_wait_time = 300  # 最多等待5分钟
            check_interval = 1   # 每秒检查一次
            waited_time = 0
            
            while waited_time < max_wait_time:
                # 检查缓存中是否已有数据
                cache_key = f"stock.{data_type}"
                data = await self._cache.get(cache_key, code)
                
                if data is not None:
                    logger.info(f"股票 {code} 的 {data_type} 数据已就绪")
                    
                    # 发送done事件
                    await self._send_done_event(code, data_type)
                    
                    # 从待处理列表中移除
                    if code in self._pending_requests:
                        del self._pending_requests[code]
                    
                    return
                
                await asyncio.sleep(check_interval)
                waited_time += check_interval
            
            # 超时
            logger.warning(f"等待股票 {code} 的 {data_type} 数据超时")
            
            # 从待处理列表中移除
            if code in self._pending_requests:
                del self._pending_requests[code]
                
        except Exception as e:
            logger.error(f"等待任务完成时发生错误: {e}")
            # 从待处理列表中移除
            if code in self._pending_requests:
                del self._pending_requests[code]
                
    async def _send_done_event(self, code: str, data_type: str):
        """
        发送任务完成事件
        
        Args:
            code: 股票代码
            data_type: 数据类型
        """
        try:
            # 尝试导入事件系统
            from seesea_core import send_string_notification_event
            
            # 构造事件数据
            event_data = {
                "code": code,
                "data_type": data_type,
                "status": "done",
                "timestamp": datetime.now().isoformat()
            }
            
            # 发送done事件
            await send_string_notification_event(
                "stock.data.done",
                json.dumps(event_data)
            )
            
            logger.info(f"已发送股票 {code} 的 {data_type} 数据完成事件")
            
        except ImportError:
            logger.warning("未找到事件系统，无法发送done事件")
        except Exception as e:
            logger.error(f"发送done事件时发生错误: {e}")
