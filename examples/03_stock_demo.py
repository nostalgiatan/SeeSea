#!/usr/bin/env python3
"""
SeeSea 股票数据示例
演示如何使用 StockClient 获取股票数据
"""

from seesea import StockClient


def main():
    """股票示例主函数"""

    print("📈 SeeSea 股票数据示例\n")

    # 创建股票客户端
    print("1. 创建股票客户端...")
    client = StockClient()
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
        print(f"   核心可用: {info.get('core_available', 'N/A')}")
        print(f"   API 来源: {info.get('api_source', 'N/A')}")
        print()

    # 健康检查
    print("4. 健康检查...")
    health_result = client.health_check()
    if health_result.success:
        health = health_result.data
        print(f"   状态: {health.get('status', 'N/A')}")
        print(f"   响应时间: {health.get('response_time', 'N/A')}ms")
        print()

    # 搜索股票
    print("5. 搜索股票: '平安银行'...")
    search_result = client.search(keyword="平安银行")
    if search_result.success:
        stocks = search_result.data
        print(f"   ✓ 找到 {len(stocks)} 只股票:\n")
        for i, stock in enumerate(stocks[:3], 1):
            name = stock.get("name", "N/A")
            code = stock.get("code", "N/A")
            print(f"   {i}. {name} ({code})\n")
    else:
        print(f"   ✗ 搜索失败: {search_result.error}\n")

    # 获取股票列表（限制数量）
    print("6. 获取股票列表（限制10只）...")
    stocks_result = client.get_stock_list(market="a")
    if stocks_result.success:
        stocks = stocks_result.data
        print(f"   ✓ 找到 {len(stocks)} 只股票，显示前10只:\n")
        for i, stock in enumerate(stocks[:10], 1):
            name = stock.get("name", "N/A")
            code = stock.get("code", "N/A")
            print(f"   {i}. {name} ({code})\n")
    else:
        print(f"   ✗ 获取失败: {stocks_result.error}\n")

    # 获取指数列表
    print("7. 获取指数列表...")
    indices_result = client.get_index_list()
    if indices_result.success:
        indices = indices_result.data
        print(f"   ✓ 找到 {len(indices)} 个指数:\n")
        for i, index in enumerate(indices[:5], 1):
            name = index.get("name", "N/A")
            code = index.get("code", "N/A")
            print(f"   {i}. {name} ({code})\n")
    else:
        print(f"   ✗ 获取失败: {indices_result.error}\n")

    # 获取行业板块列表
    print("8. 获取行业板块列表...")
    industries_result = client.get_industry_list()
    if industries_result.success:
        industries = industries_result.data
        print(f"   ✓ 找到 {len(industries)} 个行业板块:\n")
        for i, industry in enumerate(industries[:5], 1):
            name = industry.get("name", "N/A")
            code = industry.get("code", "N/A")
            print(f"   {i}. {name} ({code})\n")
    else:
        print(f"   ✗ 获取失败: {industries_result.error}\n")

    # 断开连接
    print("9. 断开连接...")
    client.disconnect()
    print("   ✓ 已断开连接\n")


if __name__ == "__main__":
    main()
