#!/usr/bin/env python3
"""
SeeSea 搜索示例
演示如何使用 SearchClient 进行搜索
"""

from seesea import SearchClient


def main():
    """搜索示例主函数"""

    print("🌊 SeeSea 搜索示例\n")

    # 创建搜索客户端
    print("1. 创建搜索客户端...")
    client = SearchClient()
    print("   ✓ 客户端创建成功\n")

    # 连接到服务器
    print("2. 连接到服务器...")
    result = client.connect()
    if not result.success:
        print(f"   ✗ 连接失败: {result.error}\n")
        return
    print("   ✓ 连接成功\n")

    # 获取客户端信息
    print("3. 获取客户端信息...")
    info_result = client.get_info()
    if info_result.success:
        info = info_result.data
        print(f"   版本: {info.get('version', 'N/A')}")
        print(f"   可用引擎数: {info.get('engines_count', 'N/A')}")
        print()

    # 执行搜索
    print("4. 执行搜索: '人工智能'")
    search_result = client.search(
        query="人工智能", page=1, page_size=5, language="zh-CN", engine_type="text"
    )

    if search_result.success and search_result.data:
        items = search_result.data.items
        print(f"   ✓ 找到 {len(items)} 条结果:\n")
        for i, item in enumerate(items, 1):
            title = item.title
            url = item.url
            print(f"   {i}. {title}")
            print(f"      {url}\n")
    else:
        print("   ⚠ 未找到结果\n")

    # 获取统计信息
    print("5. 获取统计信息...")
    stats_result = client.get_stats()
    if stats_result.success:
        stats = stats_result.data
        print(f"   缓存命中率: {stats.get('cache_hit_rate', 'N/A')}")
        print()

    # 列出可用引擎
    print("6. 列出可用引擎...")
    engines_result = client.list_engines()
    if engines_result.success:
        engines = engines_result.data
        print(f"   总计: {engines.get('total', 0)} 个引擎")
        print(f"   健康: {engines.get('healthy', 0)} 个\n")

    # 断开连接
    print("7. 断开连接...")
    client.disconnect()
    print("   ✓ 已断开连接\n")


if __name__ == "__main__":
    main()
