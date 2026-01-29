#!/usr/bin/env python3
"""
SeeSea 缓存示例
演示如何使用 CacheClient 管理缓存
"""

from seesea import CacheClient


def main():
    """缓存示例主函数"""

    print("💾 SeeSea 缓存示例\n")

    # 创建缓存客户端
    print("1. 创建缓存客户端...")
    client = CacheClient()
    print("   ✓ 客户端创建成功\n")

    # 连接到服务器
    print("2. 连接到服务器...")
    result = client.connect()
    if not result.success:
        print(f"   ✗ 连接失败: {result.error}\n")
        return
    print("   ✓ 连接成功\n")

    # 获取客户端信息
    print("3. 获取缓存信息...")
    info_result = client.get_info()
    if info_result.success:
        info = info_result.data
        print(f"   版本: {info.get('version', 'N/A')}")
        print(f"   作用域: {info.get('scope', 'N/A')}")
        print(f"   已连接: {info.get('connected', 'N/A')}\n")

    # 设置缓存
    print("4. 设置缓存...")
    set_result = client.set(key="demo_key", value="Hello, SeeSea!", ttl=3600)
    if set_result.success:
        print("   ✓ 缓存已设置\n")

    # 获取缓存
    print("5. 获取缓存...")
    get_result = client.get(key="demo_key")
    if get_result.success:
        print(f"   ✓ 缓存值: {get_result.data}\n")

    # 检查缓存是否存在
    print("6. 检查缓存是否存在...")
    exists_result = client.exists(key="demo_key")
    if exists_result.success:
        print(f"   ✓ demo_key 存在: {exists_result.data}\n")

    # 获取缓存TTL
    print("7. 获取缓存 TTL...")
    ttl_result = client.ttl(key="demo_key")
    if ttl_result.success:
        ttl = ttl_result.data
        if ttl == -1:
            print("   ✓ 缓存永不过期\n")
        elif ttl == -2:
            print("   ✓ 缓存不存在\n")
        else:
            print(f"   ✓ 剩余 TTL: {ttl} 秒\n")

    # 设置多个缓存
    print("8. 批量设置缓存...")
    mset_result = client.mset(
        mapping={"key1": "value1", "key2": "value2", "key3": "value3"}
    )
    if mset_result.success:
        print("   ✓ 批量设置完成\n")

    # 批量获取缓存
    print("9. 批量获取缓存...")
    mget_result = client.mget(keys=["key1", "key2", "key3"])
    if mget_result.success:
        values = mget_result.data
        for key, value in zip(["key1", "key2", "key3"], values):
            print(f"   {key}: {value}\n")

    # 列出所有缓存键
    print("10. 列出所有缓存键...")
    keys_result = client.keys(pattern="*")
    if keys_result.success:
        keys = keys_result.data
        print(f"   ✓ 找到 {len(keys)} 个缓存键\n")

    # 获取缓存统计
    print("11. 获取缓存统计...")
    stats_result = client.get_stats()
    if stats_result.success:
        stats = stats_result.data
        print(f"   命中次数: {stats.get('hits', 'N/A')}")
        print(f"   未命中次数: {stats.get('misses', 'N/A')}")
        print(f"   命中率: {stats.get('hit_rate', 'N/A'):.2%}\n")

    # 清除特定缓存
    print("12. 清除特定缓存...")
    delete_result = client.delete(key="demo_key")
    if delete_result.success:
        print("   ✓ demo_key 已删除\n")

    # 再次检查
    print("13. 再次检查缓存...")
    exists_result = client.exists(key="demo_key")
    if exists_result.success:
        print(f"   ✓ demo_key 存在: {exists_result.data}\n")

    # 清空所有缓存
    print("14. 清空所有缓存...")
    clear_result = client.clear()
    if clear_result.success:
        print("   ✓ 所有缓存已清空\n")

    # 健康检查
    print("15. 健康检查...")
    health_result = client.health_check()
    if health_result.success:
        health = health_result.data
        print(f"   状态: {health.get('status', 'N/A')}\n")

    # 断开连接
    print("16. 断开连接...")
    client.disconnect()
    print("   ✓ 已断开连接\n")


if __name__ == "__main__":
    main()
