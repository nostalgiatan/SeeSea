#!/usr/bin/env python3
"""
SeeSea Python SDK - Basic Search Example
基础搜索示例
"""

from seesea import SearchClient

def main():
    print("🌊 SeeSea Python SDK - 基础搜索示例\n")
    
    # 1. 创建搜索客户端
    client = SearchClient()
    print("✅ 搜索客户端已创建\n")
    
    # 2. 执行搜索
    query = "rust programming language"
    print(f"🔍 搜索: {query}")
    results = client.search(query, page=1, page_size=10)
    
    # 3. 显示结果
    print(f"\n📊 搜索结果:")
    print(f"  总结果数: {results['total_count']}")
    print(f"  查询耗时: {results['query_time_ms']}ms")
    print(f"  使用引擎: {', '.join(results['engines_used'])}")
    print(f"  缓存状态: {'✅ 缓存命中' if results['cached'] else '❌ 新查询'}")
    
    print(f"\n📝 前 5 个结果:")
    for i, item in enumerate(results['results'][:5], 1):
        print(f"\n  {i}. {item['title']}")
        print(f"     URL: {item['url']}")
        print(f"     评分: {item['score']:.2f}")
    
    # 4. 获取统计信息
    stats = client.get_stats()
    print(f"\n📈 搜索统计:")
    print(f"  总搜索次数: {stats['total_searches']}")
    print(f"  缓存命中: {stats['cache_hits']}")
    print(f"  缓存未命中: {stats['cache_misses']}")
    
    if stats['total_searches'] > 0:
        hit_rate = stats['cache_hits'] / (stats['cache_hits'] + stats['cache_misses']) * 100
        print(f"  缓存命中率: {hit_rate:.1f}%")

if __name__ == "__main__":
    main()
