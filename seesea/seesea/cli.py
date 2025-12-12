#!/usr/bin/env python3
# Copyright (C) 2025 nostalgiatan
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

"""
SeeSea 命令行接口

提供现代化的命令行工具来使用 SeeSea 搜索引擎和RSS功能
"""

import click
import json as json_module
import sys
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


@click.group(invoke_without_command=True, help="SeeSea - 隐私保护型元搜索引擎")
@click.pass_context
def cli(ctx):
    """SeeSea - 隐私保护型元搜索引擎"""
    if ctx.invoked_subcommand is None:
        # 默认启动交互式模式
        interactive()


@cli.command()
@click.argument("query")
@click.option("-p", "--pro", is_flag=True, help="使用 Pro API 进行搜索")
@click.option("--page", default=1, help="页码 (默认: 1)")
@click.option("-n", "--page-size", default=10, help="每页结果数 (默认: 10)")
@click.option("-l", "--limit", default=10, help="显示结果数 (默认: 10)")
@click.option("-j", "--json", is_flag=True, help="JSON 格式输出")
@click.option("-v", "--verbose", is_flag=True, help="详细输出")
@click.option("-e", "--engines", help="指定搜索引擎列表，用逗号分隔")
@click.option(
    "-c", "--count", type=int, help="使用的引擎数量（按延迟排序，选择低延迟引擎）"
)
def search(query, pro, page, page_size, limit, json, verbose, engines, count):
    """执行搜索"""
    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        console=console,
        transient=True,
    ) as progress:
        task = progress.add_task(f"搜索: {query}", total=None)

        try:
            if pro:
                # Pro 功能需要显式启用才会加载
                try:
                    # 直接调用本地的 Pro handlers 函数
                    from seesea.handlers.pro import (
                        handle_pro_search,
                        initialize_pro_handlers,
                    )
                    import asyncio

                    from seesea_core import init_config

                    init_config("development")

                    asyncio.run(initialize_pro_handlers())

                    req = {
                        "path": "/search",
                        "method": "GET",
                        "query_params": {
                            "q": query,
                            "page": page,
                            "page_size": page_size,
                        },
                        "body": {},
                    }

                    response = asyncio.run(handle_pro_search(req))

                    import json

                    results_dict = json.loads(response.get("body", "{}"))
                except ImportError as e:
                    console.print(f"[red]❌ Pro features not available: {e}[/red]")
                    console.print("[yellow]Install Pro dependencies:[/yellow]")
                    console.print("  pip install llama-cpp-python")
                    console.print("[yellow]Or use pre-built package:[/yellow]")
                    console.print(
                        "  pip install llama-cpp-python --index-url https://abetlen.github.io/llama-cpp-python/whl/cpu"
                    )
                    return

                # 模拟 SearchResponse 对象结构
                class ProSearchResponse:
                    def __init__(self, data):
                        self.query = data.get("query")
                        self.results = [
                            type(
                                "obj",
                                (object,),
                                {
                                    "title": item.get("title"),
                                    "url": item.get("url"),
                                    "content": item.get("description"),
                                    "score": item.get("score", 0),
                                },
                            )
                            for item in data.get("results", [])
                        ]
                        self.total_count = data.get("total_count", 0)
                        self.cached = data.get("cached", False)
                        self.query_time_ms = data.get("query_time_ms", 0)
                        self.engines_used = data.get("engines_used", [])

                results = ProSearchResponse(results_dict)
                progress.update(task, description="Pro 搜索完成")
            else:
                # 使用普通搜索
                client = SearchClient()
                # Parse engines parameter
                engines_list = None
                if engines:
                    engines_list = [e.strip() for e in engines.split(",") if e.strip()]
                elif count:
                    # If count is specified but no engines, get all engines and take first N
                    all_engines = client.list_engines()
                    engines_list = (
                        all_engines[:count] if count < len(all_engines) else None
                    )

                results = client.search(
                    query=query, page=page, page_size=page_size, engines=engines_list
                )
                progress.update(task, description="搜索完成")

        except Exception as e:
            progress.stop()
            console.print(f"[red]搜索失败: {e}[/red]")
            sys.exit(1)

    if json:
        # Convert SearchResponse to dict for JSON serialization
        results_dict = {
            "query": results.query,
            "results": [
                {
                    "title": item.title,
                    "url": item.url,
                    "snippet": item.content,
                    "score": getattr(item, "score", 0),
                }
                for item in results.results
            ],
            "total_count": results.total_count,
            "cached": results.cached,
            "query_time_ms": results.query_time_ms,
            "engines_used": results.engines_used,
        }
        console.print(json_module.dumps(results_dict, ensure_ascii=False, indent=2))
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
            content.append(item["title"], style="bold")

            if item["description"]:
                content.append(f"\n   {item['description']}", style="dim")

            if verbose:
                content.append(f"\n   🔗 {item['url']}", style="blue")
                content.append(f"\n   ⭐ 评分: {item['score']:.3f}", style="yellow")

            console.print(Panel(content, box=box.SIMPLE, border_style="green"))
            console.print()


