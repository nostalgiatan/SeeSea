#!/usr/bin/env python
"""
股票数据任务队列系统

提供优先级队列和并发任务执行功能，用于后台异步加载股票详细信息
"""

import asyncio
import logging
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Callable, Any, Tuple
from dataclasses import dataclass, field
from enum import Enum
import heapq

from .cache import CacheScope, CacheTTL, StockCacheManager

logger = logging.getLogger(__name__)


class TaskPriority(Enum):
    """任务优先级"""
    HIGH = 0  # 高优先级 - 需要立即获取的数据
    MEDIUM = 1  # 中优先级 - 较重要的数据
    LOW = 2  # 低优先级 - 可以延迟的数据
    DEFERRED = 3  # 延迟优先级 - 数据未过期，可以稍后处理


@dataclass
class StockTask:
    """股票数据获取任务"""
    priority: int  # 优先级（用于排序）
    task_type: str  # 任务类型
    code: str  # 股票代码
    created_at: datetime = field(default_factory=datetime.now)
    
    def __lt__(self, other):
        """比较函数，用于优先级队列排序"""
        if self.priority != other.priority:
            return self.priority < other.priority
        return self.created_at < other.created_at


class StockTaskQueue:
    """股票数据任务队列"""
    
    def __init__(self, max_concurrent: int = 10):
        """
        初始化任务队列
        
        Args:
            max_concurrent: 最大并发任务数
        """
        self._queue: List[StockTask] = []
        self._max_concurrent = max_concurrent
        self._running_tasks: Dict[str, asyncio.Task] = {}
        self._completed_tasks: Dict[str, datetime] = {}
        self._failed_tasks: Dict[str, Tuple[datetime, str]] = {}
        self._task_handlers: Dict[str, Callable] = {}
        self._cache: Optional[StockCacheManager] = None
        self._is_running = False
        self._worker_task: Optional[asyncio.Task] = None
        
    def set_cache(self, cache: StockCacheManager) -> None:
        """设置缓存管理器"""
        self._cache = cache
        
    def register_handler(self, task_type: str, handler: Callable) -> None:
        """
        注册任务处理器
        
        Args:
            task_type: 任务类型
            handler: 处理函数，签名为 async def handler(code: str) -> bool
        """
        self._task_handlers[task_type] = handler
        logger.info(f"注册任务处理器: {task_type}")
        
    async def add_task(
        self,
        task_type: str,
        code: str,
        priority: TaskPriority = TaskPriority.MEDIUM,
        check_expiration: bool = True,
        ttl: int = None,
        scope: str = None
    ) -> bool:
        """
        添加任务到队列
        
        Args:
            task_type: 任务类型
            code: 股票代码
            priority: 任务优先级
            check_expiration: 是否检查缓存过期
            ttl: 缓存TTL（秒）
            scope: 缓存作用域
            
        Returns:
            是否成功添加
        """
        if task_type not in self._task_handlers:
            logger.error(f"未注册的任务类型: {task_type}")
            return False
            
        # 检查是否已完成
        if code in self._completed_tasks:
            completed_time = self._completed_tasks[code]
            age = (datetime.now() - completed_time).total_seconds()
            
            # 如果任务已完成且在有效期内，跳过
            if ttl and age < ttl:
                logger.debug(f"任务已完成且未过期，跳过: {code} ({task_type})")
                return False
        
        # 检查缓存是否过期
        if check_expiration and self._cache and scope:
            try:
                is_fresh = await self._cache.is_cache_fresh(scope, code, ttl or CacheTTL.STOCK_INFO)
                if is_fresh:
                    # 缓存未过期，使用低优先级
                    logger.debug(f"缓存未过期，使用低优先级: {code} ({task_type})")
                    priority = TaskPriority.DEFERRED
            except Exception as e:
                logger.warning(f"检查缓存过期失败: {e}")
        
        # 创建任务
        task = StockTask(
            priority=priority.value,
            task_type=task_type,
            code=code
        )
        
        heapq.heappush(self._queue, task)
        logger.debug(f"添加任务: {code} ({task_type}), 优先级: {priority.name}")
        
        return True
        
    async def add_bulk_tasks(
        self,
        task_type: str,
        codes: List[str],
        priority: TaskPriority = TaskPriority.MEDIUM,
        check_expiration: bool = True,
        ttl: int = None,
        scope: str = None
    ) -> int:
        """
        批量添加任务
        
        Args:
            task_type: 任务类型
            codes: 股票代码列表
            priority: 任务优先级
            check_expiration: 是否检查缓存过期
            ttl: 缓存TTL（秒）
            scope: 缓存作用域
            
        Returns:
            成功添加的任务数
        """
        count = 0
        for code in codes:
            if await self.add_task(task_type, code, priority, check_expiration, ttl, scope):
                count += 1
        logger.info(f"批量添加任务: {count}/{len(codes)} 成功 ({task_type})")
        return count
        
    async def start(self) -> None:
        """启动任务队列处理器"""
        if self._is_running:
            logger.warning("任务队列已在运行")
            return
            
        self._is_running = True
        self._worker_task = asyncio.create_task(self._worker_loop())
        logger.info("任务队列处理器已启动")
        
    async def stop(self) -> None:
        """停止任务队列处理器"""
        if not self._is_running:
            return
            
        self._is_running = False
        
        # 取消所有运行中的任务
        for task in self._running_tasks.values():
            task.cancel()
            
        # 等待所有任务完成
        if self._running_tasks:
            await asyncio.gather(*self._running_tasks.values(), return_exceptions=True)
            
        # 取消worker任务
        if self._worker_task:
            self._worker_task.cancel()
            try:
                await self._worker_task
            except asyncio.CancelledError:
                pass
                
        logger.info("任务队列处理器已停止")
        
    async def _worker_loop(self) -> None:
        """工作循环，从队列中获取任务并执行"""
        while self._is_running:
            try:
                # 等待有可用的并发槽位
                while len(self._running_tasks) >= self._max_concurrent and self._is_running:
                    await asyncio.sleep(0.1)
                    
                if not self._is_running:
                    break
                    
                # 从队列中获取任务
                if not self._queue:
                    await asyncio.sleep(0.1)
                    continue
                    
                task = heapq.heappop(self._queue)
                
                # 检查任务是否已完成
                if task.code in self._completed_tasks:
                    continue
                    
                # 检查是否已有任务在运行
                if task.code in self._running_tasks:
                    continue
                    
                # 执行任务
                worker = asyncio.create_task(self._execute_task(task))
                self._running_tasks[task.code] = worker
                
                # 添加完成回调
                worker.add_done_callback(lambda t, code=task.code: self._on_task_done(code, t))
                
            except Exception as e:
                logger.error(f"工作循环错误: {e}")
                await asyncio.sleep(1)
                
    async def _execute_task(self, task: StockTask) -> bool:
        """执行单个任务"""
        try:
            logger.debug(f"开始执行任务: {task.code} ({task.task_type})")
            
            handler = self._task_handlers.get(task.task_type)
            if not handler:
                logger.error(f"未找到任务处理器: {task.task_type}")
                return False
                
            # 执行处理器
            success = await handler(task.code)
            
            if success:
                logger.debug(f"任务执行成功: {task.code} ({task.task_type})")
            else:
                logger.warning(f"任务执行失败: {task.code} ({task.task_type})")
                
            return success
            
        except Exception as e:
            logger.error(f"任务执行异常: {task.code} ({task.task_type}): {e}")
            return False
            
    def _on_task_done(self, code: str, task: asyncio.Task) -> None:
        """任务完成回调"""
        try:
            # 从运行任务列表中移除
            self._running_tasks.pop(code, None)
            
            # 检查任务结果
            try:
                success = task.result()
                if success:
                    self._completed_tasks[code] = datetime.now()
                else:
                    self._failed_tasks[code] = (datetime.now(), "执行失败")
            except Exception as e:
                self._failed_tasks[code] = (datetime.now(), str(e))
                
        except Exception as e:
            logger.error(f"任务完成回调错误: {e}")
            
    def get_stats(self) -> Dict[str, Any]:
        """获取队列统计信息"""
        return {
            "queue_size": len(self._queue),
            "running_tasks": len(self._running_tasks),
            "completed_tasks": len(self._completed_tasks),
            "failed_tasks": len(self._failed_tasks),
            "is_running": self._is_running,
            "max_concurrent": self._max_concurrent
        }
        
    def get_progress(self) -> Dict[str, Any]:
        """获取任务进度"""
        total = len(self._completed_tasks) + len(self._failed_tasks) + len(self._queue) + len(self._running_tasks)
        completed = len(self._completed_tasks)
        failed = len(self._failed_tasks)
        
        return {
            "total": total,
            "completed": completed,
            "failed": failed,
            "pending": len(self._queue),
            "running": len(self._running_tasks),
            "progress": completed / total if total > 0 else 0.0
        }
