# -*- coding: utf-8 -*-
"""
标准模式嵌入器

使用轻量级 all-MiniLM-L6-v2-Q4_K_M 模型，适合资源受限环境。
模型大小约 23MB，维度 384。
"""

from typing import List, Optional, Union, cast
import os
from .manager import BaseEmbedder


class StandardEmbedder(BaseEmbedder):
    """
    标准模式嵌入器

    使用 all-MiniLM-L6-v2-Q4_K_M 模型，特点：
    - 模型小巧（~23MB）
    - 推理速度快
    - 维度384，足够用于相关性计算
    - 适合标准模式下的快速嵌入
    """

    # 模型配置
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
            model_path: 模型路径（None则自动下载）
            device: 运行设备（'cuda', 'cpu', None自动检测）
            n_threads: 线程数（None自动检测）
        """
        try:
            from llama_cpp import Llama
            from seesea_core import get_file
        except ImportError as e:
            raise ImportError(
                "请先安装依赖: pip install llama-cpp-python seesea_core"
            ) from e

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
            test_result = self.embedder.create_embedding(input="test")
            self.dimension = len(test_result["data"][0]["embedding"])
            print(f"✅ [Standard] 模型加载完成，维度: {self.dimension}")

        except Exception as e:
            # 如果是本地文件且加载失败，尝试重新下载
            if model_path == local_model_file and os.path.exists(local_model_file):
                print(f"⚠️ [Standard] 本地模型文件损坏，尝试重新下载...")
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
                    test_result = self.embedder.create_embedding(input="test")
                    self.dimension = len(test_result["data"][0]["embedding"])
                    print(f"✅ [Standard] 模型加载完成，维度: {self.dimension}")
                    
                except Exception as download_e:
                    raise RuntimeError(f"模型重新下载后仍然加载失败: {download_e}") from download_e
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
            batch_size: 批处理大小（标准模式使用逐个处理）

        Returns:
            单个向量或向量列表
        """
        single_input = isinstance(texts, str)
        texts_to_process: List[str]
        if single_input:
            texts_to_process = [texts]  # type: ignore[list-item]
        else:
            texts_to_process = texts  # type: ignore[assignment]

        # 限制文本长度
        max_chars = 2048  # 标准模型上下文较小
        truncated_texts = [
            text[:max_chars] if len(text) > max_chars else text
            for text in texts_to_process
        ]

        all_embeddings = []
        for text in truncated_texts:
            try:
                result = self.embedder.create_embedding(input=[text])
                if result and "data" in result and result["data"]:
                    embedding = cast(
                        List[float], result["data"][0].get("embedding", [])
                    )
                    if embedding:
                        all_embeddings.append(embedding)
            except Exception:
                pass  # 跳过失败的文本

        if single_input and all_embeddings:
            return all_embeddings[0]
        return all_embeddings

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
        result = cast(List[float], self.encode(text))
        return result
