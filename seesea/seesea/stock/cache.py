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

from datetime import datetime, timedelta
from typing import Optional, List, Dict, Any, TypeVar, Tuple
from dataclasses import asdict, dataclass
import logging
import json
import asyncio

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
    STOCK_REALTIME = "stock.realtime"

    # 股票短期数据 (5分钟)
    STOCK_QUOTE = "stock.quote"
    STOCK_FUND_FLOW = "stock.fund_flow"
    STOCK_RANKING = "stock.ranking"  # 股票排行榜 (2分钟)

    # 股票中期数据 (30分钟)
    STOCK_KLINE_INTRADAY = "stock.kline.intraday"
    STOCK_INDEX = "stock.index"
    STOCK_MARKET_STATUS = "stock.market_status"

    # 股票长期数据 (12小时)
    STOCK_KLINE = "stock.kline"
    STOCK_KLINE_DAILY = "stock.kline.daily"
    STOCK_FINANCIAL = "stock.financial"
    STOCK_HOLDER = "stock.holder"
    STOCK_HOLDERS = "stock.holders"
    STOCK_INDUSTRY = "stock.industry"

    # 股票持久数据 (不过期/每日更新)
    STOCK_INFO = "stock.info"
    STOCK_DETAIL = "stock.detail"
    STOCK_LIST = "stock.list"
    STOCK_ANNOUNCEMENTS = "stock.announcements"


# TTL 配置 (秒)
class CacheTTL:
    """缓存TTL配置"""

    REALTIME = 5  # 实时数据: 5秒
    SHORT = 300  # 短期数据: 5分钟
    MEDIUM = 1800  # 中期数据: 30分钟
    LONG = 43200  # 长期数据: 12小时
    
    # 具体业务TTL映射
    QUOTE = SHORT  # 行情数据: 5分钟
    FUND_FLOW = SHORT  # 资金流向: 5分钟
    RANKING = 120  # 排行榜数据: 2分钟 (120秒)
    
    ANNOUNCEMENT = 3600  # 公告数据: 1小时 (3600秒) - 按小时更新
    
    KLINE_INTRADAY = MEDIUM  # 日内K线: 30分钟
    INDEX = MEDIUM  # 指数数据: 30分钟
    MARKET_STATUS = MEDIUM  # 市场状态: 30分钟
    
    KLINE_DAILY = 86400  # K线日线: 24小时
    FINANCIAL = 86400  # 财务数据: 24小时 - 基础数据按天更新
    HOLDER = 86400  # 股东数据: 24小时 - 基础数据按天更新
    HOLDERS = 86400  # 股东数据: 24小时 - 基础数据按天更新
    INDUSTRY = 86400  # 行业数据: 24小时 - 基础数据按天更新
    
    STOCK_INFO = 86400  # 股票信息: 24小时 - 基础数据按天更新
    STOCK_DETAIL = 86400  # 股票详情: 24小时 - 基础数据按天更新
    STOCK_LIST = 86400  # 股票列表: 24小时 - 基础数据按天更新
    
    ANNOUNCEMENTS = 3600  # 公告数据: 1小时 (3600秒) - 按小时更新
    
    PERSISTENT = 604800  # 持久数据: 7天 (最长保留)


T = TypeVar("T")


@dataclass
class HistoricalRecord:
    """历史记录数据类"""
    timestamp: datetime
    data: Any
    change_type: str = "update"  # update, insert, delete


