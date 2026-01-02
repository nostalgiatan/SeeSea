#!/usr/bin/env python3
"""
股票缓存跨语言访问测试

演示如何通过作用域机制实现 Rust 和 Python 之间的缓存共享。

测试场景:
1. Python 端写入股票数据到缓存
2. 验证 Rust 端可以读取相同数据
3. 展示作用域隔离机制
"""

import asyncio
import json
from datetime import datetime


async def test_stock_cache_cross_language():
    """测试股票缓存跨语言访问"""

    print("=" * 60)
    print("🧪 股票缓存跨语言访问测试")
    print("=" * 60)

    # 1. 初始化股票服务（自动使用全局缓存实例）
    print("\n📦 步骤1: 初始化股票服务...")
    from seesea.stock import initialize_stock_service, get_stock_service

    success = initialize_stock_service()
    if not success:
        print("❌ 初始化失败")
        return False

    service = get_stock_service()
    cache_manager = service._cache

    # 2. Python 端写入测试数据
    print("\n✍️  步骤2: Python 端写入股票数据...")
    test_stock_data = {
        "code": "000001",
        "name": "平安银行",
        "price": 12.34,
        "change_pct": 2.5,
        "timestamp": datetime.now().isoformat()
    }

    # 使用股票行情作用域
    from seesea.stock.cache import CacheScope, CacheTTL
    await cache_manager.set(
        scope=CacheScope.STOCK_QUOTE,
        key="000001",
        data=test_stock_data,
        ttl=CacheTTL.QUOTE
    )
    print(f"✅ 已写入股票数据到作用域: {CacheScope.STOCK_QUOTE}")
    print(f"   数据: {test_stock_data}")

    # 3. Python 端读取验证
    print("\n🔍 步骤3: Python 端读取验证...")
    cached_data = await cache_manager.get(
        scope=CacheScope.STOCK_QUOTE,
        key="000001"
    )
    if cached_data:
        print(f"✅ Python 端读取成功: {cached_data}")
    else:
        print("❌ Python 端读取失败")
        return False

    # 4. 直接通过 Rust 缓存接口访问
    print("\n🦀 步骤4: 通过 Rust 缓存接口访问...")
    from seesea_core import PyCacheInterface

    rust_cache = PyCacheInterface()  # 自动使用全局实例
    scope_cache = rust_cache.scope(CacheScope.STOCK_QUOTE)

    # 获取原始字节数据
    raw_bytes = scope_cache.get("000001")
    if raw_bytes:
        # 反序列化
        rust_data = json.loads(raw_bytes.decode('utf-8'))
        print(f"✅ Rust 缓存接口读取成功: {rust_data}")

        # 验证数据一致性
        if rust_data == test_stock_data:
            print("✅ 数据一致性验证通过！")
        else:
            print("❌ 数据不一致")
            return False
    else:
        print("❌ Rust 缓存接口读取失败")
        return False

    # 5. 测试多个作用域隔离
    print("\n🔐 步骤5: 测试作用域隔离...")

    # 写入不同作用域
    await cache_manager.set(
        scope=CacheScope.STOCK_INFO,
        key="000001",
        data={"code": "000001", "name": "平安银行", "type": "股票基础信息"},
        ttl=CacheTTL.STOCK_INFO
    )

    await cache_manager.set(
        scope=CacheScope.STOCK_FINANCIAL,
        key="000001",
        data={"code": "000001", "revenue": 1000000, "type": "财务数据"},
        ttl=CacheTTL.FINANCIAL
    )

    # 验证作用域隔离
    quote_data = await cache_manager.get(CacheScope.STOCK_QUOTE, "000001")
    info_data = await cache_manager.get(CacheScope.STOCK_INFO, "000001")
    financial_data = await cache_manager.get(CacheScope.STOCK_FINANCIAL, "000001")

    print(f"   行情作用域: {quote_data.get('type', '行情数据')}")
    print(f"   信息作用域: {info_data.get('type', '信息数据')}")
    print(f"   财务作用域: {financial_data.get('type', '财务数据')}")

    if quote_data and info_data and financial_data:
        if quote_data != info_data != financial_data:
            print("✅ 作用域隔离验证通过！")
        else:
            print("❌ 作用域隔离失败")
            return False

    # 6. 展示 Rust 端访问示例代码
    print("\n" + "=" * 60)
    print("📝 Rust 端访问示例代码:")
    print("=" * 60)
    print("""
// Rust 代码示例
use seesea_core::CacheInterface;

async fn get_stock_quote(code: &str) -> Result<StockQuote> {
    // 获取缓存接口
    let cache = CacheInterface::new(config)?;

    // 获取股票行情作用域
    let quote_cache = cache.scope("stock.quote");

    // 读取股票数据
    if let Some(data) = quote_cache.get(code)? {
        let quote: StockQuote = serde_json::from_slice(&data)?;
        Ok(quote)
    } else {
        Err("股票数据不存在")
    }
}
    """)

    print("=" * 60)
    print("✅ 所有测试通过！跨语言缓存访问功能正常！")
    print("=" * 60)

    # 7. 显示缓存统计
    print("\n📊 缓存统计信息:")
    stats = rust_cache.get_stats()
    print(f"   命中次数: {stats['hits']}")
    print(f"   未命中次数: {stats['misses']}")
    print(f"   写入次数: {stats['writes']}")
    print(f"   命中率: {stats['hit_rate']:.2%}")

    return True


async def main():
    """主函数"""
    try:
        success = await test_stock_cache_cross_language()

        if success:
            print("\n🎉 测试成功！")
            print("\n💡 要点总结:")
            print("   1. Python 和 Rust 共享同一个缓存数据库")
            print("   2. 通过作用域 (scope) 实现数据隔离")
            print("   3. Rust 端可以直接访问 Python 端写入的数据")
            print("   4. 支持多个作用域并发访问")
        else:
            print("\n❌ 测试失败")

    except Exception as e:
        print(f"\n❌ 测试异常: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    asyncio.run(main())
