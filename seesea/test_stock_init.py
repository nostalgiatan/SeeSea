#!/usr/bin/env python3
"""测试股票服务新的初始化流程"""

import asyncio
import sys
import time
from pathlib import Path

# 添加项目路径
sys.path.insert(0, str(Path(__file__).parent))

from seesea.stock.service import StockService

def test_initialization():
    """测试初始化流程"""
    print("=== 测试股票服务初始化流程 ===\n")
    
    try:
        # 创建股票服务实例
        print("1. 创建股票服务实例...")
        service = StockService(enable_cache=True, enable_scheduler=False)
        print("   ✓ 股票服务实例创建成功\n")
        
        # 测试同步初始化
        print("2. 开始同步初始化（阻塞）...")
        start_time = time.time()
        
        success = service.initialize_sync()
        
        elapsed_time = time.time() - start_time
        print(f"   ✓ 初始化完成，耗时: {elapsed_time:.2f} 秒\n")
        
        if not success:
            print("   ✗ 初始化失败")
            return False
        
        # 验证股票列表是否已加载
        print("3. 验证股票列表是否已加载...")
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        
        try:
            stock_list_data = loop.run_until_complete(
                service._cache.get("stock.list", "cn_a")
            )
            
            if stock_list_data:
                print(f"   ✓ 股票列表已加载，包含 {len(stock_list_data)} 条数据")
                print(f"   前3只股票: {[s.get('code') for s in stock_list_data[:3]]}")
            else:
                print("   ✗ 股票列表未加载")
                return False
        finally:
            loop.close()
        
        print()
        
        # 等待一段时间，观察后台任务执行
        print("4. 等待后台任务执行...")
        print("   后台任务将并发获取股票详细信息...")
        print()
        
        # 等待后台任务完成（最多60秒）
        service.wait_for_background_tasks(timeout=60)
        
        # 检查任务队列状态
        print("5. 检查任务队列状态...")
        if service._task_queue:
            stats = service._task_queue.get_stats()
            progress = service._task_queue.get_progress()
            
            print(f"   队列大小: {stats['queue_size']}")
            print(f"   运行中任务: {stats['running_tasks']}")
            print(f"   已完成任务: {stats['completed_tasks']}")
            print(f"   失败任务: {stats['failed_tasks']}")
            print(f"   总进度: {progress['completed']}/{progress['total']} ({progress['progress']:.1%})")
        else:
            print("   ⚠ 任务队列未初始化")
        
        print()
        
        # 验证缓存中是否有股票详细信息
        print("6. 验证缓存中的股票详细信息...")
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        
        try:
            # 检查前10只股票的信息缓存
            test_codes = [s.get('code') for s in stock_list_data[:10]]
            cached_count = 0
            
            for code in test_codes:
                info = loop.run_until_complete(
                    service._cache.get("stock.info", code)
                )
                if info:
                    cached_count += 1
            
            print(f"   前10只股票中，{cached_count} 只已有详细信息缓存")
            
        finally:
            loop.close()
        
        print()
        print("=== 测试完成 ===\n")
        return True
        
    except Exception as e:
        print(f"✗ 测试失败: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = test_initialization()
    sys.exit(0 if success else 1)