@cli.command()
@click.option("-j", "--json", is_flag=True, help="JSON 格式输出")
def engines(json):
    """列出所有可用的搜索引擎"""
    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        console=console,
        transient=True,
    ) as progress:
        task = progress.add_task("获取引擎列表...", total=None)

        try:
            client = SearchClient()
            engine_list = client.list_engines()
            progress.update(task, description="获取完成")

        except Exception as e:
            progress.stop()
            console.print(f"[red]获取引擎列表失败: {e}[/red]")
            sys.exit(1)

    if json:
        console.print(
            json_module.dumps({"engines": engine_list}, ensure_ascii=False, indent=2)
        )
    else:
        if engine_list:
            table = Table(title="可用搜索引擎", box=box.ROUNDED)
            table.add_column("引擎名称", style="cyan")
            table.add_column("类型", style="green")
            table.add_column("描述", style="yellow")

            # 添加引擎信息
            engine_info = {
                "google": ["Google", "Web", "全球最大的搜索引擎"],
                "bing": ["Bing", "Web", "微软搜索引擎"],
                "duckduckgo": ["DuckDuckGo", "Web", "隐私保护搜索引擎"],
                "xinhua": ["新华网", "News", "中国官方新闻媒体"],
                "baidu": ["百度", "Web", "中文搜索引擎"],
            }

            for engine in sorted(engine_list):
                info = engine_info.get(engine, [engine.title(), "Unknown", "搜索引擎"])
                table.add_row(info[0], info[1], info[2])

            console.print(table)

            # 使用提示
            usage_panel = Panel(
                "[green]使用方法:[/green]\n"
                'seesea search "关键词" -e google,bing  # 指定多个引擎\n'
                'seesea search "关键词" -e xinhua         # 只用新华网搜索',
                title="引擎选择提示",
                border_style="blue",
            )
            console.print(usage_panel)
        else:
            console.print("[yellow]没有找到可用引擎[/yellow]")


@click.group()
def rss():
    """RSS 订阅功能"""
    pass


@rss.command("list")
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
                "xinhua": "新华网官方RSS订阅源",
                "people": "人民网官方RSS订阅源",
            }
            desc = descriptions.get(template, "RSS订阅源")
            table.add_row(str(i), template, desc)

        console.print(table)
    else:
        console.print("[yellow]没有找到可用模板[/yellow]")


@rss.command("add")
@click.argument("template")
@click.option("-c", "--categories", help="分类列表，用逗号分隔")
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
            categories_list = categories.split(",") if categories else None
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
        border_style="green",
    )
    console.print(success_panel)


