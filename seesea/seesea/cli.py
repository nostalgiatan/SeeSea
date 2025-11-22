#!/usr/bin/env python3
# Copyright 2025 nostalgiatan
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""
SeeSea 命令行接口

提供现代化的命令行工具来使用 SeeSea 搜索引擎和RSS功能
"""

import click
import json
import sys
from typing import Optional, List
from rich.console import Console
from rich.table import Table
from rich.panel import Panel
from rich.progress import Progress, SpinnerColumn, TextColumn
from rich.text import Text
from rich import box

from .search import SearchClient
from .rss import RssClient
from .api import ApiServer
from .utils import format_results

# 初始化 Rich Console
console = Console()


@click.command()
@click.argument('query')
@click.option('-p', '--page', default=1, help='页码 (默认: 1)')
@click.option('-n', '--page-size', default=10, help='每页结果数 (默认: 10)')
@click.option('-l', '--limit', default=10, help='显示结果数 (默认: 10)')
@click.option('-j', '--json', is_flag=True, help='JSON 格式输出')
@click.option('-v', '--verbose', is_flag=True, help='详细输出')
@click.option('-c', '--china', is_flag=True, help='使用中国模式')
def search(query, page, page_size, limit, json, verbose, china):
    """执行搜索"""
    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        console=console,
        transient=True,
    ) as progress:
        task = progress.add_task(f"搜索: {query}", total=None)

        try:
            client = SearchClient()
            results = client.search(
                query=query,
                page=page,
                page_size=page_size,
                language='zh' if china else None
            )
            progress.update(task, description="搜索完成")

        except Exception as e:
            progress.stop()
            console.print(f"[red]搜索失败: {e}[/red]")
            sys.exit(1)

    if json:
        console.print(json.dumps(results, ensure_ascii=False, indent=2))
    else:
        # 显示搜索概要
        summary_table = Table(show_header=False, box=box.ROUNDED)
        summary_table.add_column("属性", style="bold blue")
        summary_table.add_column("值")
        summary_table.add_row("总结果", str(results.total_count))
        summary_table.add_row("耗时", f"{results.query_time_ms}ms")
        summary_table.add_row("引擎", ", ".join(results.engines_used))
        summary_table.add_row("缓存", "命中" if results.cached else "新查询")

        console.print(Panel(summary_table, title="搜索概要", border_style="blue"))

        # 显示结果列表
        formatted = format_results(results.results, max_description_length=150)
        console.print(f"\n结果列表 (显示前{min(limit, len(formatted))}个):\n")

        for i, item in enumerate(formatted[:limit], 1):
            content = Text()
            content.append(f"{i}. ", style="cyan")
            content.append(item['title'], style="bold")

            if item['description']:
                content.append(f"\n   {item['description']}", style="dim")

            if verbose:
                content.append(f"\n   🔗 {item['url']}", style="blue")
                content.append(f"\n   ⭐ 评分: {item['score']:.3f}", style="yellow")

            console.print(Panel(content, box=box.SIMPLE, border_style="green"))
            console.print()




@click.group()
def rss():
    """RSS 订阅功能"""
    pass


@rss.command('list')
def rss_list():
    """列出可用RSS模板"""
    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        console=console,
        transient=True,
    ) as progress:
        task = progress.add_task("获取RSS模板列表...", total=None)

        try:
            client = RssClient()
            templates = client.list_templates()
            progress.update(task, description="获取完成")

        except Exception as e:
            progress.stop()
            console.print(f"[red]获取模板失败: {e}[/red]")
            sys.exit(1)

    if templates:
        table = Table(title="可用RSS模板", box=box.ROUNDED)
        table.add_column("序号", style="cyan", width=6)
        table.add_column("模板名称", style="bold")
        table.add_column("描述", style="dim")

        for i, template in enumerate(templates, 1):
            descriptions = {
                'xinhua': '新华网官方RSS订阅源',
                'people': '人民网官方RSS订阅源',
            }
            desc = descriptions.get(template, 'RSS订阅源')
            table.add_row(str(i), template, desc)

        console.print(table)
    else:
        console.print("[yellow]没有找到可用模板[/yellow]")


@rss.command('add')
@click.argument('template')
@click.option('-c', '--categories', help='分类列表，用逗号分隔')
def rss_add(template, categories):
    """从模板添加RSS"""
    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        console=console,
        transient=True,
    ) as progress:
        task = progress.add_task(f"添加RSS模板: {template}", total=None)

        try:
            client = RssClient()
            categories_list = categories.split(',') if categories else None
            count = client.add_from_template(template, categories_list)
            progress.update(task, description="添加完成")

        except Exception as e:
            progress.stop()
            console.print(f"[red]添加RSS失败: {e}[/red]")
            sys.exit(1)

    # 显示成功信息
    success_panel = Panel(
        f"[green]✅ 成功添加 {count} 个RSS feeds[/green]\n"
        f"模板: {template}\n"
        f"分类: {categories or '全部'}",
        title="添加成功",
        border_style="green"
    )
    console.print(success_panel)


@rss.command('fetch')
@click.argument('url')
@click.option('-l', '--limit', default=10, help='显示项目数 (默认: 10)')
@click.option('-v', '--verbose', is_flag=True, help='详细输出')
def rss_fetch(url, limit, verbose):
    """获取RSS feed"""
    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        console=console,
        transient=True,
    ) as progress:
        task = progress.add_task("获取RSS内容...", total=None)

        try:
            client = RssClient()
            feed = client.fetch_feed(url, max_items=limit)
            progress.update(task, description="获取完成")

        except Exception as e:
            progress.stop()
            console.print(f"[red]获取RSS失败: {e}[/red]")
            sys.exit(1)

    # 显示Feed信息
    feed_info = Table(show_header=False, box=box.ROUNDED)
    feed_info.add_column("属性", style="bold blue")
    feed_info.add_column("值")
    feed_info.add_row("标题", feed['meta']['title'])
    feed_info.add_row("链接", feed['meta']['link'])
    if feed['meta'].get('description'):
        desc = feed['meta']['description'][:80] + "..." if len(feed['meta']['description']) > 80 else feed['meta']['description']
        feed_info.add_row("描述", desc)
    feed_info.add_row("项目数", str(len(feed['items'])))

    console.print(Panel(feed_info, title="RSS Feed 信息", border_style="blue"))

    # 显示项目列表
    console.print(f"\nRSS 项目 (显示前{min(limit, len(feed['items']))}个):\n")

    for i, item in enumerate(feed['items'][:limit], 1):
        content = Text()
        content.append(f"{i}. ", style="cyan")
        content.append(item['title'], style="bold")
        content.append(f"\n   🔗 {item['link']}", style="blue")

        if verbose and item.get('description'):
            desc = item['description'][:100] + "..." if len(item['description']) > 100 else item['description']
            content.append(f"\n   📄 {desc}", style="dim")

        if verbose and item.get('pub_date'):
            content.append(f"\n   📅 {item['pub_date']}", style="yellow")

        console.print(Panel(content, box=box.SIMPLE, border_style="green"))
        console.print()


@rss.command('ranking')
@click.argument('keywords')
@click.option('-u', '--urls', help='RSS URL列表，用逗号分隔')
@click.option('-l', '--limit', default=20, help='显示项目数 (默认: 20)')
@click.option('-s', '--min-score', default=3.0, help='最小评分 (默认: 3.0)')
@click.option('-v', '--verbose', is_flag=True, help='详细输出')
def rss_ranking(keywords, urls, limit, min_score, verbose):
    """创建RSS榜单"""
    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        console=console,
        transient=True,
    ) as progress:
        task = progress.add_task("创建RSS榜单...", total=None)

        try:
            client = RssClient()

            # 解析关键词和权重
            keyword_list = []
            for kw_pair in keywords.split(','):
                if ':' in kw_pair:
                    keyword, weight = kw_pair.split(':', 1)
                    try:
                        weight = float(weight.strip())
                    except:
                        weight = 5.0
                    keyword_list.append((keyword.strip(), weight))
                else:
                    keyword_list.append((kw_pair.strip(), 5.0))

            # 解析RSS URLs
            feed_urls = urls.split(',') if urls else []

            ranking = client.create_ranking(
                feed_urls=feed_urls,
                keywords=keyword_list,
                min_score=min_score,
                max_results=limit
            )

            progress.update(task, description="榜单创建完成")

        except Exception as e:
            progress.stop()
            console.print(f"[red]创建榜单失败: {e}[/red]")
            sys.exit(1)

    # 显示榜单概要
    ranking_info = Table(show_header=False, box=box.ROUNDED)
    ranking_info.add_column("属性", style="bold yellow")
    ranking_info.add_column("值")
    ranking_info.add_row("总项目数", str(ranking.get('total_items', 0)))
    ranking_info.add_row("评分阈值", str(min_score))
    ranking_info.add_row("关键词", ", ".join([kw for kw, w in keyword_list]))

    console.print(Panel(ranking_info, title="RSS 榜单概要", border_style="yellow"))

    # 显示榜单项目
    items = ranking.get('items', [])
    if items:
        console.print(f"\n热门文章榜单 (显示前{min(limit, len(items))}个):\n")

        ranking_table = Table(box=box.ROUNDED)
        ranking_table.add_column("排名", style="bold cyan", width=6)
        ranking_table.add_column("评分", style="bold yellow", width=8)
        ranking_table.add_column("标题", style="bold")
        if verbose:
            ranking_table.add_column("链接", style="blue")
            ranking_table.add_column("匹配关键词", style="green")

        for i, item in enumerate(items[:limit], 1):
            score = item.get('score', 0)
            title = item.get('title', 'N/A')[:50] + "..." if len(item.get('title', '')) > 50 else item.get('title', 'N/A')

            row = [str(i), f"{score:.1f}", title]
            if verbose:
                row.extend([
                    item.get('link', 'N/A')[:40] + "...",
                    ", ".join(item.get('matched_keywords', []))
                ])

            ranking_table.add_row(*row)

        console.print(ranking_table)
    else:
        console.print("[yellow]没有找到匹配的项目[/yellow]")


@click.group(invoke_without_command=True)
@click.pass_context
def cli(ctx):
    """SeeSea - 隐私保护型元搜索引擎"""
    if ctx.invoked_subcommand is None:
        # 默认启动交互式模式
        interactive()


@cli.command()
@click.argument('query')
@click.option('-p', '--page', default=1, help='页码 (默认: 1)')
@click.option('-n', '--page-size', default=10, help='每页结果数 (默认: 10)')
@click.option('-l', '--limit', default=10, help='显示结果数 (默认: 10)')
@click.option('-j', '--json', is_flag=True, help='JSON 格式输出')
@click.option('-v', '--verbose', is_flag=True, help='详细输出')
@click.option('-c', '--china', is_flag=True, help='使用中国模式')
def search(query, page, page_size, limit, json, verbose, china):
    """执行搜索"""
    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        console=console,
        transient=True,
    ) as progress:
        task = progress.add_task(f"搜索: {query}", total=None)

        try:
            client = SearchClient()
            results = client.search(
                query=query,
                page=page,
                page_size=page_size,
                language='zh' if china else None
            )
            progress.update(task, description="搜索完成")

        except Exception as e:
            progress.stop()
            console.print(f"[red]搜索失败: {e}[/red]")
            sys.exit(1)

    if json:
        console.print(json.dumps(results, ensure_ascii=False, indent=2))
    else:
        # 显示搜索概要
        summary_table = Table(show_header=False, box=box.ROUNDED)
        summary_table.add_column("属性", style="bold blue")
        summary_table.add_column("值")
        summary_table.add_row("总结果", str(results.total_count))
        summary_table.add_row("耗时", f"{results.query_time_ms}ms")
        summary_table.add_row("引擎", ", ".join(results.engines_used))
        summary_table.add_row("缓存", "命中" if results.cached else "新查询")

        console.print(Panel(summary_table, title="搜索概要", border_style="blue"))

        # 显示结果列表
        formatted = format_results(results.results, max_description_length=150)
        console.print(f"\n结果列表 (显示前{min(limit, len(formatted))}个):\n")

        for i, item in enumerate(formatted[:limit], 1):
            content = Text()
            content.append(f"{i}. ", style="cyan")
            content.append(item['title'], style="bold")

            if item['description']:
                content.append(f"\n   {item['description']}", style="dim")

            if verbose:
                content.append(f"\n   🔗 {item['url']}", style="blue")
                content.append(f"\n   ⭐ 评分: {item['score']:.3f}", style="yellow")

            console.print(Panel(content, box=box.SIMPLE, border_style="green"))
            console.print()


@cli.command()
@click.option('--host', default='127.0.0.1', help='监听地址 (默认: 127.0.0.1)')
@click.option('--port', default=8080, help='监听端口 (默认: 8080)')
def server(host, port):
    """启动 API 服务器"""
    server_info = Table(box=box.ROUNDED)
    server_info.add_column("属性", style="bold green")
    server_info.add_column("值")
    server_info.add_row("服务", "SeeSea API 服务器")
    server_info.add_row("地址", f"{host}:{port}")
    server_info.add_row("搜索端点", f"GET/POST http://{host}:{port}/api/search")
    server_info.add_row("健康检查", f"GET http://{host}:{port}/api/health")
    server_info.add_row("统计信息", f"GET http://{host}:{port}/api/stats")

    console.print(Panel(server_info, title="API服务器信息", border_style="green"))
    console.print(f"\n服务器启动中... 按Ctrl+C停止\n")

    try:
        api_server = ApiServer(host=host, port=port)
        api_server.start()
    except KeyboardInterrupt:
        console.print("\n[green]服务器已停止[/green]")
    except Exception as e:
        console.print(f"[red]服务器错误: {e}[/red]")
        sys.exit(1)


@cli.command()
@click.option('-j', '--json', is_flag=True, help='JSON 格式输出')
def stats(json):
    """显示统计信息"""
    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        console=console,
        transient=True,
    ) as progress:
        task = progress.add_task("获取统计信息...", total=None)

        try:
            client = SearchClient()
            stats_data = client.get_stats()
            progress.update(task, description="获取完成")

        except Exception as e:
            progress.stop()
            console.print(f"[red]获取统计信息失败: {e}[/red]")
            sys.exit(1)

    if json:
        console.print(json.dumps(stats_data, ensure_ascii=False, indent=2))
    else:
        stats_table = Table(title="SeeSea 统计信息", box=box.ROUNDED)
        stats_table.add_column("统计项", style="bold blue")
        stats_table.add_column("数值", style="bold green")

        stats_table.add_row("总搜索次数", str(stats_data['total_searches']))
        stats_table.add_row("缓存命中", str(stats_data['cache_hits']))
        stats_table.add_row("缓存未命中", str(stats_data['cache_misses']))

        if stats_data['total_searches'] > 0:
            total_cache = stats_data['cache_hits'] + stats_data['cache_misses']
            if total_cache > 0:
                hit_rate = stats_data['cache_hits'] / total_cache * 100
                stats_table.add_row("缓存命中率", f"{hit_rate:.1f}%")

        stats_table.add_row("引擎失败", str(stats_data['engine_failures']))
        stats_table.add_row("超时次数", str(stats_data['timeouts']))

        console.print(stats_table)


@cli.command()
@click.option('-c', '--china', is_flag=True, help='启动时使用中国模式')
def interactive(china):
    """交互式搜索模式"""
    console.print("SeeSea 交互式搜索")
    console.print("━" * 50)
    console.print("输入查询来搜索，输入 'quit' 或 'exit' 退出")
    console.print("输入 'engines' 列出所有引擎")
    console.print("输入 'stats' 查看统计信息")
    console.print("输入 'mode' 切换运行模式")
    console.print("━" * 50)

    if china:
        console.print("[green]当前模式: 中国模式[/green]")

    client = SearchClient()

    while True:
        try:
            from rich.prompt import Prompt
            prompt = "🔍 > "
            if china:
                prompt = "🔍 [green]中国模式[/green] > "

            query = Prompt.ask(prompt, console=console).strip()

            if not query:
                continue

            if query.lower() in ['quit', 'exit']:
                console.print("[green]再见！[/green]")
                break

            if query.lower() == 'engines':
                with console.status("获取引擎列表..."):
                    try:
                        engines = client.list_engines()
                        if engines:
                            for engine in sorted(engines):
                                console.print(f"• {engine}")
                        else:
                            console.print("未找到可用引擎")
                    except Exception as e:
                        console.print(f"获取引擎列表失败: {e}")
                continue

            if query.lower() == 'stats':
                stats({})
                continue

            if query.lower() == 'mode':
                choice = Prompt.ask("选择运行模式", choices=["1", "2"], default="1", console=console)
                china = choice == '2'
                mode_name = "中国模式" if china else "默认模式"
                console.print(f"[green]切换到{mode_name}[/green]")
                continue

            # 执行搜索
            with Progress(
                SpinnerColumn(),
                TextColumn("[progress.description]{task.description}"),
                console=console,
                transient=True,
            ) as progress:
                task = progress.add_task(f"搜索: {query}", total=None)

                try:
                    results = client.search(
                        query=query,
                        page=1,
                        page_size=10,
                        language='zh' if china else None
                    )
                    progress.update(task, description="搜索完成")

                except Exception as e:
                    progress.stop()
                    console.print(f"[red]搜索失败: {e}[/red]")
                    continue

            # 显示结果
            console.print(f"\n搜索结果:")
            console.print(f"总结果: {results.total_count}, 耗时: {results.query_time_ms}ms")
            console.print(f"引擎: {', '.join(results.engines_used)}")

            formatted = format_results(results.results, max_description_length=120)
            console.print(f"\n结果列表:\n")

            for i, item in enumerate(formatted[:10], 1):
                content = Text()
                content.append(f"{i}. ", style="cyan")
                content.append(item['title'], style="bold")

                if item['description']:
                    desc = item['description'][:120] + "..." if len(item['description']) > 120 else item['description']
                    content.append(f"\n   {desc}", style="dim")

                console.print(Panel(content, box=box.SIMPLE, border_style="green"))
                console.print()

        except KeyboardInterrupt:
            console.print("\n[green]再见！[/green]")
            break
        except EOFError:
            console.print("\n[green]再见！[/green]")
            break
        except Exception as e:
            console.print(f"[red]错误: {e}[/red]")


# 添加RSS命令组
cli.add_command(rss)


if __name__ == '__main__':
    cli()
