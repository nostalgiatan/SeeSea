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
股票数据更新调度器

实现自动化的数据更新策略:
- 实时数据: 每5秒更新 (交易时段)
- 短期数据: 每5分钟更新
- 中期数据: 每30分钟更新
- 长期数据: 每天开盘前更新
- 持久数据: 每周更新/变动时更新
"""

import asyncio
from datetime import datetime, time, timedelta
from typing import Optional, List, Dict, Any, Callable, Set
from enum import Enum
import logging

logger = logging.getLogger(__name__)


class UpdateFrequency(str, Enum):
    """更新频率"""

    REALTIME = "realtime"  # 5秒
    EVERY_MINUTE = "minute"  # 1分钟
    SHORT = "short"  # 5分钟
    MEDIUM = "medium"  # 30分钟
    HOURLY = "hourly"  # 1小时
    HALF_DAY = "half_day"  # 12小时
    DAILY = "daily"  # 每日
    WEEKLY = "weekly"  # 每周


class TaskStatus(str, Enum):
    """任务状态"""

    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"


class ScheduledTask:
    """调度任务"""

    def __init__(
        self,
        name: str,
        callback: Callable,
        frequency: UpdateFrequency,
        args: tuple = (),
        kwargs: dict[str, Any] | None = None,
    ):
        self.name = name
        self.callback = callback
        self.frequency = frequency
        self.args = args
        self.kwargs = kwargs or {}

        self.status = TaskStatus.PENDING
        self.last_run: Optional[datetime] = None
        self.next_run: Optional[datetime] = None
        self.run_count = 0
        self.error_count = 0
        self.last_error: Optional[str] = None

    def get_interval_seconds(self) -> int:
        """获取更新间隔 (秒)"""
        intervals = {
            UpdateFrequency.REALTIME: 5,
            UpdateFrequency.EVERY_MINUTE: 60,
            UpdateFrequency.SHORT: 300,
            UpdateFrequency.MEDIUM: 1800,
            UpdateFrequency.HOURLY: 3600,
            UpdateFrequency.HALF_DAY: 43200,
            UpdateFrequency.DAILY: 86400,
            UpdateFrequency.WEEKLY: 604800,
        }
        return intervals.get(self.frequency, 300)

    def should_run(self) -> bool:
        """判断是否应该执行"""
        if self.status == TaskStatus.RUNNING:
            return False

        if self.last_run is None:
            return True

        interval = timedelta(seconds=self.get_interval_seconds())
        return datetime.now() >= self.last_run + interval


class StockScheduler:
    """
    股票数据更新调度器

    管理所有数据更新任务，支持:
    - 多频率任务调度
    - 交易时段感知
    - 任务优先级
    - 错误重试
    - 并发控制
    """

    def __init__(
        self,
        max_concurrent: int = 5,
        enable_trading_hours_check: bool = True,
    ):
        """
        初始化调度器

        Args:
            max_concurrent: 最大并发任务数
            enable_trading_hours_check: 是否启用交易时段检查
        """
        self._tasks: Dict[str, ScheduledTask] = {}
        self._running = False
        self._max_concurrent = max_concurrent
        self._semaphore = asyncio.Semaphore(max_concurrent)
        self._enable_trading_hours_check = enable_trading_hours_check

        # 订阅的股票列表
        self._subscribed_codes: Set[str] = set()

        # 回调函数
        self._on_data_update: Optional[Callable] = None
        self._on_error: Optional[Callable] = None

    def is_trading_hours(self) -> bool:
        """判断是否为交易时段"""
        now = datetime.now()

        # 周末不交易
        if now.weekday() >= 5:
            return False

        current_time = now.time()

        # 上午交易时段: 9:30 - 11:30
        morning_start = time(9, 30)
        morning_end = time(11, 30)

        # 下午交易时段: 13:00 - 15:00
        afternoon_start = time(13, 0)
        afternoon_end = time(15, 0)

        return (
            morning_start <= current_time <= morning_end
            or afternoon_start <= current_time <= afternoon_end
        )

    def register_task(
        self,
        name: str,
        callback: Callable,
        frequency: UpdateFrequency,
        args: tuple = (),
        kwargs: dict[str, Any] | None = None,
    ) -> bool:
        """
        注册调度任务

        Args:
            name: 任务名称
            callback: 回调函数
            frequency: 更新频率
            args: 位置参数
            kwargs: 关键字参数

        Returns:
            是否成功
        """
        if name in self._tasks:
            logger.warning(f"任务已存在: {name}")
            return False

        task = ScheduledTask(
            name=name,
            callback=callback,
            frequency=frequency,
            args=args,
            kwargs=kwargs,
        )

        self._tasks[name] = task
        logger.info(f"注册任务: {name}, 频率: {frequency.value}")
        return True

    def unregister_task(self, name: str) -> bool:
        """注销任务"""
        if name not in self._tasks:
            return False

        del self._tasks[name]
        logger.info(f"注销任务: {name}")
        return True

    def subscribe(self, codes: List[str]):
        """订阅股票"""
        self._subscribed_codes.update(codes)
        logger.info(f"订阅股票: {codes}")

    def unsubscribe(self, codes: List[str]):
        """取消订阅"""
        self._subscribed_codes.difference_update(codes)
        logger.info(f"取消订阅: {codes}")

    def get_subscribed_codes(self) -> List[str]:
        """获取订阅列表"""
        return list(self._subscribed_codes)

    def set_on_data_update(self, callback: Callable):
        """设置数据更新回调"""
        self._on_data_update = callback

    def set_on_error(self, callback: Callable):
        """设置错误回调"""
        self._on_error = callback

    async def _run_task(self, task: ScheduledTask):
        """执行单个任务"""
        async with self._semaphore:
            task.status = TaskStatus.RUNNING

            try:
                logger.debug(f"执行任务: {task.name}")

                # 调用回调
                if asyncio.iscoroutinefunction(task.callback):
                    result = await task.callback(*task.args, **task.kwargs)
                else:
                    result = task.callback(*task.args, **task.kwargs)

                task.status = TaskStatus.COMPLETED
                task.last_run = datetime.now()
                task.run_count += 1

                # 触发数据更新回调
                if self._on_data_update:
                    await self._on_data_update(task.name, result)

            except Exception as e:
                task.status = TaskStatus.FAILED
                task.error_count += 1
                task.last_error = str(e)
                logger.error(f"任务执行失败 {task.name}: {e}")

                # 触发错误回调
                if self._on_error:
                    await self._on_error(task.name, e)

    async def _scheduler_loop(self):
        """调度循环"""
        while self._running:
            try:
                # 检查是否为交易时段
                is_trading = self.is_trading_hours()

                # 遍历任务
                tasks_to_run = []

                for task in self._tasks.values():
                    # 实时任务只在交易时段执行
                    if task.frequency == UpdateFrequency.REALTIME:
                        if not is_trading and self._enable_trading_hours_check:
                            continue

                    if task.should_run():
                        tasks_to_run.append(task)

                # 并发执行任务
                if tasks_to_run:
                    await asyncio.gather(
                        *[self._run_task(task) for task in tasks_to_run],
                        return_exceptions=True,
                    )

                # 短暂休眠
                await asyncio.sleep(1)

            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"调度器错误: {e}")
                await asyncio.sleep(5)

    async def start(self):
        """启动调度器"""
        if self._running:
            logger.warning("调度器已在运行")
            return

        self._running = True
        logger.info("启动股票数据调度器")

        # 启动调度循环
        asyncio.create_task(self._scheduler_loop())

    async def stop(self):
        """停止调度器"""
        self._running = False
        logger.info("停止股票数据调度器")

    def get_task_status(self) -> Dict[str, Dict[str, Any]]:
        """获取所有任务状态"""
        status = {}
        for name, task in self._tasks.items():
            status[name] = {
                "status": task.status.value,
                "frequency": task.frequency.value,
                "last_run": task.last_run.isoformat() if task.last_run else None,
                "run_count": task.run_count,
                "error_count": task.error_count,
                "last_error": task.last_error,
            }
        return status

    def get_stats(self) -> Dict[str, Any]:
        """获取调度器统计"""
        return {
            "running": self._running,
            "task_count": len(self._tasks),
            "subscribed_count": len(self._subscribed_codes),
            "is_trading_hours": self.is_trading_hours(),
            "tasks": self.get_task_status(),
        }

    def is_running(self) -> bool:
        """检查调度器是否运行中"""
        return self._running

    def is_trading_time(self) -> bool:
        """检查是否为交易时间（is_trading_hours的别名）"""
        return self.is_trading_hours()

    def get_tasks(self) -> List[Dict[str, Any]]:
        """
        获取所有任务信息列表

        Returns:
            任务信息列表
        """
        tasks = []
        for name, task in self._tasks.items():
            tasks.append(
                {
                    "name": name,
                    "frequency": task.frequency.value,
                    "status": task.status.value,
                    "last_run": task.last_run.isoformat() if task.last_run else None,
                    "run_count": task.run_count,
                }
            )
        return tasks