@rss.command("fetch")
@click.argument("url")
@click.option("-l", "--limit", default=10, help="显示项目数 (默认: 10)")
@click.option("-v", "--verbose", is_flag=True, help="详细输出")
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
    feed_info.add_row("标题", feed["meta"]["title"])
    feed_info.add_row("链接", feed["meta"]["link"])
    if feed["meta"].get("description"):
        desc = (
            feed["meta"]["description"][:80] + "..."
            if len(feed["meta"]["description"]) > 80
            else feed["meta"]["description"]
        )
        feed_info.add_row("描述", desc)
    feed_info.add_row("项目数", str(len(feed["items"])))

    console.print(Panel(feed_info, title="RSS Feed 信息", border_style="blue"))

    # 显示项目列表
    console.print(f"\nRSS 项目 (显示前{min(limit, len(feed['items']))}个):\n")

    for i, item in enumerate(feed["items"][:limit], 1):
        content = Text()
        content.append(f"{i}. ", style="cyan")
        content.append(item["title"], style="bold")
        content.append(f"\n   🔗 {item['link']}", style="blue")

        if verbose and item.get("description"):
            desc = (
                item["description"][:100] + "..."
                if len(item["description"]) > 100
                else item["description"]
            )
            content.append(f"\n   📄 {desc}", style="dim")

        if verbose and item.get("pub_date"):
            content.append(f"\n   📅 {item['pub_date']}", style="yellow")

        console.print(Panel(content, box=box.SIMPLE, border_style="green"))
        console.print()


