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

提供服务器管理功能
"""

import click
import sys
from rich.console import Console
from rich.table import Table
from rich.panel import Panel
from rich import box

try:
    from ..sdk.server import ApiServerManager
    from ..sdk.stock import StockScheduler
except ImportError:
    from seesea.sdk.server import ApiServerManager
    from seesea.sdk.stock import StockScheduler

# 检查 MCP 依赖
try:
    from ..mcp import create_mcp_server

    _MCP_AVAILABLE = True
except ImportError:
    try:
        from seesea.mcp import create_mcp_server

        _MCP_AVAILABLE = True
    except ImportError:
        _MCP_AVAILABLE = False

# 初始化 Rich Console
console = Console()


@click.group(invoke_without_command=True, help="SeeSea - 隐私保护型元搜索引擎")
@click.pass_context
def cli(ctx):
    """SeeSea - 隐私保护型元搜索引擎服务器管理"""
    if ctx.invoked_subcommand is None:
        console.print("[yellow]使用 'seesea <command>' 命令：[/yellow]")
        console.print("  [cyan]server[/cyan]         - 启动 API 服务器")
        console.print("  [cyan]stock-scheduler[/cyan] - 启动股票数据调度器")
        console.print("  [cyan]mcp[/cyan]            - MCP 服务器管理")
        console.print("\n运行 'seesea <command> --help' 查看详细选项")


@cli.command()
@click.option("--host", default=None, help="监听地址 (默认: 配置文件中的地址)")
@click.option(
    "--port", type=int, default=None, help="监听端口 (默认: 配置文件中的端口)"
)
@click.option("-c", "--config", default=None, help="配置文件路径")
def server(host, port, config):
    """启动 API 服务器"""
    try:
        # 创建服务器管理器，传入配置参数
        server_manager = ApiServerManager(host=host, port=port, config_file=config)

        # 显示启动前的配置信息
        server_info = Table(box=box.ROUNDED, show_header=False)
        server_info.add_column("属性", style="cyan bold", width=20)
        server_info.add_column("值", style="white")

        server_info.add_row("📡 服务", "SeeSea API 服务器")

        # 获取实际配置
        actual_host = server_manager.host
        actual_port = server_manager.port
        server_info.add_row("🌐 监听地址", f"{actual_host}:{actual_port}")

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

        console.print(
            "[bold green]⏳ 服务器启动中...[/bold green] [dim]按 Ctrl+C 停止[/dim]\n"
        )

        # 显示API端点信息
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

        console.print("\n  [bold green]💡 SeeSea API服务器已就绪[/bold green]")
        console.print("  [dim]• 支持 12+ 搜索引擎聚合[/dim]")
        console.print("  [dim]• 完整的REST API接口[/dim]\n")

        # 启动服务器
        success = server_manager.start(blocking=True)

        if not success:
            console.print(
                Panel(
                    "[red]服务器启动失败[/red]",
                    title="[bold red]❌ 启动失败[/bold red]",
                    border_style="red",
                )
            )
            sys.exit(1)

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
@click.option("-c", "--config", default=None, help="调度器配置文件路径")
def stock_scheduler(config):
    """启动股票数据调度器"""
    try:
        # 显示启动前的配置信息
        scheduler_info = Table(box=box.ROUNDED, show_header=False)
        scheduler_info.add_column("属性", style="cyan bold", width=20)
        scheduler_info.add_column("值", style="white")

        scheduler_info.add_row("📈 服务", "SeeSea 股票数据调度器")

        if config:
            scheduler_info.add_row("⚙️  配置文件", config)
        else:
            scheduler_info.add_row("⚙️  配置文件", "使用默认配置")

        console.print(
            Panel(
                scheduler_info,
                title="[bold white]📈 调度器配置[/bold white]",
                border_style="cyan",
                padding=(1, 2),
            )
        )

        console.print(
            "[bold green]⏳ 调度器启动中...[/bold green] [dim]按 Ctrl+C 停止[/dim]\n"
        )

        # 显示调度器信息
        task_table = Table(
            box=box.SIMPLE, show_header=True, header_style="bold magenta"
        )
        task_table.add_column("功能", style="cyan", width=30)
        task_table.add_column("说明", style="white")

        task_table.add_row("实时行情", "A股/B股/港股/美股实时行情")
        task_table.add_row("历史数据", "K线数据自动更新")
        task_table.add_row("板块数据", "行业/概念板块数据")
        task_table.add_row("指数数据", "市场指数实时更新")

        success_info = Table(box=box.ROUNDED, show_header=False, padding=(0, 2))
        success_info.add_column("", style="white", width=80)
        success_info.add_row(task_table)

        console.print(
            Panel(
                success_info,
                title="[bold green]✅ 调度器已启动[/bold green]",
                border_style="green",
                padding=(1, 2),
            )
        )

        console.print("\n  [bold green]💡 股票数据调度器已就绪[/bold green]")
        console.print("  [dim]• 自动更新股票数据[/dim]")
        console.print("  [dim]• 支持多种数据源[/dim]\n")

        # 启动调度器
        result = StockScheduler.start()

        if not result.success:
            console.print(
                Panel(
                    f"[red]调度器启动失败: {result.error.message}[/red]",
                    title="[bold red]❌ 启动失败[/bold red]",
                    border_style="red",
                )
            )
            sys.exit(1)

        console.print("[dim]\n调度器正在后台运行，按 Ctrl+C 停止...[/dim]")

        # 保持程序运行，等待 Ctrl+C
        import signal
        import time as _time

        def signal_handler(sig, frame):
            console.print("\n[yellow]⏹️  正在停止调度器...[/yellow]")
            stop_result = StockScheduler.stop()
            if not stop_result.success:
                console.print(f"[red]停止调度器失败: {stop_result.error.message}[/red]")
            else:
                console.print("[bold green]✅ 调度器已停止[/bold green]")
            sys.exit(0)

        signal.signal(signal.SIGINT, signal_handler)
        signal.signal(signal.SIGTERM, signal_handler)

        # 主循环，保持进程运行
        try:
            while True:
                _time.sleep(1)
        except Exception as e:
            console.print(f"\n[red]错误: {e}[/red]")
            sys.exit(1)

    except Exception as e:
        console.print(
            Panel(
                f"[red]错误: {e}[/red]",
                title="[bold red]❌ 调度器启动失败[/bold red]",
                border_style="red",
            )
        )
        sys.exit(1)


@cli.group(help="MCP (Model Context Protocol) 服务器管理")
@click.pass_context
def mcp(ctx):
    """MCP 服务器管理"""
    pass


@mcp.command()
@click.option("--name", default="seesea", help="服务器名称")
@click.option("--port", type=int, default=8000, help="服务器端口（SSE模式）")
@click.option("--host", default="127.0.0.1", help="服务器地址（SSE模式）")
@click.option(
    "--stdio", is_flag=True, help="使用 stdio 模式运行（MCP 客户端标准输入输出）"
)
def start(name, port, host, stdio):
    """启动 MCP 服务器"""
    if not _MCP_AVAILABLE:
        console.print(
            Panel(
                "[red]MCP 特性未安装！[/red]\n\n"
                "[yellow]请运行以下命令安装 MCP 特性：[/yellow]\n"
                "[cyan]pip install seesea[mcp][/cyan]\n\n"
                "[dim]或者：[/dim]\n"
                "[cyan]pip install fastmcp[/cyan]",
                title="[bold red]❌ 依赖缺失[/bold red]",
                border_style="red",
                padding=(1, 2),
            )
        )
        sys.exit(1)

    try:
        # 创建 MCP 服务器
        mcp_server = create_mcp_server(name=name)

        if stdio:
            # stdio 模式 - 用于 MCP 客户端
            # 将欢迎消息输出到 stderr，避免干扰 MCP 协议的 JSON-RPC 通信
            import sys as _sys

            _sys.stderr.write("✅ SeeSea MCP 服务器已启动 (stdio模式)\n")
            _sys.stderr.write(f"服务器名称: {name}\n")
            _sys.stderr.write("正在等待 MCP 客户端连接...\n")
            _sys.stderr.flush()

            # stdio 模式运行（不输出到 stdout）
            mcp_server.run(transport="stdio")

        else:
            # SSE 模式 - HTTP 服务器
            # 显示启动信息
            server_info = Table(box=box.ROUNDED, show_header=False)
            server_info.add_column("属性", style="cyan bold", width=20)
            server_info.add_column("值", style="white")

            server_info.add_row("🤖 服务", "SeeSea MCP 服务器")
            server_info.add_row("🌐 监听地址", f"{host}:{port}")
            server_info.add_row("📋 服务器名称", name)

            console.print(
                Panel(
                    server_info,
                    title="[bold white]🚀 MCP 服务器配置[/bold white]",
                    border_style="cyan",
                    padding=(1, 2),
                )
            )

            # 显示可用工具信息
            # 注意：FastMCP 可能没有 _mcp_tools 属性，使用固定的工具数量
            tool_count = 35  # 搜索7 + RSS7 + 股票16 + 清洗5

            tools_table = Table(
                box=box.SIMPLE, show_header=True, header_style="bold magenta"
            )
            tools_table.add_column("模块", style="cyan", width=20)
            tools_table.add_column("工具数", style="yellow", width=10)
            tools_table.add_row("搜索", "7")
            tools_table.add_row("RSS", "7")
            tools_table.add_row("股票", "16")
            tools_table.add_row("清洗", "5")

            success_info = Table(box=box.ROUNDED, show_header=False, padding=(0, 2))
            success_info.add_column("", style="white", width=80)
            success_info.add_row(tools_table)

            console.print(
                Panel(
                    success_info,
                    title=f"[bold green]✅ 服务器已就绪 ({tool_count} 个工具)[/bold green]",
                    border_style="green",
                    padding=(1, 2),
                )
            )

            console.print("\n  [bold green]💡 SeeSea MCP服务器已就绪[/bold green]")
            console.print("  [dim]• 搜索: 文本/图片/视频搜索[/dim]")
            console.print("  [dim]• RSS: 订阅源获取和解析[/dim]")
            console.print("  [dim]• 股票: 实时行情和历史数据[/dim]")
            console.print("  [dim]• 清洗: 文本数据处理[/dim]\n")
            console.print(f"  [dim]访问: http://{host}:{port}/sse[/dim]\n")

            # 启动服务器（fastmcp 使用 SSE 传输）
            console.print(
                "[bold green]⏳ MCP 服务器启动中...[/bold green] [dim]按 Ctrl+C 停止[/dim]\n"
            )

            # SSE 模式运行
            mcp_server.run(transport="sse", host=host, port=port)

    except KeyboardInterrupt:
        console.print("\n[bold yellow]⏹️  MCP 服务器已停止[/bold yellow]")
    except Exception as e:
        console.print(
            Panel(
                f"[red]错误: {e}[/red]",
                title="[bold red]❌ 服务器启动失败[/bold red]",
                border_style="red",
            )
        )
        sys.exit(1)


@mcp.command()
@click.option(
    "--format", type=click.Choice(["text", "json"]), default="text", help="输出格式"
)
def list(format):
    """列出所有可用的 MCP 工具"""
    from seesea.mcp import create_mcp_server
    import asyncio

    mcp_server = create_mcp_server()

    if format == "json":
        # 输出 JSON 格式的 MCP 配置
        config = {
            "mcpServers": {
                "seesea": {
                    "command": "python",
                    "args": ["-m", "seesea.cli", "mcp", "start", "--stdio"],
                    "description": "SeeSea MCP 服务器 - 数据聚合、RSS订阅、股票数据、文本清洗",
                    "tools": [],
                }
            }
        }

        # 获取工具详细信息
        async def get_tools_info():
            from fastmcp.client import Client

            async with Client(mcp_server) as client:
                tools = await client.list_tools()
                for tool in tools:
                    tool_info = {"name": tool.name, "description": tool.description}
                    if tool.inputSchema:
                        tool_info["inputSchema"] = tool.inputSchema
                    config["mcpServers"]["seesea"]["tools"].append(tool_info)
                return config

        try:
            result = asyncio.run(get_tools_info())
            import json

            print(json.dumps(result, indent=2, ensure_ascii=False))
        except Exception:
            # 如果获取详细信息失败，使用 get_tools 获取基本工具列表
            async def get_basic_tools():
                tools = await mcp_server.get_tools()
                return [{"name": t.name} for t in tools]

            try:
                tools = asyncio.run(get_basic_tools())
                config["mcpServers"]["seesea"]["tools"] = tools
                import json

                print(json.dumps(config, indent=2, ensure_ascii=False))
            except Exception:
                # 最后的降级方案
                import json

                config["mcpServers"]["seesea"]["tools"] = []
                print(json.dumps(config, indent=2, ensure_ascii=False))
    else:
        # 输出文本格式
        async def get_text_format():
            tools = await mcp_server.get_tools()
            print("📦 SeeSea MCP 服务器工具列表\n")
            print(f"服务器名称: {mcp_server.name}")
            print(f"工具总数: {len(tools)}")
            print("\n工具列表:")
            for tool in tools:
                print(f"  • {tool.name}")
            print("\n💡 使用 'seesea mcp list --format json' 输出 JSON 格式配置")
            print("💡 使用 'seesea mcp start --stdio' 启动 MCP 服务器")

        try:
            asyncio.run(get_text_format())
        except Exception as e:
            print(f"获取工具列表失败: {e}")
    """列出所有可用的 MCP 工具"""
    if not _MCP_AVAILABLE:
        console.print(
            Panel(
                "[red]MCP 特性未安装！[/red]\n\n"
                "[yellow]请运行以下命令安装 MCP 特性：[/yellow]\n"
                "[cyan]pip install seesea[mcp][/cyan]",
                title="[bold red]❌ 依赖缺失[/bold red]",
                border_style="red",
                padding=(1, 2),
            )
        )
        sys.exit(1)

    try:
        # 显示工具列表（静态列表，不动态获取）
        tools_table = Table(box=box.ROUNDED)
        tools_table.add_column("模块", style="cyan bold", width=15)
        tools_table.add_column("工具", style="yellow", width=30)
        tools_table.add_column("说明", style="white")

        # 搜索模块
        tools_table.add_row("搜索", "search", "执行搜索查询")
        tools_table.add_row("搜索", "search_images", "搜索图片")
        tools_table.add_row("搜索", "search_videos", "搜索视频")
        tools_table.add_row("搜索", "list_engines", "获取搜索引擎列表")
        tools_table.add_row("搜索", "get_search_info", "获取搜索客户端信息")
        tools_table.add_row("搜索", "clear_cache", "清除搜索缓存")
        tools_table.add_row("搜索", "get_stats", "获取搜索统计信息")

        # RSS 模块
        tools_table.add_row("RSS", "fetch_feed", "获取 RSS feed")
        tools_table.add_row("RSS", "parse_feed", "解析 RSS 内容")
        tools_table.add_row("RSS", "list_templates", "列出 RSS 模板")
        tools_table.add_row("RSS", "add_from_template", "从模板添加 feeds")
        tools_table.add_row("RSS", "create_ranking", "创建 RSS 榜单")
        tools_table.add_row("RSS", "get_template_info", "获取模板信息")
        tools_table.add_row("RSS", "get_rss_info", "获取 RSS 客户端信息")

        # 股票模块
        tools_table.add_row("股票", "get_stock_list", "获取股票列表")
        tools_table.add_row("股票", "get_stock_info", "获取个股信息")
        tools_table.add_row("股票", "get_quote", "获取实时行情")
        tools_table.add_row("股票", "get_quotes", "获取全市场行情")
        tools_table.add_row("股票", "get_kline", "获取 K 线数据")
        tools_table.add_row("股票", "get_kline_hk", "获取港股 K 线")
        tools_table.add_row("股票", "get_industry_list", "获取行业列表")
        tools_table.add_row("股票", "get_concept_list", "获取概念列表")
        tools_table.add_row("股票", "get_industry_stocks", "获取行业成分股")
        tools_table.add_row("股票", "get_concept_stocks", "获取概念成分股")
        tools_table.add_row("股票", "get_index_list", "获取指数列表")
        tools_table.add_row("股票", "get_market_fund_flow", "获取资金流向")
        tools_table.add_row("股票", "get_zt_pool", "获取涨停板")
        tools_table.add_row("股票", "get_dt_pool", "获取跌停板")
        tools_table.add_row("股票", "search_stock", "搜索股票")
        tools_table.add_row("股票", "get_stock_client_info", "获取股票客户端信息")

        # 热点模块
        tools_table.add_row("热点", "fetch_hot_platform", "获取平台热点")
        tools_table.add_row("热点", "fetch_all_hot_platforms", "获取所有平台热点")
        tools_table.add_row("热点", "fetch_multiple_hot_platforms", "批量获取平台热点")
        tools_table.add_row("热点", "list_hot_platforms", "列出支持的平台")
        tools_table.add_row("热点", "search_hot_platforms", "搜索平台")
        tools_table.add_row("热点", "get_hot_client_info", "获取热点客户端信息")

        # 清洗模块
        tools_table.add_row("清洗", "clean_text", "清洗文本")
        tools_table.add_row("清洗", "remove_html", "移除 HTML 标签")
        tools_table.add_row("清洗", "normalize_text", "标准化文本")
        tools_table.add_row("清洗", "extract_urls", "提取 URL")
        tools_table.add_row("清洗", "clean_batch", "批量清洗文本")

        console.print(
            Panel(
                tools_table,
                title="[bold white]📋 可用工具列表 (41 个)[/bold white]",
                border_style="cyan",
                padding=(1, 2),
            )
        )

    except Exception as e:
        console.print(
            Panel(
                f"[red]错误: {e}[/red]",
                title="[bold red]❌ 获取工具列表失败[/bold red]",
                border_style="red",
            )
        )
        sys.exit(1)


def main():
    """主入口函数，供 __main__.py 调用"""
    cli()


if __name__ == "__main__":
    main()
