#!/usr/bin/env python3
"""
SeeSea 数据清洗示例
演示如何使用 CleanerClient 清洗和处理文本数据
"""

from seesea import CleanerClient


def main():
    """数据清洗示例主函数"""

    print("🧹 SeeSea 数据清洗示例\n")

    # 创建清洗客户端
    print("1. 创建清洗客户端...")
    client = CleanerClient()
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
        print(f"   已连接: {info.get('connected', 'N/A')}\n")

    # 清洗文本
    print("4. 清洗文本...")
    dirty_text = "  这是一段   包含多余  空格  的  文本  \n\n  还有换行符  "
    clean_result = client.clean_text(text=dirty_text)
    if clean_result.success:
        cleaned_text = clean_result.data
        print(f"   原文本: '{dirty_text}'")
        print(f"   清洗后: '{cleaned_text}'\n")

    # 移除 HTML 标签
    print("5. 移除 HTML 标签...")
    html_content = """
    <div class="content">
        <h1>标题</h1>
        <p>这是一段<strong>重要</strong>的文本。</p>
        <a href="https://example.com">链接</a>
    </div>
    """
    remove_html_result = client.remove_html(html=html_content)
    if remove_html_result.success:
        plain_text = remove_html_result.data
        print(f"   HTML: {html_content[:50]}...")
        print(f"   纯文本: {plain_text[:50]}...\n")

    # 标准化文本
    print("6. 标准化文本...")
    messy_text = "Ｈｅｌｌｏ　Ｗｏｒｌｄ！你好，世界！"
    normalize_result = client.normalize_text(text=messy_text)
    if normalize_result.success:
        normalized_text = normalize_result.data
        print(f"   原文本: {messy_text}")
        print(f"   标准化后: {normalized_text}\n")

    # 提取 URL
    print("7. 提取 URL...")
    text_with_urls = """
    访问我们的网站 https://www.example.com
    或者查看文档 https://docs.example.com/guide
    还有测试页面 http://test.example.com
    """
    extract_urls_result = client.extract_urls(text=text_with_urls)
    if extract_urls_result.success:
        urls = extract_urls_result.data
        print(f"   找到 {len(urls)} 个 URL:\n")
        for url in urls:
            print(f"   - {url}\n")

    # 批量清洗文本
    print("8. 批量清洗文本...")
    texts = ["  文本1  ", "  文本2  \n", "  文本3  "]
    clean_batch_result = client.clean_batch(texts=texts)
    if clean_batch_result.success:
        cleaned_texts = clean_batch_result.data
        print(f"   ✓ 批量清洗 {len(texts)} 段文本:\n")
        for original, cleaned in zip(texts, cleaned_texts):
            print(f"   原文: '{original}'")
            print(f"   清洗: '{cleaned}'\n")

    # 处理文本（使用 Rust 清洗器）
    print("9. 处理文本（使用 Rust 清洗器）...")
    sample_text = """
    第一段内容。
    
    第二段内容，这是重要信息。
    
    第三段内容。
    """
    process_result = client.process_text(text=sample_text)
    if process_result.success:
        blocks = process_result.data
        print(f"   ✓ 处理得到 {len(blocks)} 个数据块:\n")
        for i, block in enumerate(blocks[:3], 1):
            print(f"   {i}. 内容: {block.get('content', 'N/A')[:50]}...")
            print(f"      评分: {block.get('score', 'N/A'):.2f}")
            print(f"      有效: {block.get('is_valid', 'N/A')}\n")

    # 处理文本并获取上下文
    print("10. 处理文本并获取上下文...")
    context_result = client.process_text_with_context(text=sample_text)
    if context_result.success:
        context = context_result.data
        print("   ✓ 上下文:\n")
        print(f"   {context[:100]}...\n")

    # 健康检查
    print("11. 健康检查...")
    health_result = client.health_check()
    if health_result.success:
        health = health_result.data
        print(f"   状态: {health.get('status', 'N/A')}\n")

    # 断开连接
    print("12. 断开连接...")
    client.disconnect()
    print("   ✓ 已断开连接\n")


if __name__ == "__main__":
    main()