class StockCacheManager:
    """
    股票数据缓存管理器

    提供统一的缓存访问接口，支持:
    - 多级TTL策略
    - 作用域隔离
    - 序列化/反序列化
    - 缓存预热
    - 过期数据回退

    注意：
    - 自动使用全局缓存实例，确保跨语言（Rust/Python）数据共享
    - 缓存路径由系统自动管理，无需手动指定
    """

    def __init__(self):
        """
        初始化缓存管理器

        内部使用全局缓存实例，确保 Rust 端和 Python 端访问同一份数据。
        """
        # 统一缓存系统，使用全局缓存实例
        self._cache: Optional[Any] = None
        self._initialized = False

        # 内存缓存 (用于超高频访问)
        self._memory_cache: Dict[str, Dict[str, Any]] = {}
        self._memory_cache_time: Dict[str, datetime] = {}
        self._memory_cache_ttl: Dict[str, int] = {}

        # 变动历史记录
        self._change_history: Dict[str, List[HistoricalRecord]] = {}
        self._max_history_size = 100  # 每个缓存键最多保留100条历史记录

    def is_initialized(self) -> bool:
        """检查缓存管理器是否已初始化"""
        return self._initialized

    async def close(self) -> None:
        """关闭缓存管理器"""
        if self._cache:
            try:
                # 清理资源
                if hasattr(self._cache, 'close'):
                    self._cache.close()
            except Exception as e:
                logger.warning(f"关闭缓存失败: {e}")
        
        # 清理内存缓存
        self._memory_cache.clear()
        self._memory_cache_time.clear()
        self._memory_cache_ttl.clear()
        self._change_history.clear()
        self._initialized = False
        logger.info("缓存管理器已关闭")

    async def initialize(self) -> bool:
        """初始化缓存系统（异步模式）"""
        if self._initialized:
            return True

        if CACHE_AVAILABLE and PyCacheInterface:
            try:
                # 创建缓存接口实例（内部自动使用全局缓存实例）
                self._cache = PyCacheInterface()
                self._initialized = True
                logger.info("✅ 股票缓存初始化成功（使用全局缓存实例，支持跨语言访问）")
                return True
            except Exception as e:
                logger.warning(f"缓存初始化失败: {e}")
                self._cache = None

        # 即使缓存不可用，也标记为已初始化，不影响主服务
        self._initialized = True
        logger.warning("⚠️ 缓存不可用，使用内存缓存模式")
        return True

    def initialize_sync(self) -> bool:
        """初始化缓存系统（同步模式，用于tokio任务）"""
        if self._initialized:
            return True

        if CACHE_AVAILABLE and PyCacheInterface:
            try:
                # 创建缓存接口实例（内部自动使用全局缓存实例）
                self._cache = PyCacheInterface()
                self._initialized = True
                logger.info("✅ 股票缓存同步初始化成功（使用全局缓存实例）")
                return True
            except Exception as e:
                logger.warning(f"缓存初始化失败: {e}")
                self._cache = None

        # 降级为内存缓存模式
        self._initialized = True
        logger.warning("⚠️ 缓存同步初始化完成（内存模式）")
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
        """根据作用域获取TTL - 更新为新的缓存策略"""
        ttl_map = {
            # 实时数据 - 5分钟流式更新
            CacheScope.STOCK_REALTIME: CacheTTL.REALTIME,
            CacheScope.STOCK_QUOTE: CacheTTL.QUOTE,
            CacheScope.STOCK_FUND_FLOW: CacheTTL.FUND_FLOW,
            
            # 公告数据 - 按小时更新
            CacheScope.STOCK_ANNOUNCEMENTS: CacheTTL.ANNOUNCEMENT,
            
            # 中期数据 - 30分钟
            CacheScope.STOCK_KLINE_INTRADAY: CacheTTL.KLINE_INTRADAY,
            CacheScope.STOCK_INDEX: CacheTTL.INDEX,
            CacheScope.STOCK_MARKET_STATUS: CacheTTL.MARKET_STATUS,
            
            # 基础数据 - 按天更新
            CacheScope.STOCK_KLINE_DAILY: CacheTTL.KLINE_DAILY,
            CacheScope.STOCK_FINANCIAL: CacheTTL.FINANCIAL,
            CacheScope.STOCK_HOLDER: CacheTTL.HOLDER,
            CacheScope.STOCK_INDUSTRY: CacheTTL.INDUSTRY,
            CacheScope.STOCK_INFO: CacheTTL.STOCK_INFO,
            CacheScope.STOCK_LIST: CacheTTL.STOCK_LIST,
        }
        return ttl_map.get(scope, CacheTTL.QUOTE)  # 默认使用QUOTE的5分钟TTL

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
            # 在同步上下文中使用同步初始化
            self.initialize_sync()

        cache_key = self._get_cache_key(scope, key)

        # 先查内存缓存
        if cache_key in self._memory_cache:
            cache_time = self._memory_cache_time.get(cache_key)
            ttl = self._memory_cache_ttl.get(cache_key) or self._get_ttl(scope)

            if cache_time:
                age = (datetime.now() - cache_time).total_seconds()
                if age < ttl:
                    return self._memory_cache[cache_key]
                elif include_stale:
                    return self._memory_cache[cache_key]

        # 查 seesea_core 缓存
        if self._cache:
            try:
                # 检查 PyCacheInterface 是否有 scope 方法
                if hasattr(self._cache, 'scope'):
                    scope_cache = self._cache.scope(scope)
                    
                    # 从Rust缓存获取原始字节数据
                    result = scope_cache.get(key)
                    if result:
                        # 反序列化字节数据
                        return self._deserialize(result)
                else:
                    # 降级：直接使用管理器方法（如果有的话）
                    if hasattr(self._cache, 'manager'):
                        manager = self._cache.manager()
                        if include_stale:
                            result = manager.get_include_stale(scope, key)
                            if result:
                                data, is_stale = result
                                return self._deserialize(data.data)
                        else:
                            result = manager.get(scope, key)
                            if result:
                                return self._deserialize(result.data)
            except Exception as e:
                logger.warning(f"缓存读取失败 {cache_key}: {e}")

        return None

    async def set(self, scope: str, key: str, data: Any, ttl: Optional[int] = None) -> bool:
        """设置缓存，支持变动历史记录"""
        try:
            # 拒绝None数据
            if data is None:
                logger.error("不允许设置None数据到缓存")
                return False
                
            cache_key = self._get_cache_key(scope, key)
            ttl = ttl or self._get_ttl(scope)
            
            # 获取旧数据用于历史记录
            old_data = None
            if cache_key in self._memory_cache:
                old_data = self._memory_cache[cache_key]
            
            # 序列化数据
            serialized_data = self._serialize(data)
            if serialized_data is None:
                return False
            
            # 内存缓存
            self._memory_cache[cache_key] = data
            self._memory_cache_time[cache_key] = datetime.now()
            self._memory_cache_ttl[cache_key] = ttl
            
            # 添加变动历史记录
            if old_data is not None:
                self._add_change_history(cache_key, old_data, data, "update")
            else:
                self._add_change_history(cache_key, None, data, "insert")
            
            # 使用新的scope功能写入Rust缓存（如果可用）
            if self._cache and hasattr(self._cache, 'scope'):
                try:
                    scope_cache = self._cache.scope(scope)
                    scope_cache.set(key, serialized_data, ttl)
                    logger.info(f"✅ 缓存已设置到Rust缓存: {cache_key}, TTL: {ttl}s")
                except Exception as e:
                    logger.warning(f"写入Rust缓存失败 {cache_key}: {e}，使用内存缓存")
            else:
                logger.debug(f"使用内存缓存模式: {cache_key}")
            
            return True
            
        except Exception as e:
            logger.error(f"设置缓存失败: {e}")
            return False

    async def set_bulk(self, scope: str, data_list: List[Dict[str, Any]], ttl: Optional[int] = None) -> bool:
        """批量设置缓存
        
        Args:
            scope: 缓存作用域
            data_list: 数据列表，每个元素应包含 'code' 字段作为键
            ttl: TTL覆盖值
            
        Returns:
            是否全部成功
        """
        try:
            success_count = 0
            for item in data_list:
                if 'code' in item:
                    key = item['code']
                    if await self.set(scope, key, item, ttl):
                        success_count += 1
            
            logger.info(f"批量缓存设置完成: {success_count}/{len(data_list)} 成功")
            return success_count == len(data_list)
            
        except Exception as e:
            logger.error(f"批量设置缓存失败: {e}")
            return False

    async def get_bulk(self, scope: str, keys: List[str]) -> List[Optional[Any]]:
        """批量获取缓存
        
        Args:
            scope: 缓存作用域
            keys: 键列表
            
        Returns:
            数据列表，不存在的键对应 None
        """
        try:
            result = []
            for key in keys:
                data = await self.get(scope, key)
                result.append(data)
            return result
            
        except Exception as e:
            logger.error(f"批量获取缓存失败: {e}")
            return [None] * len(keys)

    async def exists(self, scope: str, key: str) -> bool:
        """检查缓存是否存在
        
        Args:
            scope: 缓存作用域
            key: 缓存键
            
        Returns:
            是否存在
        """
        try:
            data = await self.get(scope, key)
            return data is not None
        except Exception as e:
            logger.error(f"检查缓存存在失败: {e}")
            return False

    async def get_all_keys(self, scope: str) -> List[str]:
        """获取指定作用域的所有键
        
        Args:
            scope: 缓存作用域
            
        Returns:
            键列表
        """
        try:
            prefix = f"{scope}:"
            keys = []
            for cache_key in self._memory_cache.keys():
                if cache_key.startswith(prefix):
                    # 移除前缀返回原始键
                    key = cache_key[len(prefix):]
                    keys.append(key)
            return keys
        except Exception as e:
            logger.error(f"获取所有键失败: {e}")
            return []

    async def get_stats(self, scope: str) -> Dict[str, Any]:
        """获取缓存统计信息
        
        Args:
            scope: 缓存作用域
            
        Returns:
            统计信息字典
        """
        try:
            keys = await self.get_all_keys(scope)
            total_keys = len(keys)
            
            # 计算内存使用量（粗略估计）
            memory_usage = 0
            prefix = f"{scope}:"
            for cache_key, data in self._memory_cache.items():
                if cache_key.startswith(prefix):
                    # 粗略估计：字符串长度 + 一些开销
                    memory_usage += len(str(data)) * 2  # 假设每个字符2字节
            
            return {
                "total_keys": total_keys,
                "memory_usage_bytes": memory_usage,
                "memory_usage_kb": memory_usage / 1024,
                "scope": scope
            }
        except Exception as e:
            logger.error(f"获取缓存统计失败: {e}")
            return {"total_keys": 0, "memory_usage_bytes": 0, "memory_usage_kb": 0, "scope": scope}

    async def delete(self, scope: str, key: str) -> bool:
        """删除缓存"""
        cache_key = self._get_cache_key(scope, key)
        
        # 检查键是否存在
        exists = cache_key in self._memory_cache
        if not exists and not self._cache:
            return False

        # 删除内存缓存
        removed_from_memory = self._memory_cache.pop(cache_key, None) is not None
        self._memory_cache_time.pop(cache_key, None)
        self._memory_cache_ttl.pop(cache_key, None)

        # 删除 seesea_core 缓存
        if self._cache:
            try:
                # 检查 PyCacheInterface 是否有 scope 方法
                if hasattr(self._cache, 'scope'):
                    scope_cache = self._cache.scope(scope)
                    scope_cache.delete(key)
                    logger.info(f"从Rust缓存删除: {cache_key}")
                else:
                    # 降级：直接使用管理器方法（如果有的话）
                    if hasattr(self._cache, 'manager'):
                        manager = self._cache.manager()
                        manager.delete(scope, key)
            except Exception as e:
                logger.warning(f"缓存删除失败 {cache_key}: {e}")
                return False

        return exists or removed_from_memory

    async def clear_scope(self, scope: str) -> bool:
        """清空指定作用域的缓存"""
        # 清理内存缓存
        prefix = f"{scope}:"
        keys_to_delete = [k for k in self._memory_cache if k.startswith(prefix)]
        for k in keys_to_delete:
            self._memory_cache.pop(k, None)
            self._memory_cache_time.pop(k, None)
            self._memory_cache_ttl.pop(k, None)

        # 清理 seesea_core 缓存
        if self._cache:
            try:
                # 检查 PyCacheInterface 是否有 scope 方法
                if hasattr(self._cache, 'scope'):
                    scope_cache = self._cache.scope(scope)
                    # 注意：Rust的ScopeCache没有clear方法，需要遍历删除
                    keys = await self.get_all_keys(scope)
                    for key in keys:
                        scope_cache.delete(key)
                    logger.info(f"清空Rust缓存作用域: {scope}")
                else:
                    # 降级：使用管理器清理指定作用域
                    if hasattr(self._cache, 'manager'):
                        manager = self._cache.manager()
                        # 注意：这里可能需要遍历键来删除，简化处理
                        pass
                return True
            except Exception as e:
                logger.warning(f"清空作用域失败 {scope}: {e}")

        return True

    async def cleanup_expired(self) -> bool:
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
                ttl = self._memory_cache_ttl.get(cache_key) or self._get_ttl(scope)
                age = (now - cache_time).total_seconds()

                if age > ttl:  # 超过TTL就清理
                    keys_to_delete.append(cache_key)

        for k in keys_to_delete:
            self._memory_cache.pop(k, None)
            self._memory_cache_time.pop(k, None)
            self._memory_cache_ttl.pop(k, None)
            cleaned += 1

        # 清理 seesea_core 缓存
        if self._cache:
            try:
                # 检查 PyCacheInterface 是否有 scope 方法
                if hasattr(self._cache, 'scope'):
                    # 遍历所有作用域清理
                    for scope in [
                        CacheScope.STOCK_REALTIME,
                        CacheScope.STOCK_QUOTE,
                        CacheScope.STOCK_KLINE_INTRADAY,
                    ]:
                        scope_cache = self._cache.scope(scope)
                        # 注意：Rust的ScopeCache没有cleanup_expired方法
                        # 需要遍历键来检查过期，这里简化处理
                        keys = await self.get_all_keys(scope)
                        for key in keys:
                            # 检查是否过期并删除
                            data = await self.get(scope, key)
                            if data is None:  # 已过期
                                scope_cache.delete(key)
                                cleaned += 1
                else:
                    # 降级：使用全局清理
                    if hasattr(self._cache, 'cleanup'):
                        count = self._cache.cleanup()
                        cleaned += count
            except Exception as e:
                logger.warning(f"清理过期缓存失败: {e}")

        logger.info(f"清理过期缓存完成: 清理了 {cleaned} 项")
        return True

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

    async def get_fund_flow(self, code: str) -> Optional[Dict]:
        """获取资金流向缓存"""
        return await self.get(CacheScope.STOCK_FUND_FLOW, code)

    async def set_fund_flow(self, code: str, data: Any) -> bool:
        """设置资金流向缓存"""
        return await self.set(CacheScope.STOCK_FUND_FLOW, code, data)

    async def get_financial(self, code: str) -> Optional[Dict]:
        """获取财务数据缓存"""
        return await self.get(CacheScope.STOCK_FINANCIAL, code)

    async def set_financial(self, code: str, data: Any) -> bool:
        """设置财务数据缓存"""
        return await self.set(CacheScope.STOCK_FINANCIAL, code, data)

    async def get_holder(self, code: str) -> Optional[Dict]:
        """获取股东数据缓存"""
        return await self.get(CacheScope.STOCK_HOLDER, code)

    async def set_holder(self, code: str, data: Any) -> bool:
        """设置股东数据缓存"""
        return await self.set(CacheScope.STOCK_HOLDER, code, data)

    async def get_industry(self, code: str) -> Optional[Dict]:
        """获取行业数据缓存"""
        return await self.get(CacheScope.STOCK_INDUSTRY, code)

    async def set_industry(self, code: str, data: Any) -> bool:
        """设置行业数据缓存"""
        return await self.set(CacheScope.STOCK_INDUSTRY, code, data)

    async def get_announcements(self, code: str) -> Optional[Dict]:
        """获取公告数据缓存"""
        return await self.get(CacheScope.STOCK_ANNOUNCEMENTS, code)

    async def set_announcements(self, code: str, data: Any) -> bool:
        """设置公告数据缓存"""
        return await self.set(CacheScope.STOCK_ANNOUNCEMENTS, code, data)

    async def get_stock_list(self, market: str) -> Optional[List[Dict]]:
        """获取股票列表缓存"""
        return await self.get(CacheScope.STOCK_LIST, market)

    async def set_stock_list(self, market: str, data: Any) -> bool:
        """设置股票列表缓存"""
        return await self.set(CacheScope.STOCK_LIST, market, data)

    async def get_market_status(self, market: str) -> Optional[Dict]:
        """获取市场状态缓存"""
        return await self.get(CacheScope.STOCK_MARKET_STATUS, market)

    async def set_market_status(self, market: str, data: Any) -> bool:
        """设置市场状态缓存"""
        return await self.set(CacheScope.STOCK_MARKET_STATUS, market, data)

    async def get_index(self, code: str) -> Optional[Dict]:
        """获取指数数据缓存"""
        return await self.get(CacheScope.STOCK_INDEX, code)

    async def set_index(self, code: str, data: Any) -> bool:
        """设置指数数据缓存"""
        return await self.set(CacheScope.STOCK_INDEX, code, data)

    async def get_ranking(self, ranking_type: str) -> Optional[List[Dict]]:
        """获取排行榜数据缓存"""
        return await self.get(CacheScope.STOCK_RANKING, ranking_type)

    async def set_ranking(self, ranking_type: str, data: Any) -> bool:
        """设置排行榜数据缓存"""
        return await self.set(CacheScope.STOCK_RANKING, ranking_type, data)

    def _add_change_history(self, cache_key: str, old_data: Optional[Any], new_data: Any, change_type: str) -> None:
        """添加变动历史记录"""
        if cache_key not in self._change_history:
            self._change_history[cache_key] = []
        
        record = HistoricalRecord(
            timestamp=datetime.now(),
            data=new_data,
            change_type=change_type
        )
        
        self._change_history[cache_key].append(record)
        
        # 限制历史记录大小
        if len(self._change_history[cache_key]) > self._max_history_size:
            self._change_history[cache_key].pop(0)

    async def is_cache_fresh(self, scope: str, key: str, max_age: Optional[int] = None) -> bool:
        """检查缓存是否仍然新鲜（未过期）
        
        Args:
            scope: 缓存作用域
            key: 缓存键
            max_age: 最大允许年龄（秒），None则使用作用域默认TTL
            
        Returns:
            缓存是否仍然新鲜可用
        """
        if max_age is None:
            max_age = self._get_ttl(scope)
        
        cache_key = self._get_cache_key(scope, key)
        
        # 检查内存缓存
        if cache_key in self._memory_cache_time:
            cache_time = self._memory_cache_time[cache_key]
            age = (datetime.now() - cache_time).total_seconds()
            return age < max_age
        
        # 检查外部缓存是否存在（如果存在则认为有效，因为TTL由底层管理）
        if await self.exists(scope, key):
            return True
            
        return False
    
    async def get_cache_age(self, scope: str, key: str) -> Optional[int]:
        """获取缓存年龄（秒）
        
        Args:
            scope: 缓存作用域
            key: 缓存键
            
        Returns:
            缓存年龄（秒），如果不存在则返回None
        """
        cache_key = self._get_cache_key(scope, key)
        
        # 检查内存缓存
        if cache_key in self._memory_cache_time:
            cache_time = self._memory_cache_time[cache_key]
            age = int((datetime.now() - cache_time).total_seconds())
            return age
        
        # 外部缓存无法获取准确年龄，返回None
        return None

    async def get_change_history(self, scope: str, key: str, limit: int = 10) -> List[HistoricalRecord]:
        """获取变动历史记录"""
        cache_key = self._get_cache_key(scope, key)
        return self._change_history.get(cache_key, [])[-limit:]

    async def get_all_change_history(self) -> Dict[str, List[HistoricalRecord]]:
        """获取所有变动历史记录"""
        return self._change_history.copy()

# 全局缓存管理器实例
_global_cache_manager: Optional[StockCacheManager] = None


async def get_cache_manager() -> StockCacheManager:
    """
    获取全局缓存管理器实例

    Returns:
        StockCacheManager: 缓存管理器实例（使用全局缓存）
    """
    global _global_cache_manager

    if _global_cache_manager is None:
        _global_cache_manager = StockCacheManager()
        await _global_cache_manager.initialize()

    return _global_cache_manager


async def initialize_cache() -> StockCacheManager:
    """
    初始化缓存系统

    Returns:
        StockCacheManager: 初始化后的缓存管理器实例
    """
    global _global_cache_manager

    if _global_cache_manager is not None:
        await _global_cache_manager.close()

    _global_cache_manager = StockCacheManager()
    await _global_cache_manager.initialize()

    return _global_cache_manager


async def close_cache() -> None:
    """关闭缓存系统"""
    global _global_cache_manager
    
    if _global_cache_manager is not None:
        await _global_cache_manager.close()
        _global_cache_manager = None