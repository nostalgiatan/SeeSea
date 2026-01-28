# -*- coding: utf-8 -*-
"""
标准模式嵌入器

优先使用轻量级 all-MiniLM-L6-v2-Q4_K_M 模型（需要 llama-cpp-python），
如果未安装则回退到纯 Python 实现的简单向量化器。
"""

from typing import List, Optional, Union, Any, cast
import os
from .manager import BaseEmbedder

# 检查 llama-cpp-python 是否可用
try:
    from seesea_core import get_file
    import importlib.util

    spec = importlib.util.find_spec("llama_cpp")
    LLAMA_CPP_AVAILABLE = spec is not None
except (ImportError, AttributeError):
    LLAMA_CPP_AVAILABLE = False

# 导入简单向量化器作为后备
from .simple import SimpleEmbedder


class StandardEmbedder(BaseEmbedder):
    """
    标准模式嵌入器

    如果 llama-cpp-python 可用，使用 all-MiniLM-L6-v2-Q4_K_M 模型：
    - 模型小巧（~23MB）
    - 推理速度快
    - 维度384，足够用于相关性计算

    如果 llama-cpp-python 不可用，使用纯 Python 实现的简单向量化器：
    - 无需外部依赖
    - 固定维度 512
    - 基于词频和哈希
    """

    # 模型配置（仅在使用 llama-cpp-python 时使用）
    MODEL_FILENAME = "all-MiniLM-L6-v2-Q4_K_M.gguf"
    MODEL_URL = "https://hf-mirror.com/second-state/All-MiniLM-L6-v2-Embedding-GGUF/resolve/main/all-MiniLM-L6-v2-Q4_K_M.gguf?download=true"
    EXPECTED_DIMENSION = 384

    def __init__(
        self,
        model_path: Optional[str] = None,
        device: Optional[str] = None,
        n_threads: Optional[int] = None,
    ):
        """
        初始化标准嵌入器

        Args:
            model_path: 模型路径（None则自动下载，仅在使用 llama-cpp-python 时有效）
            device: 运行设备（'cuda', 'cpu', None自动检测，仅在使用 llama-cpp-python 时有效）
            n_threads: 线程数（None自动检测，仅在使用 llama-cpp-python 时有效）
        """
        # 检查 llama-cpp-python 是否可用
        if not LLAMA_CPP_AVAILABLE:
            print("⚠️  [Standard] llama-cpp-python 未安装，使用简单向量化器")
            print("💡 提示: 安装 llama-cpp-python 以获得更好的效果")
            print("   pip install llama-cpp-python")
            self.embedder = SimpleEmbedder(dimension=512)
            self.dimension = self.embedder.get_dimension()
            self._use_llama = False
            self.model_name = "simple-embedder-512"
            return

        # 使用 llama-cpp-python
        self._use_llama = True
        self.model_name = "all-MiniLM-L6-v2"

        # 模型目录 - 使用用户主目录下的固定位置
        import platform

        system = platform.system()
        if system == "Windows":
            llm_dir = os.path.join(
                os.path.expanduser("~"), "AppData", "Local", "SeeSea", "models"
            )
        elif system == "Darwin":  # macOS
            llm_dir = os.path.join(
                os.path.expanduser("~"),
                "Library",
                "Application Support",
                "SeeSea",
                "models",
            )
        else:  # Linux and other Unix-like systems
            llm_dir = os.path.join(
                os.path.expanduser("~"), ".local", "share", "seesea", "models"
            )
        models_dir = llm_dir
        local_model_file = os.path.join(models_dir, self.MODEL_FILENAME)

        # 确定模型路径
        if model_path is None:
            if os.path.exists(local_model_file):
                print(f"📁 [Standard] 使用已存在模型: {local_model_file}")
                model_path = local_model_file
            else:
                print("⬇️  [Standard] 下载轻量级嵌入模型...")
                os.makedirs(models_dir, exist_ok=True)

                headers = {
                    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
                }

                try:
                    result = get_file(self.MODEL_URL, local_model_file, headers)
                    if result.get("status") != 200:
                        raise RuntimeError(f"下载失败，状态码: {result.get('status')}")
                    print(f"✅ [Standard] 模型下载完成: {local_model_file}")
                except Exception as e:
                    raise RuntimeError(f"模型下载失败: {e}") from e

                model_path = local_model_file

        # GPU配置
        n_gpu_layers = self._detect_gpu(device)

        # 线程配置
        if n_threads is None:
            n_threads = max(1, os.cpu_count() or 4)
        self.n_threads = n_threads

        # 加载模型
        print("🔄 [Standard] 加载嵌入模型...")
        try:
            from llama_cpp import Llama

            self.embedder = Llama(
                model_path=model_path,
                embedding=True,
                n_gpu_layers=n_gpu_layers,
                n_ctx=512,  # 小模型使用较小上下文
                n_threads=n_threads,
                verbose=False,
                use_mmap=True,
                use_mlock=False,
            )

            # 测试获取维度
            if self.embedder is None:
                raise RuntimeError("嵌入模型初始化失败")
            # 调用 llama-cpp-python 的 create_embedding 方法
            llama_embedder = cast(Any, self.embedder)
            test_result = llama_embedder.create_embedding(input="test")
            self.dimension = len(test_result["data"][0]["embedding"])
            print(f"✅ [Standard] 模型加载完成，维度: {self.dimension}")

        except Exception as e:
            # 如果是本地文件且加载失败，尝试重新下载
            if model_path == local_model_file and os.path.exists(local_model_file):
                print("⚠️ [Standard] 本地模型文件损坏，尝试重新下载...")
                try:
                    os.remove(local_model_file)
                    print("⬇️  [Standard] 重新下载模型...")

                    headers = {
                        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
                    }

                    result = get_file(self.MODEL_URL, local_model_file, headers)
                    if result.get("status") != 200:
                        raise RuntimeError(f"下载失败，状态码: {result.get('status')}")

                    print(f"✅ [Standard] 模型重新下载完成: {local_model_file}")

                    # 重新加载模型
                    print("🔄 [Standard] 重新加载嵌入模型...")
                    from llama_cpp import Llama

                    self.embedder = Llama(
                        model_path=local_model_file,
                        embedding=True,
                        n_gpu_layers=n_gpu_layers,
                        n_ctx=512,
                        n_threads=n_threads,
                        verbose=False,
                        use_mmap=True,
                        use_mlock=False,
                    )

                    # 测试获取维度
                    if self.embedder is None:
                        raise RuntimeError("嵌入模型初始化失败")
                    llama_embedder = cast(Any, self.embedder)
                    test_result = llama_embedder.create_embedding(input="test")
                    self.dimension = len(test_result["data"][0]["embedding"])
                    print(f"✅ [Standard] 模型加载完成，维度: {self.dimension}")

                except Exception as download_e:
                    raise RuntimeError(
                        f"模型重新下载后仍然加载失败: {download_e}"
                    ) from download_e
            else:
                raise RuntimeError(f"模型加载失败: {e}") from e

    def _encode_llama(self, texts: List[str], **kwargs) -> List[List[float]]:
        """
        使用 llama-cpp-python 编码

        Args:
            texts: 文本列表
            **kwargs: 其他参数

        Returns:
            向量列表
        """
        if self.embedder is None:
            raise RuntimeError("嵌入模型未初始化")

        # 调用 llama-cpp-python 的 embedding API
        try:
            llama_embedder = cast(Any, self.embedder)
            response = llama_embedder.create_embedding(
                input=texts, model=self.model_name, **kwargs
            )
            embeddings = [item["embedding"] for item in response["data"]]
            return embeddings

        except Exception as e:
            raise RuntimeError(f"编码失败: {e}") from e

    def _detect_gpu(self, device: Optional[str]) -> int:
        """检测GPU配置"""
        if device == "cuda":
            return -1
        elif device == "cpu":
            return 0
        else:
            # 自动检测
            gpu_env_vars = [
                "CUDA_VISIBLE_DEVICES",
                "NVIDIA_VISIBLE_DEVICES",
                "CUDA_PATH",
            ]
            for var in gpu_env_vars:
                if os.environ.get(var):
                    return -1
            return 0

    def encode(
        self, texts: Union[str, List[str]], batch_size: int = 8
    ) -> Union[List[float], List[List[float]]]:
        """
        编码文本为向量

        Args:
            texts: 单个文本或文本列表
            batch_size: 批处理大小（标准模式使用逐个处理）

        Returns:
            单个向量或向量列表
        """
        # 如果使用简单向量化器，直接调用
        if not self._use_llama:
            return self.embedder.encode(texts, batch_size)  # type: ignore

        # 使用 llama-cpp-python
        single_input = isinstance(texts, str)
        texts_to_process: List[str]
        if single_input:
            texts_to_process = [texts]  # type: ignore[list-item]
        else:
            texts_to_process = texts  # type: ignore[assignment]

        # 调用 llama 编码
        embeddings = self._encode_llama(texts_to_process)

        if single_input and embeddings:
            return embeddings[0]
        return embeddings

    def create_embedding(self, text: str) -> List[float]:
        """
        创建嵌入向量（兼容接口）

        Args:
            text: 要编码的文本

        Returns:
            向量
        """
        result = self.encode(text)
        if isinstance(result, list) and len(result) > 0 and isinstance(result[0], list):
            return result[0]
        return result  # type: ignore[return-value]

    def get_dimension(self) -> int:
        """获取向量维度"""
        return self.dimension

    def encode_callback(self, text: str) -> List[float]:
        """
        Rust回调接口

        Args:
            text: 要编码的文本

        Returns:
            向量
        """
        # 如果使用简单向量化器，直接调用
        if not self._use_llama:
            return self.embedder.encode_callback(text)

        result = cast(List[float], self.encode(text))
        return result
