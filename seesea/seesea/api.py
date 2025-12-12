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
SeeSea API Server - API 服务器

提供完整的 REST API 接口，支持搜索、RSS、缓存管理、统计、健康检查等功能。

主要功能:
- 搜索接口（支持 GET/POST）
- 引擎管理和状态监控
- RSS 订阅和模板管理
- 缓存管理和统计
- 健康检查和指标导出
- 支持多种网络模式（内网、外网、双模式）
- 安全机制（CORS、IP过滤、速率限制、熔断）
- Pro API 扩展支持
- 嵌入向量化支持（标准模式和Pro模式）

API 端点分类:
- 搜索相关: /api/search, /api/engines
- 统计相关: /api/stats, /api/metrics
- 健康相关: /api/health, /api/version
- RSS 相关: /api/rss/*
- 缓存相关: /api/cache/*
- Pro 扩展: /api/pro/*

性能特性:
- 异步处理，高并发支持
- 共享连接池
- 智能路由匹配
- 完整的指标监控
- 向量化相关性评分（标准模式使用轻量级模型）
"""

from typing import Optional, Dict, List
from seesea_core import PyApiServer  # type: ignore[import-untyped]
from .embeddings.manager import EmbeddingManager


class ApiServer:
    """
    SeeSea API 服务器

    提供完整的 REST API 接口，支持搜索、RSS、缓存管理、统计、健康检查等功能。

    参数:
        host: 监听地址 (默认: "127.0.0.1")
        port: 监听端口 (默认: 8080)
        network_mode: 网络模式 - "internal", "external", 或 "dual" (默认: "internal")

    示例:
        >>> # 启动内网服务器（无安全限制）
        >>> server = ApiServer(host="127.0.0.1", port=8080)
        >>> server.start()  # 阻塞运行

        >>> # 启动外网服务器（带安全限制）
        >>> server = ApiServer(host="0.0.0.0", port=8080, network_mode="external")
        >>> server.start_external()

        >>> # 双模式（同时启动内网和外网）
        >>> server = ApiServer(network_mode="dual")
        >>> server.start()
    """

    def __init__(
        self,
        host: Optional[str] = None,
        port: Optional[int] = None,
        network_mode: str = "internal",
        config_file: Optional[str] = None,
        enable_pro: bool = False,
    ):
        """
        初始化 API 服务器

        Args:
            host: 监听地址
            port: 监听端口
            network_mode: 网络模式 - "internal"（内网）, "external"（外网）, 或 "dual"（双模式）
            config_file: 配置文件路径
            enable_pro: 是否启用 Pro 功能（默认: False）
                       启用后会加载 LLM、向量数据库等高级功能
                       ⚠️  注意: 首次启用会下载 ~2GB 的模型数据

        Raises:
            ValueError: 当 network_mode 不是有效值时
        """
        if network_mode not in ["internal", "external", "dual"]:
            raise ValueError("network_mode must be 'internal', 'external', or 'dual'")

        # 如果提供了配置文件，不传递host和port，让PyApiServer自己从配置文件中获取
        if config_file:
            # 使用配置文件时，不传递host和port，让PyApiServer自己从配置文件中获取
            self._server = PyApiServer(None, None, network_mode, config_file=config_file)
            # 这些值会被PyApiServer实际使用的值覆盖
            self.host = "127.0.0.1"
            self.port = 8080
        else:
            # 没有配置文件时，使用提供的值或默认值
            self._server = PyApiServer(host, port, network_mode, config_file=config_file)
            self.host = host if host is not None else "127.0.0.1"
            self.port = port if port is not None else 8080
        self.network_mode = network_mode
        self.config_file = config_file
        self.enable_pro = enable_pro
        self._embedding_manager: Optional[EmbeddingManager] = None

        # 初始化嵌入模型并注册回调
        self._init_embedding(enable_pro)

        # 初始化调控中心守护进程
        self._init_system_controller()

        # 仅在显式启用时初始化Pro API路由和处理器
        if enable_pro:
            try:
                from seesea.handlers.pro import add_pro_routes, initialize_pro_handlers
                import asyncio

                # 注册Pro路由
                add_pro_routes(self)

                # 初始化Pro处理器
                asyncio.run(initialize_pro_handlers())
                print("✅ Pro API routes initialized and handlers started")
            except ImportError as e:
                print(f"⚠️  Pro features not available: {e}")
                print("   Install with: pip install llama-cpp-python")
                print(
                    "   Or use pre-built: pip install llama-cpp-python --index-url https://abetlen.github.io/llama-cpp-python/whl/cpu"
                )
            except Exception as e:
                print(f"⚠️  Failed to initialize Pro API routes: {e}")
                import traceback

                print(f"   Detailed error: {traceback.format_exc()}")
        else:
            print("ℹ️  Running in standard mode with lightweight embedding model.")

    def _init_embedding(self, enable_pro: bool) -> None:
        """
        初始化嵌入模型并注册回调

        Args:
            enable_pro: 是否启用 Pro 模式
        """
        try:
            from seesea.embeddings import EmbeddingManager, EmbeddingMode
            from seesea_core import register_embedding_callback

            # 根据模式选择嵌入器
            mode = EmbeddingMode.PRO if enable_pro else EmbeddingMode.STANDARD
            mode_name = "Pro (Qwen3-Q8_0)" if enable_pro else "Standard (MiniLM-Q4)"

            print(f"🔄 初始化嵌入模型 ({mode_name})...")

            # 创建嵌入管理器
            self._embedding_manager = EmbeddingManager.get_instance(mode=mode)

            # 注册回调到 Rust
            callback = self._embedding_manager.register_callback()
            dimension = self._embedding_manager.get_dimension()

            register_embedding_callback(
                callback, dimension, "pro" if enable_pro else "standard", 4  # 最大并发数
            )

            print(f"✅ 嵌入模型已加载，维度: {dimension}")

        except ImportError as e:
            print(f"⚠️  嵌入模型初始化跳过: {e}")
            print("   提示: 安装 llama-cpp-python 以启用向量化功能")
        except Exception as e:
            print(f"⚠️  嵌入模型初始化失败: {e}")

    def _init_system_controller(self) -> None:
        """
        初始化调控中心守护进程

        确保系统控制器在后台运行，监控资源使用和动态调整并发。
        """
        try:
            from seesea_core import start_system_controller_daemon

            print("🔄 启动调控中心守护进程...")
            start_system_controller_daemon()
            print("✅ 调控中心守护进程已启动")

        except ImportError:
            # 如果 seesea_core 不支持此功能，静默跳过
            pass
        except Exception as e:
            print(f"⚠️  调控中心守护进程启动失败: {e}")

    def start(self):
        """
        启动 API 服务器（阻塞）

        根据初始化时指定的 network_mode 启动相应模式的服务器。

        内网模式路由（无安全限制）:
        - GET/POST /api/search - 搜索
        - GET /api/engines - 引擎列表
        - GET /api/stats - 统计信息
        - GET /api/health - 健康检查
        - GET /api/version - 版本信息
        - GET /api/metrics - Prometheus 格式指标
        - GET /api/metrics/realtime - JSON 格式实时指标
        - GET /api/rss/feeds - RSS 源列表
        - POST /api/rss/fetch - 获取 RSS 内容
        - GET /api/rss/templates - RSS 模板列表
        - POST /api/rss/template/add - 添加 RSS 模板
        - GET /api/cache/stats - 缓存统计
        - POST /api/cache/clear - 清空缓存
        - POST /api/cache/cleanup - 清理过期缓存
        - POST /api/magic-link/generate - 生成魔法链接

        外网模式路由（带安全限制）:
        - 基础搜索和查询路由（启用限流、熔断、IP过滤、JWT认证等）

        Raises:
            RuntimeError: 服务器启动失败时抛出
        """
        self._server.start()

    def start_internal(self):
        """
        启动内网模式服务器（阻塞）

        明确使用内网路由器启动，无安全限制。
        适合本地开发和内部网络使用。

        Raises:
            RuntimeError: 服务器启动失败时抛出
        """
        self._server.start_internal()

    def start_external(self):
        """
        启动外网模式服务器（阻塞）

        明确使用外网路由器启动，启用所有安全特性：
        - 请求限流
        - 熔断保护
        - IP 过滤
        - JWT 认证
        - 魔法链接

        Raises:
            RuntimeError: 服务器启动失败时抛出
        """
        self._server.start_external()

    @property
    def address(self) -> str:
        """获取服务器地址 (host:port)"""
        return self._server.get_address()  # type: ignore[no-any-return]

    @property
    def url(self) -> str:
        """获取服务器完整 URL"""
        return self._server.get_url()  # type: ignore[no-any-return]

    def get_endpoints(self) -> Dict[str, List[str]]:
        """
        获取当前模式下可用的 API 端点

        Returns:
            Dict[str, List[str]]: 端点分类及其路径列表
        """
        endpoints_list = self._server.get_endpoints()
        return {category: routes for category, routes in endpoints_list}

    def print_endpoints(self):
        """打印所有可用的 API 端点"""
        self.get_endpoints()

    def __repr__(self) -> str:
        return f"<ApiServer(address='{self.address}', mode='{self.network_mode}')>"

    def add_pro_route(self, path: str, callback, method: str = "POST") -> None:
        """
        添加 Pro API 路由

        Args:
            path: 路由路径（如 "/process-url"，自动添加 "/api/pro/" 前缀）
            callback: Python 回调函数，接收请求上下文并返回响应字典
            method: HTTP 方法（默认: "POST"）

        示例:
            >>> def my_callback(req):
            ...     return {"status": 200, "body": "{\"message\": \"Hello from Pro API\"}"}
            >>> server = ApiServer()
            >>> server.add_pro_route("/hello", my_callback, method="GET")
        """
        self._server.add_pro_route(path, callback, method)

    def __str__(self) -> str:
        return f"SeeSea API Server @ {self.url} ({self.network_mode} mode)"
