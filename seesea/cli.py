#!/usr/bin/env python3
"""
SeeSea 命令行接口

提供简单的命令行工具来使用 SeeSea 搜索引擎
"""

import argparse
import sys
import json
from typing import Optional
from .search import SearchClient
from .api import ApiServer
from .utils import format_results


def cmd_search(args):
    """执行搜索命令"""
    client = SearchClient()
    
    print(f"🔍 搜索: {args.query}")
    if args.verbose:
        print(f"   页码: {args.page}, 每页: {args.page_size}")
    
    try:
        results = client.search(
            query=args.query,
            page=args.page,
            page_size=args.page_size
        )
        
        if args.json:
            # JSON 输出
            print(json.dumps(results, ensure_ascii=False, indent=2))
        else:
            # 人类可读输出
            print(f"\n📊 搜索结果:")
            print(f"   总结果: {results['total_count']}")
            print(f"   耗时: {results['query_time_ms']}ms")
            print(f"   引擎: {', '.join(results['engines_used'])}")
            print(f"   缓存: {'✅ 命中' if results['cached'] else '❌ 新查询'}")
            
            formatted = format_results(results['results'], max_description_length=args.desc_length)
            
            print(f"\n📝 结果列表 (显示前{min(args.limit, len(formatted))}个):\n")
            for i, item in enumerate(formatted[:args.limit], 1):
                print(f"{i}. {item['title']}")
                print(f"   🔗 {item['url']}")
                if item['description']:
                    print(f"   📄 {item['description']}")
                if args.verbose:
                    print(f"   ⭐ 评分: {item['score']:.3f}")
                print()
                
    except Exception as e:
        print(f"❌ 搜索失败: {e}", file=sys.stderr)
        sys.exit(1)


def cmd_server(args):
    """启动 API 服务器命令"""
    print(f"🌊 SeeSea API 服务器")
    print(f"   地址: {args.host}:{args.port}\n")
    
    try:
        server = ApiServer(host=args.host, port=args.port)
        
        print(f"📍 可用端点:")
        print(f"   GET/POST http://{args.host}:{args.port}/api/search")
        print(f"   GET      http://{args.host}:{args.port}/api/health")
        print(f"   GET      http://{args.host}:{args.port}/api/stats")
        print(f"\n🚀 服务器启动中...")
        print(f"   按 Ctrl+C 停止\n")
        
        server.start()
    except KeyboardInterrupt:
        print("\n\n👋 服务器已停止")
    except Exception as e:
        print(f"❌ 服务器错误: {e}", file=sys.stderr)
        sys.exit(1)


def cmd_stats(args):
    """显示统计信息命令"""
    client = SearchClient()
    stats = client.get_stats()
    
    if args.json:
        print(json.dumps(stats, ensure_ascii=False, indent=2))
    else:
        print("📈 SeeSea 统计信息\n")
        print(f"   总搜索次数: {stats['total_searches']}")
        print(f"   缓存命中: {stats['cache_hits']}")
        print(f"   缓存未命中: {stats['cache_misses']}")
        
        if stats['total_searches'] > 0:
            total_cache = stats['cache_hits'] + stats['cache_misses']
            if total_cache > 0:
                hit_rate = stats['cache_hits'] / total_cache * 100
                print(f"   缓存命中率: {hit_rate:.1f}%")
        
        print(f"   引擎失败: {stats['engine_failures']}")
        print(f"   超时次数: {stats['timeouts']}")


def main():
    """主命令行入口"""
    parser = argparse.ArgumentParser(
        description='SeeSea - 隐私保护型元搜索引擎',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  # 搜索
  seesea search "python programming"
  seesea search "rust" -n 20
  
  # 启动服务器
  seesea server --port 8080
  
  # 查看统计
  seesea stats
        """
    )
    
    parser.add_argument('--version', action='version', version='SeeSea 0.1.0')
    
    subparsers = parser.add_subparsers(dest='command', help='可用命令')
    
    # search 命令
    search_parser = subparsers.add_parser('search', help='执行搜索')
    search_parser.add_argument('query', help='搜索关键词')
    search_parser.add_argument('-p', '--page', type=int, default=1, help='页码 (默认: 1)')
    search_parser.add_argument('-n', '--page-size', type=int, default=10, help='每页结果数 (默认: 10)')
    search_parser.add_argument('-l', '--limit', type=int, default=10, help='显示结果数 (默认: 10)')
    search_parser.add_argument('-d', '--desc-length', type=int, default=150, help='描述长度 (默认: 150)')
    search_parser.add_argument('-j', '--json', action='store_true', help='JSON 格式输出')
    search_parser.add_argument('-v', '--verbose', action='store_true', help='详细输出')
    search_parser.set_defaults(func=cmd_search)
    
    # server 命令
    server_parser = subparsers.add_parser('server', help='启动 API 服务器')
    server_parser.add_argument('--host', default='127.0.0.1', help='监听地址 (默认: 127.0.0.1)')
    server_parser.add_argument('--port', type=int, default=8080, help='监听端口 (默认: 8080)')
    server_parser.set_defaults(func=cmd_server)
    
    # stats 命令
    stats_parser = subparsers.add_parser('stats', help='显示统计信息')
    stats_parser.add_argument('-j', '--json', action='store_true', help='JSON 格式输出')
    stats_parser.set_defaults(func=cmd_stats)
    
    args = parser.parse_args()
    
    if not args.command:
        parser.print_help()
        sys.exit(0)
    
    args.func(args)


if __name__ == '__main__':
    main()
