#!/usr/bin/env python3
"""
股票缓存键名验证工具

检查 Python 端写入的缓存键名，并生成对应的 Rust 访问代码。
"""

import asyncio
import sys


async def verify_cache_keys():
    """验证缓存键名是否正确"""

    print("=" * 70)
    print("🔍 股票缓存键名验证")
    print("=" * 70)

    try:
        from seesea.stock import initialize_stock_service, get_stock_service
        from seesea.stock.cache import CacheScope
        from seesea_core import PyCacheInterface

        # 1. 初始化服务
        print("\n📦 步骤1: 初始化股票服务...")
        if not initialize_stock_service():
            print("❌ 服务初始化失败")
            return False

        service = get_stock_service()
        if not service or not service._cache:
            print("❌ 缓存未初始化")
            return False

        # 2. 检查预加载数据
        print("\n📋 步骤2: 检查预加载数据...")

        cache = service._cache
        rust_cache = PyCacheInterface()

        # 检查股票列表
        print("\n  检查: CacheScope.STOCK_LIST, key='list:cn_a'")
        stock_list = await cache.get(CacheScope.STOCK_LIST, "list:cn_a")
        if stock_list:
            print(f"    ✅ Python端: 找到 {len(stock_list)} 只股票")

            # Rust端验证
            scope_cache = rust_cache.scope(CacheScope.STOCK_LIST)
            raw_data = scope_cache.get("list:cn_a")
            if raw_data:
                import json
                rust_list = json.loads(raw_data.decode('utf-8'))
                print(f"    ✅ Rust端: 找到 {len(rust_list)} 只股票")
                print(f"    ✅ 数据一致性: {'通过' if len(stock_list) == len(rust_list) else '失败'}")
            else:
                print(f"    ❌ Rust端: 未找到数据")
                return False
        else:
            print(f"    ⚠️  数据未加载（可能是后台加载中）")

        # 3. 检查作用域键名映射
        print("\n🗺️  步骤3: 检查作用域键名映射...")

        scope_mappings = [
            ("STOCK_LIST", "stock.list"),
            ("STOCK_INFO", "stock.info"),
            ("STOCK_QUOTE", "stock.quote"),
            ("STOCK_FINANCIAL", "stock.financial"),
            ("STOCK_INDEX", "stock.index"),
        ]

        print("\n  Python CacheScope → Rust scope 映射:")
        for py_scope, expected_rust in scope_mappings:
            actual_scope = getattr(CacheScope, py_scope)
            if actual_scope == expected_rust:
                print(f"    ✅ CacheScope.{py_scope} = '{actual_scope}'")
            else:
                print(f"    ❌ CacheScope.{py_scope} = '{actual_scope}' (期望: '{expected_rust}')")

        # 4. 生成 Rust 访问代码示例
        print("\n" + "=" * 70)
        print("📝 Rust 端访问代码示例")
        print("=" * 70)
        print("""
// 获取股票列表
let cache = CacheInterface::new(config)?;
let list_cache = cache.scope("stock.list");
let data = list_cache.get("list:cn_a")?;

// 获取单只股票信息
let info_cache = cache.scope("stock.info");
let stock_info = info_cache.get("000001")?;

// 获取股票行情
let quote_cache = cache.scope("stock.quote");
let quote = quote_cache.get("000001")?;

// 获取K线数据
let kline_cache = cache.scope("stock.kline");
let klines = kline_cache.get("000001:daily")?;
        """)

        print("=" * 70)
        print("✅ 验证完成")
        print("=" * 70)

        return True

    except Exception as e:
        print(f"\n❌ 验证失败: {e}")
        import traceback
        traceback.print_exc()
        return False


async def main():
    """主函数"""
    success = await verify_cache_keys()
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    asyncio.run(main())