@rss.command("ranking")
@click.argument("keywords")
@click.option("-u", "--urls", help="RSS URL列表，用逗号分隔")
@click.option("-l", "--limit", default=20, help="显示项目数 (默认: 20)")
@click.option("-s", "--min-score", default=3.0, help="最小评分 (默认: 3.0)")
@click.option("-v", "--verbose", is_flag=True, help="详细输出")
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
            for kw_pair in keywords.split(","):
                if ":" in kw_pair:
                    keyword, weight = kw_pair.split(":", 1)
                    try:
                        weight = float(weight.strip())
                    except ValueError:
                        weight = 5.0
                    keyword_list.append((keyword.strip(), weight))
                else:
                    keyword_list.append((kw_pair.strip(), 5.0))

            # 解析RSS URLs
            feed_urls = urls.split(",") if urls else []

            ranking = client.create_ranking(
                feed_urls=feed_urls,
                keywords=keyword_list,
                min_score=min_score,
                max_results=limit,
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
    ranking_info.add_row("总项目数", str(ranking.get("total_items", 0)))
    ranking_info.add_row("评分阈值", str(min_score))
    ranking_info.add_row("关键词", ", ".join([kw for kw, w in keyword_list]))

    console.print(Panel(ranking_info, title="RSS 榜单概要", border_style="yellow"))

    # 显示榜单项目
    items = ranking.get("items", [])
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
            score = item.get("score", 0)
            title = (
                item.get("title", "N/A")[:50] + "..."
                if len(item.get("title", "")) > 50
                else item.get("title", "N/A")
            )

            row = [str(i), f"{score:.1f}", title]
            if verbose:
                row.extend(
                    [
                        item.get("link", "N/A")[:40] + "...",
                        ", ".join(item.get("matched_keywords", [])),
                    ]
                )

            ranking_table.add_row(*row)

        console.print(ranking_table)
    else:
        console.print("[yellow]没有找到匹配的项目[/yellow]")


@cli.command()
@click.option("--host", default=None, help="监听地址 (默认: 配置文件中的地址)")
@click.option(
    "--port", type=int, default=None, help="监听端口 (默认: 配置文件中的端口)"
)
@click.option("-c", "--config", default=None, help="配置文件路径")
@click.option("--pro", is_flag=True, help="启用 Pro 功能 (LLM、向量搜索等)")
def server(host, port, config, pro):
    """启动 API 服务器"""
    try:
        # 如果提供了配置文件，传递给ApiServer，不传递host和port
        if config:
            # 使用配置文件时，不传递host和port，让ApiServer自己从配置文件中获取
            api_server = ApiServer(config_file=config, enable_pro=pro)
        else:
            # 没有配置文件时，使用默认值
            default_host = host if host is not None else "127.0.0.1"
            default_port = port if port is not None else 8080
            api_server = ApiServer(host=default_host, port=default_port, enable_pro=pro)

        # 获取实际使用的地址
        actual_address = api_server.address
        actual_host, actual_port = actual_address.split(":")

        # 显示启动前的配置信息
        server_info = Table(box=box.ROUNDED, show_header=False)
        server_info.add_column("属性", style="cyan bold", width=20)
        server_info.add_column("值", style="white")

        server_info.add_row("📡 服务", "SeeSea API 服务器")
        server_info.add_row("🌐 监听地址", f"{actual_host}:{actual_port}")

        # Pro 功能状态显示
        if pro:
            pro_status = "[bold green]✅ 已启用[/bold green]"
            pro_features = "LLM 分析、向量搜索、语义缓存"
        else:
            pro_status = "[bold yellow]⚠️  未启用[/bold yellow]"
            pro_features = "使用 --pro 参数启用"

        server_info.add_row("⚡ Pro 功能", pro_status)
        server_info.add_row("   功能详情", f"[dim]{pro_features}[/dim]")

        if config:
            server_info.add_row("⚙️  配置文件", config)

        console.print(
            Panel(
                server_info,
                title="[bold white]🚀 API 服务器配置[/bold white]",
                border_style="cyan",
                padding=(1, 2),
            )
        )

        if pro:
            console.print(
                "  [dim]提示: Pro 功能需要额外的依赖和模型,首次使用时会自动下载[/dim]\n"
            )
        else:
            console.print(
                "  [dim]提示: 使用 'seesea server --pro' 启用 LLM 和向量搜索功能[/dim]\n"
            )

        console.print(
            "[bold green]⏳ 服务器启动中...[/bold green] [dim]按 Ctrl+C 停止[/dim]\n"
        )

        # 启动成功后显示详细信息
        endpoint_table = Table(
            box=box.SIMPLE, show_header=True, header_style="bold magenta"
        )
        endpoint_table.add_column("端点", style="cyan", width=30)
        endpoint_table.add_column("方法", style="yellow", width=10)
        endpoint_table.add_column("说明", style="white")

        endpoint_table.add_row(
            f"http://{actual_host}:{actual_port}/api/search", "GET/POST", "搜索接口"
        )
        endpoint_table.add_row(
            f"http://{actual_host}:{actual_port}/api/health", "GET", "健康检查"
        )
        endpoint_table.add_row(
            f"http://{actual_host}:{actual_port}/api/stats", "GET", "统计信息"
        )

        if pro:
            endpoint_table.add_row(
                f"http://{actual_host}:{actual_port}/api/semantic-search",
                "POST",
                "[green]语义搜索 (Pro)[/green]",
            )
            endpoint_table.add_row(
                f"http://{actual_host}:{actual_port}/api/analyze",
                "POST",
                "[green]LLM 分析 (Pro)[/green]",
            )

        success_info = Table(box=box.ROUNDED, show_header=False, padding=(0, 2))
        success_info.add_column("", style="white", width=80)
        success_info.add_row(endpoint_table)

        console.print(
            Panel(
                success_info,
                title="[bold green]✅ 服务器已启动[/bold green]",
                border_style="green",
                padding=(1, 2),
            )
        )

        if pro:
            console.print("\n  [bold green]💡 Pro 功能已启用[/bold green]")
            console.print("  [dim]• LLM 模型会在首次调用时自动加载[/dim]")
            console.print("  [dim]• 向量数据库已就绪,支持语义搜索[/dim]\n")
        else:
            console.print("\n  [bold yellow]💡 当前运行在标准模式[/bold yellow]")
            console.print("  [dim]• 支持 12+ 搜索引擎聚合[/dim]")
            console.print("  [dim]• 需要 LLM 功能? 使用 'seesea server --pro'[/dim]\n")

        # 启动服务器
        api_server.start()
    except KeyboardInterrupt:
        console.print("\n[bold yellow]⏹️  服务器已停止[/bold yellow]")
    except Exception as e:
        console.print(
            Panel(
                f"[red]错误: {e}[/red]",
                title="[bold red]❌ 服务器启动失败[/bold red]",
                border_style="red",
            )
        )
        sys.exit(1)


@cli.command()
@click.option("-j", "--json", is_flag=True, help="JSON 格式输出")
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
        console.print(json_module.dumps(stats_data, ensure_ascii=False, indent=2))
    else:
        stats_table = Table(title="SeeSea 统计信息", box=box.ROUNDED)
        stats_table.add_column("统计项", style="bold blue")
        stats_table.add_column("数值", style="bold green")

        stats_table.add_row("总搜索次数", str(stats_data.total_searches))
        stats_table.add_row("缓存命中", str(stats_data.cache_hits))
        stats_table.add_row("缓存未命中", str(stats_data.cache_misses))

        if stats_data.total_searches > 0:
            total_cache = stats_data.cache_hits + stats_data.cache_misses
            if total_cache > 0:
                hit_rate = stats_data.cache_hits / total_cache * 100
                stats_table.add_row("缓存命中率", f"{hit_rate:.1f}%")

        stats_table.add_row("引擎失败", str(stats_data.engine_failures))
        stats_table.add_row("超时次数", str(stats_data.timeouts))

        console.print(stats_table)


@cli.command()
@click.option(
    "-c", "--count", type=int, help="使用的引擎数量（按延迟排序，选择低延迟引擎）"
)
def interactive(count):
    """交互式搜索模式"""
    console.print("SeeSea 交互式搜索")
    console.print("━" * 50)
    console.print("输入查询来搜索，输入 'quit' 或 'exit' 退出")
    console.print("输入 'engines' 列出所有引擎")
    console.print("输入 'stats' 查看统计信息")
    console.print("输入 'count N' 设置使用的引擎数量")
    console.print("━" * 50)

    engine_count = count
    if engine_count:
        console.print(f"[green]当前引擎数量: {engine_count}[/green]")

    client = SearchClient()

    while True:
        try:
            from rich.prompt import Prompt

            prompt = "🔍 > "
            if engine_count:
                prompt = f"🔍 [green]引擎数量:{engine_count}[/green] > "

            query = Prompt.ask(prompt, console=console).strip()

            if not query:
                continue

            if query.lower() in ["quit", "exit"]:
                console.print("[green]再见！[/green]")
                break

            if query.lower() == "engines":
                engines({})
                continue

            if query.lower() == "stats":
                stats({})
                continue

            if query.lower().startswith("count"):
                parts = query.split()
                if len(parts) == 2 and parts[1].isdigit():
                    engine_count = int(parts[1])
                    if engine_count <= 0:
                        engine_count = None
                        console.print("[green]已切换到全引擎模式[/green]")
                    else:
                        console.print(f"[green]已设置引擎数量为 {engine_count}[/green]")
                else:
                    console.print(
                        "[yellow]用法: count N (N为引擎数量，0表示全部)[/yellow]"
                    )
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
                    # 根据引擎数量获取引擎列表
                    engines_list = None
                    if engine_count:
                        all_engines = client.list_engines()
                        engines_list = (
                            all_engines[:engine_count]
                            if engine_count < len(all_engines)
                            else None
                        )

                    results = client.search(
                        query=query, page=1, page_size=10, engines=engines_list
                    )
                    progress.update(task, description="搜索完成")

                except Exception as e:
                    progress.stop()
                    console.print(f"[red]搜索失败: {e}[/red]")
                    continue

            # 显示结果
            console.print("\n搜索结果:")
            console.print(
                f"总结果: {results.total_count}, 耗时: {results.query_time_ms}ms"
            )
            console.print(f"引擎: {', '.join(results.engines_used)}")

            formatted = format_results(results.results, max_description_length=120)
            console.print("\n结果列表:\n")

            for i, item in enumerate(formatted[:10], 1):
                content = Text()
                content.append(f"{i}. ", style="cyan")
                content.append(item["title"], style="bold")

                if item["description"]:
                    desc = (
                        item["description"][:120] + "..."
                        if len(item["description"]) > 120
                        else item["description"]
                    )
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


def _init_standard_embedding():
    """初始化标准模式嵌入模型并注册回调"""
    try:
        from .embeddings import EmbeddingManager

        # 使用标准模式（轻量级）
        manager = EmbeddingManager(pro_mode=False)
        manager.register_rust_callback()
        console.print("[dim]✓ 标准嵌入模型已初始化[/dim]")
    except Exception as e:
        # 嵌入模型初始化失败不应该阻止 CLI 正常运行
        console.print(f"[dim yellow]⚠ 嵌入模型初始化失败: {e}[/dim yellow]")


def main():
    """主入口函数，供 __main__.py 调用"""
    # 初始化标准模式嵌入模型
    _init_standard_embedding()
    cli()


if __name__ == "__main__":
    main()
