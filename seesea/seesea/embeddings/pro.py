# -*- coding: utf-8 -*-
"""
Pro模式嵌入器

优先使用高质量 Qwen3-Embedding-0.6B-Q8_0 模型（需要 llama-cpp-python），
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


class ProEmbedder(BaseEmbedder):
    """
    Pro模式嵌入器

    如果 llama-cpp-python 可用，使用 Qwen3-Embedding-0.6B-Q8_0 模型：
    - 高质量嵌入（Q8_0量化保留更多精度）
    - 维度1024，语义表达能力更强
    - 支持32K上下文
    - 适合Pro模式下的高精度语义搜索

    如果 llama-cpp-python 不可用，使用纯 Python 实现的简单向量化器：
    - 无需外部依赖
    - 固定维度 512
    - 基于词频和哈希
    """

    # 模型配置（仅在使用 llama-cpp-python 时使用）
    MODEL_FILENAME = "Qwen3-Embedding-0.6B-Q8_0.gguf"
    MODEL_URL = "https://hf-mirror.com/Qwen/Qwen3-Embedding-0.6B-GGUF/resolve/main/Qwen3-Embedding-0.6B-Q8_0.gguf?download=true"
    EXPECTED_DIMENSION = 1024

    def __init__(
        self,
        model_path: Optional[str] = None,
        device: Optional[str] = None,
        n_threads: Optional[int] = None,
    ):
        """
        初始化Pro嵌入器

        Args:
            model_path: 模型路径（None则自动下载，仅在使用 llama-cpp-python 时有效）
            device: 运行设备（'cuda', 'cpu', None自动检测，仅在使用 llama-cpp-python 时有效）
            n_threads: 线程数（None自动检测，仅在使用 llama-cpp-python 时有效）
        """
        # 检查 llama-cpp-python 是否可用
        if not LLAMA_CPP_AVAILABLE:
            print("⚠️  [Pro] llama-cpp-python 未安装，使用简单向量化器")
            print("💡 提示: 安装 llama-cpp-python 以获得更好的效果")
            print("   pip install llama-cpp-python")
            self.embedder = SimpleEmbedder(dimension=512)
            self.dimension = self.embedder.get_dimension()
            self._use_llama = False
            self.model_name = "simple-embedder-512"
            return

        # 使用 llama-cpp-python
        self._use_llama = True
        self.model_name = "Qwen3-Embedding-0.6B"

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
                print(f"📁 [Pro] 使用已存在模型: {local_model_file}")
                model_path = local_model_file
            else:
                print("⬇️  [Pro] 下载高质量嵌入模型（Q8_0量化）...")
                os.makedirs(models_dir, exist_ok=True)

                headers = {
                    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
                }

                try:
                    result = get_file(self.MODEL_URL, local_model_file, headers)
                    if result.get("status") != 200:
                        raise RuntimeError(f"下载失败，状态码: {result.get('status')}")
                    print(f"✅ [Pro] 模型下载完成: {local_model_file}")
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
        print("🔄 [Pro] 加载高质量嵌入模型...")
        self._load_model(model_path, n_gpu_layers, n_threads)

    def _load_model(
        self, model_path: str, n_gpu_layers: int, n_threads: int, retry: bool = True
    ):
        """加载模型，支持重试"""
        from llama_cpp import Llama

        try:
            self.embedder = Llama(
                model_path=model_path,
                embedding=True,
                n_gpu_layers=n_gpu_layers,
                n_ctx=32768,  # 完整32K上下文
                n_threads=n_threads,
                verbose=False,
                n_output=0,
                logits_all=False,
                use_mmap=True,
                use_mlock=False,
            )

            # 测试获取维度
            if self.embedder is None:
                raise RuntimeError("嵌入模型初始化失败")
            llama_embedder = cast(Any, self.embedder)
            test_result = llama_embedder.create_embedding(input="test")
            self.dimension = len(test_result["data"][0]["embedding"])
            print(f"✅ [Pro] 模型加载完成，维度: {self.dimension}")

        except Exception as e:
            if retry and "Failed to load model" in str(e):
                print("❌ [Pro] 模型加载失败，尝试重新下载...")
                if os.path.exists(model_path):
                    os.remove(model_path)

                from seesea_core import get_file

                headers = {
                    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
                }
                get_file(self.MODEL_URL, model_path, headers)

                # 重试加载（不再重试）
                self._load_model(model_path, n_gpu_layers, n_threads, retry=False)
            else:
                raise RuntimeError(f"模型加载失败: {e}") from e

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
            batch_size: 批处理大小

        Returns:
            单个向量或向量列表
        """
        if self.embedder is None:
            raise RuntimeError("嵌入模型未初始化")

        single_input = isinstance(texts, str)

        if single_input:
            texts_to_process: List[str] = [texts]  # type: ignore[list-item]
        else:
            texts_to_process = texts  # type: ignore[assignment]

        # 调用 llama-cpp-python 的 embedding API
        try:
            llama_embedder = cast(Any, self.embedder)
            response = llama_embedder.create_embedding(
                input=texts_to_process,
                model=self.model_name,
            )
            embeddings = [item["embedding"] for item in response["data"]]
            typed_embeddings: List[List[float]] = cast(List[List[float]], embeddings)

            if single_input and typed_embeddings:
                return typed_embeddings[0]
            return typed_embeddings

        except Exception as e:
            raise RuntimeError(f"编码失败: {e}") from e

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
        return result  # type: ignore

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
        if not self._use_llama:
            return self.embedder.encode_callback(text)

        result = cast(List[float], self.encode(text))
        return result
