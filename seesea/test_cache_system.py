#!/usr/bin/env python
"""
测试缓存系统
"""

import asyncio
import logging
import sys
from datetime import datetime, timedelta

# 设置日志
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

async def test_cache_system():
    """测试缓存系统"""
    print("=== 测试缓存系统 ===\n")
    
    try:
        # 导入缓存模块
        from seesea.stock.cache import StockCacheManager, CacheScope, CacheTTL
        
        # 创建缓存管理器
        print("1. 创建缓存管理器...")
        cache_manager = StockCacheManager()
        
        # 初始化缓存
        print("2. 初始化缓存系统...")
        success = await cache_manager.initialize()
        if success:
            print("   ✓ 缓存初始化成功")
        else:
            print("   ✗ 缓存初始化失败")
            return False
        
        # 测试设置和获取缓存
        print("\n3. 测试缓存设置和获取...")
        test_data = {
            "code": "000001",
            "name": "平安银行",
            "price": 12.50,
            "timestamp": datetime.now().isoformat()
        }
        
        # 设置缓存
        set_success = await cache_manager.set(CacheScope.STOCK_INFO, "000001", test_data, CacheTTL.STOCK_INFO)
        if set_success:
            print("   ✓ 缓存设置成功")
        else:
            print("   ✗ 缓存设置失败")
            return False
        
        # 获取缓存
        cached_data = await cache_manager.get(CacheScope.STOCK_INFO, "000001")
        if cached_data:
            print(f"   ✓ 缓存获取成功: {cached_data}")
        else:
            print("   ✗ 缓存获取失败")
            return False
        
        # 测试缓存过期检查
        print("\n4. 测试缓存过期检查...")
        is_fresh = await cache_manager.is_cache_fresh(CacheScope.STOCK_INFO, "000001", CacheTTL.STOCK_INFO)
        if is_fresh:
            print("   ✓ 缓存未过期")
        else:
            print("   ✗ 缓存已过期")
        
        # 测试批量操作
        print("\n5. 测试批量缓存操作...")
        batch_data = [
            {"code": "000002", "name": "万科A", "price": 15.30},
            {"code": "000003", "name": "万科B", "price": 10.20},
            {"code": "000004", "name": "万科C", "price": 8.50}
        ]
        
        batch_success = await cache_manager.set_bulk(CacheScope.STOCK_INFO, batch_data, CacheTTL.STOCK_INFO)
        if batch_success:
            print("   ✓ 批量缓存设置成功")
        else:
            print("   ✗ 批量缓存设置失败")
        
        # 验证批量缓存
        batch_keys = ["000002", "000003", "000004"]
        batch_results = await cache_manager.get_bulk(CacheScope.STOCK_INFO, batch_keys)
        print(f"   批量获取结果: {len(batch_results)} 条记录")
        
        # 测试缓存统计
        print("\n6. 测试缓存统计...")
        stats = await cache_manager.get_stats(CacheScope.STOCK_INFO)
        print(f"   缓存统计: {stats}")
        
        # 关闭缓存
        print("\n7. 关闭缓存系统...")
        await cache_manager.close()
        print("   ✓ 缓存系统已关闭")
        
        print("\n=== 缓存系统测试完成 ===\n")
        return True
        
    except Exception as e:
        print(f"✗ 测试失败: {e}")
        import traceback
        traceback.print_exc()
        return False

async def test_stock_service_cache():
    """测试股票服务缓存集成"""
    print("=== 测试股票服务缓存集成 ===\n")
    
    try:
        # 导入股票服务和缓存模块
        from seesea.stock.service import StockService
        from seesea.stock.cache import CacheTTL
        
        print("1. 创建股票服务...")
        service = StockService(enable_cache=True, enable_scheduler=False)
        
        print("2. 初始化服务...")
        success = service.initialize_sync()
        if not success:
            print("   ✗ 服务初始化失败")
            return False
        
        print("   ✓ 服务初始化成功")
        
        print("\n3. 测试股票信息缓存...")
        # 获取股票信息
        stock_info = await service.get_stock_info("000001")
        if stock_info:
            print(f"   ✓ 获取股票信息成功: {stock_info.name}")
            
            # 再次获取，应该从缓存获取
            stock_info_cached = await service.get_stock_info("000001")
            if stock_info_cached:
                print("   ✓ 从缓存获取股票信息成功")
            else:
                print("   ✗ 从缓存获取股票信息失败")
        else:
            print("   ✗ 获取股票信息失败")
        
        print("\n4. 测试股票列表缓存...")
        # 获取股票列表
        stock_list = await service.get_stock_list("cn_a")
        if stock_list and len(stock_list) > 0:
            print(f"   ✓ 获取股票列表成功，包含 {len(stock_list)} 条记录")
            
            # 再次获取，应该从缓存获取
            stock_list_cached = await service.get_stock_list("cn_a")
            if stock_list_cached and len(stock_list_cached) > 0:
                print("   ✓ 从缓存获取股票列表成功")
            else:
                print("   ✗ 从缓存获取股票列表失败")
        else:
            print("   ✗ 获取股票列表失败")
        
        print("\n5. 测试缓存过期检查...")
        # 检查股票信息缓存是否新鲜
        is_fresh = await service._cache.is_cache_fresh("stock.info", "000001", CacheTTL.STOCK_INFO)
        if is_fresh:
            print("   ✓ 股票信息缓存未过期")
        else:
            print("   ✗ 股票信息缓存已过期")
        
        print("\n=== 股票服务缓存集成测试完成 ===\n")
        return True
        
    except Exception as e:
        print(f"✗ 测试失败: {e}")
        import traceback
        traceback.print_exc()
        return False

async def main():
    """主测试函数"""
    print("开始缓存系统测试...\n")
    
    # 测试基础缓存系统
    success1 = await test_cache_system()
    
    # 测试股票服务缓存集成
    success2 = await test_stock_service_cache()
    
    # 总结
    if success1 and success2:
        print("✅ 所有缓存测试通过")
        return 0
    else:
        print("❌ 部分缓存测试失败")
        return 1

if __name__ == "__main__":
    sys.exit(asyncio.run(main()))