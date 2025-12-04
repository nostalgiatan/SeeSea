#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
命令行工具：测试ContentProcessor处理链接的流程

用法：
python test_processor.py <url>

示例：
python test_processor.py https://example.com
"""

import sys
from seesea.Pro.content_processor import ContentProcessor


def main():
    """主函数，处理命令行参数并执行处理流程"""
    # 检查命令行参数
    if len(sys.argv) != 2:
        print("用法: python test_processor.py <url>")
        print("示例: python test_processor.py https://example.com")
        sys.exit(1)

    url = sys.argv[1]
    print(f"\n正在处理链接: {url}\n")

    # 创建ContentProcessor实例
    processor = ContentProcessor()

    try:
        # 使用同步方法处理URL
        result = processor.get_pure_data(url)

        # 输出处理结果
        print("=== 处理结果 ===")
        print(f"URL: {result['url']}")
        print(f"页面类型: {result['page_type']}")
        print(f"标题: {result['metadata'].get('title', 'N/A')}")
        print(f"描述: {result['metadata'].get('description', 'N/A')}")
        print(f"HTML大小: {len(result['html'])} 字符")
        print(f"Markdown大小: {len(result['markdown'])} 字符")
        print(f"清理后Markdown大小: {len(result['cleaned_markdown'])} 字符")
        print(f"数据块数量: {len(result['cleaned_data'])}")

        print("\n=== 清理后数据块示例 ===")
        # 输出前3个数据块
        for i, data_block in enumerate(result["cleaned_data"][:3]):
            print(f"\n数据块 {i+1}:")
            print(f"类型: {data_block.get('type', 'N/A')}")
            print(f"内容: {data_block.get('content', '').strip()[:200]}...")
            print(f"相关度: {data_block.get('relevance', 0.0):.2f}")

        print("\n✅ 处理完成！")
        return 0

    except Exception as e:
        print(f"❌ 处理失败: {e}")
        import traceback

        traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())
