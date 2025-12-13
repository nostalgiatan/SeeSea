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
股票数据缓存管理

基于 seesea_core 的缓存系统，实现多级缓存策略:
- 实时数据: 5秒TTL (股票作用域)
- 短期数据: 5分钟TTL (资金流向等)
- 中期数据: 30分钟TTL (K线日内)
- 长期数据: 12小时TTL (财务、股东等)
- 持久数据: 无过期 (公司基础信息、公告)
"""

import json
from datetime import datetime, timedelta
from typing import Optional, List, Dict, Any, TypeVar
from dataclasses import asdict
import logging

logger = logging.getLogger(__name__)


# 默认缓存路径（后备方案）
def _get_default_cache_path() -> str:
    """获取默认缓存路径"""
    import platform
    from pathlib import Path

    system = platform.system().lower()
    if system == "windows":
        base = Path("D:/seesea") if Path("D:/").exists() else Path("C:/seesea")
    elif system == "darwin":
        base = Path.home() / "Library" / "Application Support" / "seesea"
    else:
        base = Path.home() / ".local" / "share" / "seesea"

    cache_dir = base / "cache"
    cache_dir.mkdir(parents=True, exist_ok=True)
    return str(cache_dir / "stock_cache.db")


# 尝试导入 seesea_core 缓存
try:
    from seesea_core import PyCacheInterface

    CACHE_AVAILABLE = True
except ImportError:
    CACHE_AVAILABLE = False
    PyCacheInterface = None
    logger.warning("seesea_core 缓存不可用")


# 缓存作用域定义
class CacheScope:
    """缓存作用域常量"""

    # 股票实时数据 (5秒)
    STOCK_REALTIME = "stock:realtime"

    # 股票短期数据 (5分钟)
    STOCK_QUOTE = "stock:quote"
    STOCK_FUND_FLOW = "stock:fund_flow"

    # 股票中期数据 (30分钟)
    STOCK_KLINE_INTRADAY = "stock:kline:intraday"
    STOCK_INDEX = "stock:index"
    STOCK_MARKET_STATUS = "stock:market_status"

    # 股票长期数据 (12小时)
    STOCK_KLINE = "stock:kline"
    STOCK_KLINE_DAILY = "stock:kline:daily"
    STOCK_FINANCIAL = "stock:financial"
    STOCK_HOLDER = "stock:holder"
    STOCK_INDUSTRY = "stock:industry"

    # 股票持久数据 (不过期/每日更新)
    STOCK_INFO = "stock:info"
    STOCK_LIST = "stock:list"
    STOCK_ANNOUNCEMENT = "stock:announcement"


# TTL 配置 (秒)
class CacheTTL:
    """缓存TTL配置"""

    REALTIME = 5  # 实时数据: 5秒
    SHORT = 300  # 短期数据: 5分钟
    MEDIUM = 1800  # 中期数据: 30分钟
    LONG = 43200  # 长期数据: 12小时
    DAILY = 86400  # 每日数据: 24小时
    PERSISTENT = 604800  # 持久数据: 7天


T = TypeVar("T")


class StockCacheManager:
    """
    股票数据缓存管理器

    提供统一的缓存访问接口，支持:
    - 多级TTL策略
    - 作用域隔离
    - 序列化/反序列化
    - 缓存预热
    - 过期数据回退
    """

    def __init__(self, cache_path: Optional[str] = None):
        """
        初始化缓存管理器

        Args:
            cache_path: 缓存数据库路径，None 则使用后备默认路径
        """
        if cache_path is None:
            cache_path = _get_default_cache_path()

        self._cache_path = cache_path
        self._cache: Optional[Any] = None
        self._initialized = False

        # 内存缓存 (用于超高频访问)
        self._memory_cache: Dict[str, Dict[str, Any]] = {}
        self._memory_cache_time: Dict[str, datetime] = {}

    async def initialize(self) -> bool:
        """初始化缓存系统"""
        if self._initialized:
            return True

        if CACHE_AVAILABLE and PyCacheInterface:
            try:
                # 创建缓存接口，提供必需的参数
                self._cache = PyCacheInterface(
                    db_path=self._cache_path,
                    ttl_secs=CacheTTL.MEDIUM,  # 默认30分钟TTL
                    max_size_mb=100,  # 最大100MB缓存
                )
                self._initialized = True
                logger.info(f"股票缓存初始化成功: {self._cache_path}")
                return True
            except Exception as e:
                logger.error(f"股票缓存初始化失败: {e}")

        # 降级到纯内存缓存
        logger.warning("使用内存缓存模式")
        self._initialized = True
        return True

    def _get_cache_key(self, scope: str, key: str) -> str:
        """生成缓存键"""
        return f"{scope}:{key}"

    def _serialize(self, data: Any) -> bytes:
        """序列化数据"""
        if hasattr(data, "to_dict"):
            data = data.to_dict()
        elif hasattr(data, "__dataclass_fields__"):
            data = asdict(data)

        return json.dumps(data, ensure_ascii=False, default=str).encode("utf-8")

    def _deserialize(self, data: bytes) -> Any:
        """反序列化数据"""
        return json.loads(data.decode("utf-8"))

    def _get_ttl(self, scope: str) -> int:
        """根据作用域获取TTL"""
        ttl_map = {
            CacheScope.STOCK_REALTIME: CacheTTL.REALTIME,
            CacheScope.STOCK_QUOTE: CacheTTL.SHORT,
            CacheScope.STOCK_FUND_FLOW: CacheTTL.SHORT,
            CacheScope.STOCK_KLINE_INTRADAY: CacheTTL.MEDIUM,
            CacheScope.STOCK_INDEX: CacheTTL.MEDIUM,
            CacheScope.STOCK_MARKET_STATUS: CacheTTL.MEDIUM,
            CacheScope.STOCK_KLINE: CacheTTL.LONG,
            CacheScope.STOCK_KLINE_DAILY: CacheTTL.LONG,
            CacheScope.STOCK_FINANCIAL: CacheTTL.LONG,
            CacheScope.STOCK_HOLDER: CacheTTL.LONG,
            CacheScope.STOCK_INDUSTRY: CacheTTL.LONG,
            CacheScope.STOCK_INFO: CacheTTL.DAILY,
            CacheScope.STOCK_LIST: CacheTTL.DAILY,
            CacheScope.STOCK_ANNOUNCEMENT: CacheTTL.PERSISTENT,
        }
        return ttl_map.get(scope, CacheTTL.SHORT)

    async def get(
        self,
        scope: str,
        key: str,
        include_stale: bool = False,
    ) -> Optional[Any]:
        """
        获取缓存数据

        Args:
            scope: 缓存作用域
            key: 缓存键
            include_stale: 是否包含过期数据

        Returns:
            缓存数据，不存在返回 None
        """
        if not self._initialized:
            await self.initialize()

        cache_key = self._get_cache_key(scope, key)

        # 先查内存缓存
        if cache_key in self._memory_cache:
            cache_time = self._memory_cache_time.get(cache_key)
            ttl = self._get_ttl(scope)

            if cache_time:
                age = (datetime.now() - cache_time).total_seconds()
                if age < ttl:
                    return self._memory_cache[cache_key]
                elif include_stale:
                    return self._memory_cache[cache_key]

        # 查 seesea_core 缓存
        if self._cache:
            try:
                scope_cache = self._cache.scope(scope)

                if include_stale:
                    result = scope_cache.get_include_stale(key)
                    if result:
                        data, is_stale = result
                        return self._deserialize(data)
                else:
                    result = scope_cache.get(key)
                    if result:
                        return self._deserialize(result)
            except Exception as e:
                logger.warning(f"缓存读取失败 {cache_key}: {e}")

        return None

    async def set(
        self,
        scope: str,
        key: str,
        data: Any,
        ttl: Optional[int] = None,
    ) -> bool:
        """
        设置缓存数据

        Args:
            scope: 缓存作用域
            key: 缓存键
            data: 缓存数据
            ttl: 生存时间 (秒)，None 使用默认值

        Returns:
            是否成功
        """
        if not self._initialized:
            await self.initialize()

        cache_key = self._get_cache_key(scope, key)

        if ttl is None:
            ttl = self._get_ttl(scope)

        # 写入内存缓存
        self._memory_cache[cache_key] = data
        self._memory_cache_time[cache_key] = datetime.now()

        # 写入 seesea_core 缓存
        if self._cache:
            try:
                scope_cache = self._cache.scope(scope)
                serialized = self._serialize(data)
                scope_cache.set(key, serialized, timedelta(seconds=ttl))
                return True
            except Exception as e:
                logger.warning(f"缓存写入失败 {cache_key}: {e}")

        return True

    async def delete(self, scope: str, key: str) -> bool:
        """删除缓存"""
        cache_key = self._get_cache_key(scope, key)

        # 删除内存缓存
        self._memory_cache.pop(cache_key, None)
        self._memory_cache_time.pop(cache_key, None)

        # 删除 seesea_core 缓存
        if self._cache:
            try:
                scope_cache = self._cache.scope(scope)
                scope_cache.delete(key)
                return True
            except Exception as e:
                logger.warning(f"缓存删除失败 {cache_key}: {e}")

        return True

    async def clear_scope(self, scope: str) -> bool:
        """清空指定作用域的缓存"""
        # 清理内存缓存
        prefix = f"{scope}:"
        keys_to_delete = [k for k in self._memory_cache if k.startswith(prefix)]
        for k in keys_to_delete:
            self._memory_cache.pop(k, None)
            self._memory_cache_time.pop(k, None)

        # 清理 seesea_core 缓存
        if self._cache:
            try:
                scope_cache = self._cache.scope(scope)
                scope_cache.clear()
                return True
            except Exception as e:
                logger.warning(f"清空作用域失败 {scope}: {e}")

        return True

    async def cleanup_expired(self) -> int:
        """清理过期缓存"""
        cleaned = 0

        # 清理内存缓存
        now = datetime.now()
        keys_to_delete = []

        for cache_key, cache_time in self._memory_cache_time.items():
            # 解析 scope
            parts = cache_key.split(":", 1)
            if len(parts) == 2:
                scope = parts[0]
                ttl = self._get_ttl(scope)
                age = (now - cache_time).total_seconds()

                if age > ttl * 2:  # 超过2倍TTL才清理
                    keys_to_delete.append(cache_key)

        for k in keys_to_delete:
            self._memory_cache.pop(k, None)
            self._memory_cache_time.pop(k, None)
            cleaned += 1

        # 清理 seesea_core 缓存
        if self._cache:
            try:
                # 遍历所有作用域清理
                for scope in [
                    CacheScope.STOCK_REALTIME,
                    CacheScope.STOCK_QUOTE,
                    CacheScope.STOCK_KLINE_INTRADAY,
                ]:
                    scope_cache = self._cache.scope(scope)
                    count = scope_cache.cleanup_expired()
                    cleaned += count
            except Exception as e:
                logger.warning(f"清理过期缓存失败: {e}")

        return cleaned

    # ==================== 便捷方法 ====================

    async def get_realtime_quote(self, code: str) -> Optional[Dict]:
        """获取实时行情缓存"""
        return await self.get(CacheScope.STOCK_REALTIME, code)

    async def set_realtime_quote(self, code: str, data: Any) -> bool:
        """设置实时行情缓存"""
        return await self.set(CacheScope.STOCK_REALTIME, code, data)

    async def get_stock_info(self, code: str) -> Optional[Dict]:
        """获取股票信息缓存"""
        return await self.get(CacheScope.STOCK_INFO, code)

    async def set_stock_info(self, code: str, data: Any) -> bool:
        """设置股票信息缓存"""
        return await self.set(CacheScope.STOCK_INFO, code, data)

    async def get_klines(self, code: str, period: str) -> Optional[List[Dict]]:
        """获取K线缓存"""
        key = f"{code}:{period}"
        if period in ("1", "5", "15", "30", "60"):
            return await self.get(CacheScope.STOCK_KLINE_INTRADAY, key)
        return await self.get(CacheScope.STOCK_KLINE_DAILY, key)

    async def set_klines(self, code: str, period: str, data: Any) -> bool:
        """设置K线缓存"""
        key = f"{code}:{period}"
        if period in ("1", "5", "15", "30", "60"):
            return await self.set(CacheScope.STOCK_KLINE_INTRADAY, key, data)
        return await self.set(CacheScope.STOCK_KLINE_DAILY, key, data)

    async def get_announcements(self, code: str) -> Optional[List[Dict]]:
        """获取公告缓存"""
        return await self.get(CacheScope.STOCK_ANNOUNCEMENT, code, include_stale=True)

    async def set_announcements(self, code: str, data: Any) -> bool:
        """设置公告缓存"""
        return await self.set(CacheScope.STOCK_ANNOUNCEMENT, code, data)


# 全局缓存管理器实例
_cache_manager: Optional[StockCacheManager] = None


def get_cache_manager() -> StockCacheManager:
    """获取全局缓存管理器"""
    global _cache_manager
    if _cache_manager is None:
        _cache_manager = StockCacheManager()
    return _cache_manager
