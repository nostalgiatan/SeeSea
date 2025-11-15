#!/usr/bin/env python3
"""
SeeSea Python SDK - Advanced Usage Example
高级用法示例
"""

from seesea import SearchClient, Config
from seesea.utils import format_results, parse_query

def main():
    print("🌊 SeeSea Python SDK - 高级用法示例\n")
    
    # 1. 配置
    config = Config()
    config.debug = True
    config.max_results = 200
    print(f"⚙️ 配置: {config}\n")
    
    # 2. 查询解析
    complex_query = "python lang:en site:github.com"
    parsed = parse_query(complex_query)
    print(f"🔍 查询解析:")
    print(f"  原始查询: {complex_query}")
    print(f"  解析结果: {parsed}\n")
    
    # 3. 搜索
    client = SearchClient()
    results = client.search("python programming", page=1, page_size=20)
    
    # 4. 格式化结果
    formatted = format_results(results['results'], max_description_length=100)
    print(f"📝 格式化结果 (前3个):")
    for i, item in enumerate(formatted[:3], 1):
        print(f"\n  {i}. {item['title']}")
        print(f"     {item['url']}")
        print(f"     {item['description']}...")
        print(f"     评分: {item['score']:.3f}")
    
    # 5. 批量搜索
    queries = ["rust", "python", "javascript"]
    print(f"\n🔄 批量搜索:")
    for query in queries:
        result = client.search(query, page_size=5)
        print(f"  {query}: {result['total_count']} 个结果")

if __name__ == "__main__":
    main()
