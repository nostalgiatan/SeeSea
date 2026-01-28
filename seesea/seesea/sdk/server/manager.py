"""
SeeSea 服务器管理器

提供API服务器的启动、关闭和管理功能。
不提供客户端接口，而是专注于服务器生命周期管理。
"""

from typing import Optional, Dict, Any
from enum import Enum
import threading
import time
import signal
import sys
from datetime import datetime

from ..seesea_types.common_types import Error

try:
    from seesea_core import PyApiServer

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False


class ServerStatus(Enum):
    """服务器状态"""

    STOPPED = "stopped"
    STARTING = "starting"
    RUNNING = "running"
    STOPPING = "stopping"
    ERROR = "error"


class ApiServerManager:
    """
    SeeSea API 服务器管理器

    提供完整的API服务器生命周期管理，包括启动、停止、状态监控等功能。
    """

    # 全局实例引用，用于信号处理
    _instance: Optional["ApiServerManager"] = None

    def __init__(
        self,
        host: Optional[str] = None,
        port: Optional[int] = None,
        network_mode: str = "internal",
        config_file: Optional[str] = None,
        auto_restart: bool = False,
        verbose: bool = False,
    ):
        """
        初始化服务器管理器

        Args:
            host: 监听地址 (默认: "127.0.0.1")
            port: 监听端口 (默认: 8080)
            network_mode: 网络模式 - "internal", "external", 或 "dual"
            config_file: 配置文件路径
            auto_restart: 是否自动重启
            verbose: 是否输出详细日志
        """
        if not _CORE_AVAILABLE:
            raise ImportError("seesea_core not available")

        if network_mode not in ["internal", "external", "dual"]:
            raise ValueError("network_mode must be 'internal', 'external', or 'dual'")

        self.host = host if host is not None else "127.0.0.1"
        self.port = port if port is not None else 8080
        self.network_mode = network_mode
        self.config_file = config_file
        self.auto_restart = auto_restart
        self.verbose = verbose

        self._server: Optional[PyApiServer] = None
        self._status = ServerStatus.STOPPED
        self._start_time: Optional[datetime] = None
        self._last_error: Optional[Error] = None
        self._server_thread: Optional[threading.Thread] = None
        self._stop_event = threading.Event()
        self._shutdown_complete = threading.Event()

        # 设置全局实例
        ApiServerManager._instance = self

        # 注册信号处理器
        self._setup_signal_handlers()

        if self.verbose:
            print(f"📋 SeeSea API服务器管理器已初始化 ({self.host}:{self.port})")

    def _setup_signal_handlers(self):
        """设置信号处理器"""

        def signal_handler(signum, frame):
            print("\n⏹️  收到停止信号，正在关闭...")
            self._graceful_shutdown()
            self._shutdown_complete.set()
            sys.exit(0)

        signal.signal(signal.SIGINT, signal_handler)
        signal.signal(signal.SIGTERM, signal_handler)

    def start(self, blocking: bool = True) -> bool:
        """
        启动API服务器

        Args:
            blocking: 是否阻塞运行 (默认: True)

        Returns:
            bool: 启动是否成功
        """
        if self._status == ServerStatus.RUNNING:
            return True

        if self._status == ServerStatus.STARTING:
            return False

        try:
            self._status = ServerStatus.STARTING
            self._last_error = None

            # 初始化PyApiServer
            if self.config_file:
                self._server = PyApiServer(
                    None, None, self.network_mode, config_file=self.config_file
                )
            else:
                self._server = PyApiServer(
                    self.host,
                    self.port,
                    self.network_mode,
                    config_file=self.config_file,
                )

            # 执行初始化流程
            self._initialize_services()

            print(f"🚀 SeeSea API 服务器启动中 ({self.host}:{self.port})...")

            if blocking:
                self._run_server()
            else:
                self._stop_event.clear()
                self._server_thread = threading.Thread(
                    target=self._run_server_background, daemon=True
                )
                self._server_thread.start()

                timeout = 10
                start_check = time.time()
                while (
                    self._status == ServerStatus.STARTING
                    and (time.time() - start_check) < timeout
                ):
                    time.sleep(0.1)

                return self._status == ServerStatus.RUNNING

            return True

        except KeyboardInterrupt:
            self._graceful_shutdown()
            return False
        except Exception as e:
            self._status = ServerStatus.ERROR
            self._last_error = Error(
                code="ApiServerManager.start",
                message=f"启动失败: {e}",
                timestamp=datetime.now(),
            )
            print(f"❌ 启动失败: {e}")
            return False

    def _run_server(self):
        """运行服务器（内部方法）"""
        try:
            self._status = ServerStatus.RUNNING
            self._start_time = datetime.now()
            print(f"✅ SeeSea API 服务器已启动 ({self.host}:{self.port})")

            if self._server:
                self._server.start()

        except Exception as e:
            self._status = ServerStatus.ERROR
            self._last_error = Error(
                code="ApiServerManager._run_server",
                message=f"运行异常: {e}",
                timestamp=datetime.now(),
            )
            if self.verbose:
                print(f"❌ 运行异常: {e}")
            raise

    def _run_server_background(self):
        """后台运行服务器"""
        try:
            self._run_server()
        except Exception as e:
            if self.verbose:
                print(f"❌ 后台服务器异常: {e}")
            if self.auto_restart and not self._stop_event.is_set():
                time.sleep(5)
                self._run_server_background()

    def stop(self, timeout: int = 30) -> bool:
        """停止API服务器"""
        if self._status == ServerStatus.STOPPED:
            return True

        if self._status == ServerStatus.STOPPING:
            return False

        try:
            self._status = ServerStatus.STOPPING
            self._graceful_shutdown()

            if self._server_thread and self._server_thread.is_alive():
                self._server_thread.join(timeout=timeout)
                if self._server_thread.is_alive():
                    return False

            self._status = ServerStatus.STOPPED
            print("✅ 服务器已停止")
            return True

        except Exception as e:
            self._status = ServerStatus.ERROR
            self._last_error = Error(
                code="ApiServerManager.stop",
                message=f"停止失败: {e}",
                timestamp=datetime.now(),
            )
            return False

    def _graceful_shutdown(self):
        """优雅关闭"""
        self._stop_event.set()

    def restart(self) -> bool:
        """重启服务器"""
        if not self.stop():
            return False
        time.sleep(1)
        return self.start(blocking=False)

    def get_status(self) -> Dict[str, Any]:
        """获取服务器状态信息"""
        uptime_seconds = 0
        if self._start_time and self._status == ServerStatus.RUNNING:
            uptime_seconds = int((datetime.now() - self._start_time).total_seconds())

        return {
            "status": self._status.value,
            "host": self.host,
            "port": self.port,
            "network_mode": self.network_mode,
            "start_time": self._start_time.isoformat() if self._start_time else None,
            "uptime_seconds": uptime_seconds,
            "last_error": (
                {
                    "code": self._last_error.code,
                    "message": self._last_error.message,
                    "timestamp": (
                        self._last_error.timestamp.isoformat()
                        if self._last_error.timestamp
                        else None
                    ),
                }
                if self._last_error
                else None
            ),
        }

    def is_running(self) -> bool:
        """检查服务器是否正在运行"""
        return self._status == ServerStatus.RUNNING

    def is_healthy(self) -> bool:
        """检查服务器是否健康"""
        return self._status == ServerStatus.RUNNING and self._last_error is None

    def _initialize_services(self):
        """初始化相关服务"""
        self._init_embedding()
        self._init_system_controller()
        # 股票服务由 seesea-core 自动处理，不再需要这里初始化

    def _init_embedding(self) -> None:
        """初始化嵌入模型"""
        try:
            from seesea.embeddings import EmbeddingManager, EmbeddingMode
            from seesea_core import register_embedding_callback

            embedding_manager = EmbeddingManager.get_instance(
                mode=EmbeddingMode.STANDARD
            )
            callback = embedding_manager.register_callback()
            dimension = embedding_manager.get_dimension()

            register_embedding_callback(callback, dimension, "standard", 4)

            if self.verbose:
                print(f"✅ 嵌入模型已加载，维度: {dimension}")

        except ImportError:
            pass
        except Exception as e:
            if self.verbose:
                print(f"⚠️  嵌入模型初始化失败: {e}")

    def _init_system_controller(self) -> None:
        """初始化系统控制器"""
        try:
            from seesea_core import start_system_controller_daemon

            start_system_controller_daemon()
        except (ImportError, Exception):
            pass
