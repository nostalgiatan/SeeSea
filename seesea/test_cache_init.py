#!/usr/bin/env python3
"""测试缓存目录初始化和功能"""

import os
import sys
import shutil
from pathlib import Path

def cleanup_cache_dirs():
    """清理旧的缓存目录"""
    dirs_to_clean = [
        Path("D:/seesea"),
        Path(os.path.join(os.environ.get("TEMP", ""), "seesea")),
        Path(os.path.join(os.environ.get("TEMP", ""), "seesea_fallback")),
    ]
    
    for dir_path in dirs_to_clean:
        if dir_path.exists():
            try:
                print(f"  删除目录: {dir_path}")
                shutil.rmtree(dir_path)
                print(f"  ✓ 成功删除 {dir_path}")
            except Exception as e:
                print(f"  ✗ 删除 {dir_path} 失败: {e}")

def test_cache_directory():
    """测试缓存目录是否正确创建"""
    print("=== 测试缓存目录初始化 ===\n")
    
    # 清理旧的数据库文件
    print("清理旧的数据库文件...")
    cleanup_cache_dirs()
    
    # 导入 seesea_core
    try:
        import seesea_core
        print("✓ 成功导入 seesea_core")
    except Exception as e:
        print(f"✗ 导入 seesea_core 失败: {e}")
        return False
    
    # 测试缓存接口
    try:
        cache = seesea_core.PyCacheInterface(None, None, None)
        print("✓ 成功创建 PyCacheInterface 实例")
    except Exception as e:
        print(f"✗ 创建 PyCacheInterface 失败: {e}")
        import traceback
        traceback.print_exc()
        return False
    
    # 测试缓存统计
    try:
        stats = cache.get_stats()
        print(f"✓ 获取缓存统计信息成功:")
        if isinstance(stats, dict):
            print(f"  - 命中次数: {stats.get('cache_hits', 0)}")
            print(f"  - 未命中次数: {stats.get('cache_misses', 0)}")
            print(f"  - 总条目数: {stats.get('total_entries', 0)}")
        else:
            print(f"  - 命中次数: {stats.cache_hits}")
            print(f"  - 未命中次数: {stats.cache_misses}")
            print(f"  - 总条目数: {stats.total_entries}")
    except Exception as e:
        print(f"✗ 获取缓存统计信息失败: {e}")
        import traceback
        traceback.print_exc()
        return False
    
    # 检查缓存目录是否存在
    print("\n=== 检查缓存目录 ===\n")
    
    # Windows 系统 - 使用系统缓存目录
    if sys.platform == 'win32':
        import os
        # Windows 缓存目录通常是 C:\Users\<username>\AppData\Local\Cache\seesea
        # 或者是 temp 目录下的 seesea
        local_app_data = os.environ.get('LOCALAPPDATA', '')
        if local_app_data:
            expected_cache_dir = Path(local_app_data) / "Cache" / "seesea"
        else:
            expected_cache_dir = Path(os.environ.get('TEMP', '')) / "seesea"
    elif sys.platform == 'linux':
        expected_cache_dir = Path("/var/cache/seesea")
    elif sys.platform == 'darwin':
        # macOS - 使用用户缓存目录
        expected_cache_dir = Path.home() / "Library" / "Caches" / "seesea"
    else:
        expected_cache_dir = Path(".seesea/cache")
    
    print(f"预期缓存目录: {expected_cache_dir}")
    
    if expected_cache_dir.exists():
        print(f"✓ 缓存目录存在")
        
        # 检查数据库文件
        db_file = expected_cache_dir / "cache.db"
        if db_file.exists():
            print(f"✓ 缓存数据库文件存在: {db_file}")
            print(f"  文件大小: {db_file.stat().st_size} 字节")
        else:
            print(f"⚠ 缓存数据库文件不存在: {db_file}")
    else:
        print(f"⚠ 缓存目录不存在: {expected_cache_dir}")
        print(f"  这可能是正常的，如果使用了内存缓存或临时目录")
    
    # 测试缓存读写
    print("\n=== 测试缓存读写 ===\n")
    
    try:
        # 获取作用域缓存
        scope_cache = cache.scope("test_scope")
        print("✓ 获取作用域缓存成功")
        
        # 写入测试数据
        test_key = "test_key_12345"
        test_value = b"test_value_from_python"
        
        scope_cache.set(test_key, test_value, None)
        print(f"✓ 写入缓存: {test_key}")
        
        # 读取测试数据
        retrieved_value = scope_cache.get(test_key)
        if retrieved_value == test_value:
            print(f"✓ 读取缓存成功: {test_key}")
        else:
            print(f"✗ 读取缓存失败: 期望 {test_value}, 得到 {retrieved_value}")
            return False
        
        # 检查键是否存在
        exists = scope_cache.exists(test_key)
        if exists:
            print(f"✓ 键存在检查成功: {test_key}")
        else:
            print(f"✗ 键存在检查失败: {test_key}")
            return False
        
        # 删除测试数据
        deleted = scope_cache.delete(test_key)
        if deleted:
            print(f"✓ 删除缓存: {test_key}")
        else:
            print(f"✗ 删除缓存失败: {test_key}")
            return False
        
        # 验证删除
        exists = scope_cache.exists(test_key)
        if not exists:
            print(f"✓ 验证删除成功")
        else:
            print(f"✗ 删除失败: 数据仍然存在")
            return False
        
    except Exception as e:
        print(f"✗ 缓存读写测试失败: {e}")
        import traceback
        traceback.print_exc()
        return False
    
    print("\n=== 所有测试通过 ===\n")
    return True

def test_rss_cache():
    """测试 RSS 缓存功能"""
    print("\n=== 测试 RSS 缓存功能 ===\n")
    
    try:
        import seesea_core
        rss_client = seesea_core.PyRssClient()
        print("✓ 成功创建 PyRssClient 实例")
        
        # 获取 RSS 模板列表
        rss_list = rss_client.list_templates()
        print(f"✓ 获取 RSS 模板列表成功，共 {len(rss_list)} 个模板")
        
        # 尝试获取一个 RSS 源的内容
        if rss_list:
            first_rss = rss_list[0]
            print(f"  测试 RSS 模板: {first_rss}")
            
            try:
                content = rss_client.fetch_feed(first_rss)
                print(f"✓ 成功获取 RSS 内容")
                print(f"  标题数量: {len(content.get('items', []))}")
            except Exception as e:
                print(f"⚠ 获取 RSS 内容时出现错误（可能是网络问题）: {e}")
        
        return True
    except Exception as e:
        print(f"✗ RSS 缓存测试失败: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = True
    
    # 测试缓存目录
    if not test_cache_directory():
        success = False
    
    # 测试 RSS 缓存
    if not test_rss_cache():
        success = False
    
    sys.exit(0 if success else 1)
