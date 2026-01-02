"""
SeeSea Stock Module - 股票数据模块

提供股票数据获取、缓存、处理和API接口功能。

主要功能:
- 股票数据获取和缓存
- API桥接（回调机制已废弃，使用缓存优先模式）
- 数据模型定义
- 调度器管理
- 客户端封装

注意: 回调注册机制已被废弃，当前使用缓存优先模式。
Rust端通过HTTP API直接访问缓存系统，不再使用回调机制。
"""

from . import cache, client, models, service
import os
from pathlib import Path
from typing import Optional

# 全局服务实例
_service_instance: Optional["service.StockService"] = None


def initialize_stock_service(cache_path: Optional[str] = None) -> bool:
    """
    初始化股票服务（共享函数）
    
    用于web服务器和命令行工具初始化股票服务。
    支持自定义缓存路径，如果未指定则使用默认路径。
    
    Args:
        cache_path: 缓存路径，None则使用默认路径
        
    Returns:
        bool: 初始化是否成功
        
    Example:
        >>> from seesea.stock import initialize_stock_service
        >>> success = initialize_stock_service()
        >>> if success:
        ...     print("股票服务初始化成功")
    """
    global _service_instance
    
    try:
        # 如果已初始化，直接返回成功
        if _service_instance is not None and _service_instance._initialized:
            return True
            
        # 获取默认缓存路径
        if cache_path is None:
            if os.name == 'nt':  # Windows
                cache_path = os.path.expandvars(r'%LOCALAPPDATA%\SeeSea\stock_cache')
            else:  # Unix-like
                cache_path = os.path.expandvars('$HOME/.local/share/seesea/stock_cache')
        
        # 确保缓存目录存在
        Path(cache_path).parent.mkdir(parents=True, exist_ok=True)
        
        # 创建股票服务实例
        print(f"🚀 初始化股票服务...")
        print(f"📁 缓存路径: {cache_path}")
        _service_instance = service.StockService(cache_path=cache_path)
        
        # 同步初始化服务（避免异步上下文问题）
        print(f"📊 开始预加载股票数据到缓存...")
        success = _service_instance.initialize_sync()
        
        if success:
            # 检查预加载是否完成
            if _service_instance.is_preload_complete():
                print(f"✅ 股票服务初始化成功，缓存已就绪")
                return True
            else:
                print(f"⚠️ 股票服务初始化完成，但预加载尚未完成")
                print(f"💡 提示：数据正在后台加载，请稍后再试")
                return True  # 服务初始化成功，但数据可能不完整
        else:
            print(f"⚠️ 股票服务初始化失败")
            return False
            
    except Exception as e:
        print(f"❌ 股票服务初始化失败: {e}")
        _service_instance = None
        return False


def get_stock_service() -> Optional["service.StockService"]:
    """
    获取已初始化的股票服务实例
    
    Returns:
        StockService: 已初始化的服务实例，如果未初始化则返回None
    """
    global _service_instance
    
    if _service_instance is None or not _service_instance._initialized:
        # 尝试自动初始化
        if initialize_stock_service():
            return _service_instance
        return None
    
    return _service_instance


def refresh_stock_cache() -> bool:
    """
    刷新股票缓存
    
    重新加载所有股票数据到缓存中。
    
    Returns:
        bool: 刷新是否成功
    """
    service = get_stock_service()
    if service is None:
        print("❌ 股票服务未初始化")
        return False
    
    try:
        print(f"🔄 开始刷新股票缓存...")
        # 重新初始化服务以刷新数据
        global _service_instance
        cache_path = _service_instance._cache._cache_path if _service_instance._cache else None
        _service_instance = None  # 重置实例
        return initialize_stock_service(cache_path)
    except Exception as e:
        print(f"❌ 刷新股票缓存失败: {e}")
        return False


__all__ = ["cache", "client", "models", "service", 
           "initialize_stock_service", "get_stock_service", "refresh_stock_cache"]