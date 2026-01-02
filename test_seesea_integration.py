#!/usr/bin/env python3
"""
测试脚本：验证 seesea Python SDK 能否完整调用 Rust 库
"""

import sys
import traceback


def test_imports():
    """测试所有导入是否正常"""
    print("=" * 60)
    print("测试 1: 导入 Rust 核心模块")
    print("=" * 60)
    
    try:
        from seesea_core import (
            PySearchClient,
            PyApiServer,
            PyConfig,
            PyCacheStats,
            PyCacheInterface,
            PyRssClient,
            PyBrowserConfig,
            PyBrowserEngineClient,
            PyNetClient,
            register_engine,
            unregister_engine,
            list_engines,
            has_engine,
            get,
            post,
            get_file,
            post_file,
        )
        print("✅ 所有 Rust 核心模块导入成功")
        return True
    except ImportError as e:
        print(f"❌ 导入失败: {e}")
        traceback.print_exc()
        return False


def test_seesea_imports():
    """测试 seesea 包的导入"""
    print("\n" + "=" * 60)
    print("测试 2: 导入 seesea Python 包")
    print("=" * 60)
    
    try:
        from seesea import (
            SearchClient,
            RssClient,
            BrowserEngineClient,
            BrowserConfig,
            BrowserEngine,
            BaseBrowserEngine,
            XinhuaEngine,
            ApiServer,
            Config,
            SearchResponse,
            SearchResultItem,
            EngineState,
            CacheInfo,
            SearchStats,
            PrivacyStats,
            format_results,
            parse_query,
        )
        print("✅ 所有 seesea Python 模块导入成功")
        return True
    except ImportError as e:
        print(f"❌ 导入失败: {e}")
        traceback.print_exc()
        return False


def test_search_client():
    """测试搜索客户端"""
    print("\n" + "=" * 60)
    print("测试 3: 搜索客户端")
    print("=" * 60)
    
    try:
        from seesea import SearchClient
        
        client = SearchClient()
        print(f"✅ SearchClient 创建成功: {client}")
        
        # 测试引擎注册
        from seesea_core import list_engines
        engines = list_engines()
        print(f"✅ 已注册引擎数量: {len(engines)}")
        
        return True
    except Exception as e:
        print(f"❌ 测试失败: {e}")
        traceback.print_exc()
        return False


def test_config():
    """测试配置"""
    print("\n" + "=" * 60)
    print("测试 4: 配置系统")
    print("=" * 60)
    
    try:
        from seesea import Config
        from seesea_core import PyConfig
        
        # 测试 PyConfig
        py_config = PyConfig()
        print(f"✅ PyConfig 创建成功")
        
        # 测试 Config
        config = Config()
        print(f"✅ Config 创建成功: {config}")
        
        return True
    except Exception as e:
        print(f"❌ 测试失败: {e}")
        traceback.print_exc()
        return False


def test_rss_client():
    """测试 RSS 客户端"""
    print("\n" + "=" * 60)
    print("测试 5: RSS 客户端")
    print("=" * 60)
    
    try:
        from seesea import RssClient
        
        client = RssClient()
        print(f"✅ RssClient 创建成功: {client}")
        
        return True
    except Exception as e:
        print(f"❌ 测试失败: {e}")
        traceback.print_exc()
        return False


def test_browser_client():
    """测试浏览器客户端"""
    print("\n" + "=" * 60)
    print("测试 6: 浏览器客户端")
    print("=" * 60)
    
    try:
        from seesea import BrowserEngineClient, BrowserConfig
        
        config = BrowserConfig(headless=True, stealth=False)
        print(f"✅ BrowserConfig 创建成功: {config}")
        
        client = BrowserEngineClient(config)
        print(f"✅ BrowserEngineClient 创建成功: {client}")
        
        return True
    except Exception as e:
        print(f"❌ 测试失败: {e}")
        traceback.print_exc()
        return False


def test_api_server():
    """测试 API 服务器"""
    print("\n" + "=" * 60)
    print("测试 7: API 服务器")
    print("=" * 60)
    
    try:
        from seesea import ApiServer
        
        # 创建服务器实例（不启动）
        server = ApiServer(host="127.0.0.1", port=8080)
        print(f"✅ ApiServer 创建成功: {server}")
        print(f"   地址: {server.address}")
        print(f"   URL: {server.url}")
        
        # 获取端点列表
        endpoints = server.get_endpoints()
        print(f"✅ 获取端点列表成功，共 {len(endpoints)} 个分类")
        
        return True
    except Exception as e:
        print(f"❌ 测试失败: {e}")
        traceback.print_exc()
        return False


def test_network_client():
    """测试网络客户端"""
    print("\n" + "=" * 60)
    print("测试 8: 网络客户端")
    print("=" * 60)
    
    try:
        from seesea_core import PyNetClient
        
        client = PyNetClient()
        print(f"✅ PyNetClient 创建成功: {client}")
        
        return True
    except Exception as e:
        print(f"❌ 测试失败: {e}")
        traceback.print_exc()
        return False


def test_type_safety():
    """测试类型安全"""
    print("\n" + "=" * 60)
    print("测试 9: 类型安全")
    print("=" * 60)
    
    try:
        from seesea import (
            SearchResultItem,
            SearchResponse,
            EngineState,
            CacheInfo,
            SearchStats,
            PrivacyStats,
        )
        
        # 创建测试对象
        item = SearchResultItem(
            title="Test Title",
            url="https://example.com",
            snippet="Test snippet",
            score=0.95,
            engine="test_engine",
        )
        print(f"✅ SearchResultItem 创建成功: {item}")
        
        response = SearchResponse(
            query="test",
            total_count=1,
            results=[item],
            engines_used=["test_engine"],
            execution_time_ms=100,
        )
        print(f"✅ SearchResponse 创建成功")
        
        return True
    except Exception as e:
        print(f"❌ 测试失败: {e}")
        traceback.print_exc()
        return False


def main():
    """运行所有测试"""
    print("\n" + "=" * 60)
    print("SeeSea Python SDK 集成测试")
    print("=" * 60)
    print(f"Python 版本: {sys.version}")
    print()
    
    tests = [
        ("导入 Rust 核心模块", test_imports),
        ("导入 seesea Python 包", test_seesea_imports),
        ("搜索客户端", test_search_client),
        ("配置系统", test_config),
        ("RSS 客户端", test_rss_client),
        ("浏览器客户端", test_browser_client),
        ("API 服务器", test_api_server),
        ("网络客户端", test_network_client),
        ("类型安全", test_type_safety),
    ]
    
    results = []
    for name, test_func in tests:
        try:
            success = test_func()
            results.append((name, success))
        except Exception as e:
            print(f"\n❌ 测试 '{name}' 发生异常: {e}")
            traceback.print_exc()
            results.append((name, False))
    
    # 打印总结
    print("\n" + "=" * 60)
    print("测试总结")
    print("=" * 60)
    
    passed = sum(1 for _, success in results if success)
    total = len(results)
    
    for name, success in results:
        status = "✅ 通过" if success else "❌ 失败"
        print(f"{status}: {name}")
    
    print()
    print(f"总计: {passed}/{total} 测试通过")
    
    if passed == total:
        print("\n🎉 所有测试通过！seesea Python SDK 可以完整调用 Rust 库。")
        return 0
    else:
        print(f"\n⚠️  {total - passed} 个测试失败，请检查上述错误信息。")
        return 1


if __name__ == "__main__":
    sys.exit(main())
