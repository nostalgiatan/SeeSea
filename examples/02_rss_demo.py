#!/usr/bin/env python3
"""
SeeSea RSS 订阅示例
演示如何使用 RssClient 获取和管理 RSS 订阅源
"""

from seesea import RssClient


def main():
    """RSS 示例主函数"""

    print("📰 SeeSea RSS 订阅示例\n")

    # 创建 RSS 客户端
    print("1. 创建 RSS 客户端...")
    client = RssClient()
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
        print()

    # 列出可用模板
    print("4. 列出可用模板...")
    templates_result = client.list_templates()
    if templates_result.success:
        templates = templates_result.data
        print(f"   ✓ 找到 {len(templates)} 个模板:\n")
        for template in templates:
            print(f"   - {template}\n")

    # 获取模板信息
    if templates_result.success and templates_result.data:
        print("5. 获取模板详情...")
        template_name = templates_result.data[0]
        template_info_result = client.get_template_info(template_name=template_name)
        if template_info_result.success:
            template_info = template_info_result.data
            print(f"   模板名称: {template_name}")
            print(f"   描述: {template_info.get('description', 'N/A')}")
            feeds = template_info.get("feeds", [])
            print(f"   Feed 数量: {len(feeds)}\n")
            if feeds:
                print("   示例 Feed:")
                for i, feed in enumerate(feeds[:2], 1):
                    print(f"   {i}. {feed.get('name', 'N/A')}")
                    print(f"      URL: {feed.get('url', 'N/A')}\n")

    # 获取 RSS feed
    print("6. 获取 RSS feed 内容...")
    feed_url = "https://www.people.com.cn/rss/politics.xml"
    fetch_result = client.fetch_feed(url=feed_url)
    if fetch_result.success:
        feed_data = fetch_result.data
        print(f"   ✓ 获取到 feed: {feed_data.get('title', 'N/A')}")
        print(f"   描述: {feed_data.get('description', 'N/A')[:100]}...")
        items = feed_data.get("items", [])
        print(f"   文章数量: {len(items)}\n")

        # 显示前3篇文章
        for i, item in enumerate(items[:3], 1):
            title = item.get("title", "N/A")
            link = item.get("link", "N/A")
            published = item.get("published", "N/A")
            print(f"   {i}. {title}")
            print(f"      发布时间: {published}")
            print(f"      链接: {link}\n")
    else:
        print(f"   ✗ 获取失败: {fetch_result.error}\n")

    # 解析 RSS 内容
    print("7. 解析 RSS 内容...")
    sample_rss = """<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
  <title>示例 Feed</title>
  <description>这是示例描述</description>
  <item>
    <title>示例文章1</title>
    <link>https://example.com/1</link>
    <pubDate>Mon, 27 Jan 2026 08:00:00 GMT</pubDate>
  </item>
</channel>
</rss>"""
    parse_result = client.parse_feed(content=sample_rss)
    if parse_result.success:
        parsed_data = parse_result.data
        print("   ✓ 解析成功")
        print(f"   标题: {parsed_data.get('title', 'N/A')}")
        items = parsed_data.get("items", [])
        print(f"   文章数量: {len(items)}\n")
    else:
        print(f"   ✗ 解析失败: {parse_result.error}\n")

    # 健康检查
    print("8. 健康检查...")
    health_result = client.health_check()
    if health_result.success:
        health = health_result.data
        print(f"   状态: {health.get('status', 'N/A')}")
        print()

    # 断开连接
    print("9. 断开连接...")
    client.disconnect()
    print("   ✓ 已断开连接\n")


if __name__ == "__main__":
    main()
