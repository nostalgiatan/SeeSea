"""
Embeddings module for text vectorization using Qwen3-Embedding model with llama-cpp-python.
模块名称: llama_cpp_embeddings
职责范围: 文本向量化，使用llama-cpp-python加载和运行GGUF格式的Qwen3嵌入模型
期望实现计划: 提供高效的文本嵌入服务，支持模型自动下载和加载
已实现功能: 文本嵌入、批量处理、回调函数支持、模型自动下载
使用依赖: llama-cpp-python, seesea_core
主要接口: LlamaCppEmbedder类，包含encode、get_dimension、encode_callback方法
注意事项: 首次使用时会自动下载模型到.llm/models目录
"""

from typing import List, Union, Optional
import os


class LlamaCppEmbedder:
    """
    Text embedder using Qwen3-Embedding model with llama-cpp-python backend.

    This class handles the conversion of text to vector embeddings
    that can be used with the Rust vector store, using llama-cpp-python
    to load and run the Qwen3 embedding model in GGUF format.
    """

    def __init__(
        self,
        model_path: Optional[str] = None,
        device: Optional[str] = None,
        n_threads: Optional[int] = None,
    ):
        """
        Initialize the text embedder with llama-cpp-python.

        Args:
            model_path: Path to the GGUF format Qwen3 embedding model
            device: Device to use ('cuda', 'cpu', or None for auto-detect)
            n_threads: Number of threads to use for embedding generation. If None, auto-detects based on CPU cores.
        """
        try:
            # Import required modules
            from llama_cpp import Llama

            # Import seesea_core functions for model download
            from seesea_core import get_file

            # Model settings
            model_filename = "Qwen3-Embedding-0.6B-f16.gguf"
            # Use fixed directory for models
            llm_dir = ".llm"
            models_dir = os.path.join(llm_dir, "models")
            local_model_file = os.path.join(models_dir, model_filename)

            # Set default model path if not provided
            if model_path is None:
                # 1. Check if local model file exists
                if os.path.exists(local_model_file):
                    print(f"📁 检测到已存在模型文件: {local_model_file}")
                    print(f"🔍 模型文件大小: {os.path.getsize(local_model_file)} bytes")
                    model_path = local_model_file
                else:
                    print("❌ 模型文件不存在，开始下载")
                    # 2. Create directory if it doesn't exist
                    os.makedirs(models_dir, exist_ok=True)
                    print(f"📁 创建模型目录: {models_dir}")

                    # Download model using seesea_core get_file function with zero-copy
                    model_url = "https://hf-mirror.com/Qwen/Qwen3-Embedding-0.6B-GGUF/resolve/main/Qwen3-Embedding-0.6B-f16.gguf?download=true"

                    # Set custom headers for faster download
                    headers = {
                        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
                    }

                    print(f"🔄 开始下载模型: {model_filename}")
                    print(f"📥 下载地址: {model_url}")
                    print(f"💾 保存路径: {local_model_file}")

                    # Download model using seesea_core get_file (zero-copy optimized for large files)
                    try:
                        download_result = get_file(model_url, local_model_file, headers)

                        # Check if download was successful
                        status = download_result["status"]
                        print(f"📊 下载状态码: {status}")
                        print(f"📋 下载结果: {download_result}")

                        if status != 200:
                            raise RuntimeError(f"Failed to download model. Status code: {status}")

                        print("✅ 模型下载成功！")
                        print(f"📁 下载的模型文件大小: {os.path.getsize(local_model_file)} bytes")
                    except Exception as download_error:
                        print(f"❌ 模型下载失败: {download_error}")
                        raise

                    model_path = local_model_file

            # Configure GPU layers based on device and auto-detection
            n_gpu_layers = 0

            # 1. Check if device is explicitly set
            if device == "cuda":
                n_gpu_layers = -1  # Use all GPU layers
            elif device is not None and "cpu" not in device.lower():
                # Try to use GPU if device is not explicitly CPU
                n_gpu_layers = -1
            else:
                # 2. Auto-detect GPU if device is None
                # Use lightweight detection methods without adding new dependencies
                try:
                    # Check if CUDA is available via environment variables
                    # and other lightweight methods
                    gpu_env_vars = ["CUDA_VISIBLE_DEVICES", "NVIDIA_VISIBLE_DEVICES"]
                    for env_var in gpu_env_vars:
                        if os.environ.get(env_var):
                            n_gpu_layers = -1  # Auto-detect: use all GPU layers
                            break
                    # Additional fallback: check if CUDA_PATH is set (Windows)
                    if n_gpu_layers == 0 and os.environ.get("CUDA_PATH"):
                        n_gpu_layers = -1
                except Exception:
                    # Ignore any errors during auto-detection
                    pass

            # Configure number of threads
            if n_threads is None:
                # Auto-detect based on CPU cores
                n_threads = os.cpu_count() or 4  # Default to 4 if auto-detection fails

            # Ensure n_threads is at least 1
            n_threads = max(1, n_threads)

            # Save n_threads as an instance attribute for VectorStoreWrapper to access
            self.n_threads = n_threads

            # 加载模型，处理模型文件不完整的情况
            max_attempts = 2  # 最大尝试次数
            current_attempt = 1
            success = False

            while current_attempt <= max_attempts and not success:
                try:
                    # Initialize llama-cpp-python with embedding support
                    # Use correct parameter names based on llama-cpp-python API
                    # Add parameters to reduce verbosity and fix embedding issues
                    self.embedder = Llama(
                        model_path=model_path,
                        embedding=True,  # Enable embedding mode
                        n_gpu_layers=n_gpu_layers,
                        n_ctx=32768,  # Full context size for embedding, match training context
                        n_threads=n_threads,  # Number of threads to use
                        verbose=False,  # Reduce verbosity
                        n_output=0,  # No output needed for embedding models
                        output_format="json",  # Ensure proper output format
                        logits_all=False,  # Don't return logits
                        use_mmap=True,  # Use memory mapping for faster loading
                        use_mlock=False,  # Don't lock memory
                    )

                    # Test embedding to get dimension
                    # Call create_embedding with correct parameter name 'input'
                    test_embedding = self.embedder.create_embedding(input="test")
                    # Extract embedding correctly from the response
                    self.dimension = len(test_embedding["data"][0]["embedding"])

                    success = True
                except Exception as e:
                    error_msg = str(e)
                    current_attempt += 1

                    # 检查是否是模型加载失败，且是第一次尝试
                    if "Failed to load model" in error_msg and current_attempt <= max_attempts:
                        print("❌ 模型加载失败，开始清理并重新下载")
                        # 删除可能损坏的模型文件
                        if os.path.exists(model_path):
                            print(f"🗑️ 删除损坏的模型文件: {model_path}")
                            os.remove(model_path)

                        # 重新创建模型目录
                        print("📁 重新创建模型目录")
                        os.makedirs(os.path.dirname(model_path), exist_ok=True)

                        # 重新下载模型
                        print("🔄 开始重新下载模型")
                        from seesea_core import get_file

                        model_url = "https://hf-mirror.com/Qwen/Qwen3-Embedding-0.6B-GGUF/resolve/main/Qwen3-Embedding-0.6B-f16.gguf?download=true"
                        headers = {
                            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
                        }

                        try:
                            print(f"📥 重新下载地址: {model_url}")
                            print(f"💾 重新保存路径: {model_path}")
                            download_result = get_file(model_url, model_path, headers)
                            print(f"📊 重新下载状态码: {download_result['status']}")
                            print(f"📋 重新下载结果: {download_result}")
                        except Exception as redownload_error:
                            print(f"❌ 重新下载失败: {redownload_error}")
                            raise

                        # 检查下载状态
                        if download_result["status"] != 200:
                            raise RuntimeError(
                                f"重新下载模型失败。状态码: {download_result['status']}"
                            )
                    else:
                        # 超过最大尝试次数或不是模型加载错误，抛出异常
                        raise RuntimeError(
                            f"Failed to initialize Qwen3 embedding model with llama-cpp-python after {current_attempt-1} attempts: {error_msg}"
                        ) from e

        except ImportError as e:
            raise ImportError(
                "Failed to import required modules. Please install llama-cpp-python and seesea_core first: "
                "'pip install llama-cpp-python seesea_core'"
            ) from e

    def encode(
        self, texts: Union[str, List[str]], batch_size: int = 8
    ) -> Union[List[float], List[List[float]]]:
        """
        Encode text(s) into vector embeddings using llama-cpp-python.
        Implements safe processing by handling documents individually when necessary.

        Args:
            texts: Single text string or list of text strings
            batch_size: Not used for safety reasons, individual processing is safer

        Returns:
            Single embedding (List[float]) if input is a string,
            or list of embeddings (List[List[float]]) if input is a list
        """
        # Handle single text input
        if isinstance(texts, str):
            texts = [texts]
            single_input = True
        else:
            single_input = False

        # Limit input text length to avoid llama_decode errors
        max_chars_per_text = 8192  # ~2048 tokens
        truncated_texts = []
        for text in texts:
            if isinstance(text, str) and len(text) > max_chars_per_text:
                truncated_texts.append(text[:max_chars_per_text])
            else:
                truncated_texts.append(text)

        # Generate embeddings with safe processing
        all_embeddings = []

        # Process each document individually to avoid context overflow
        # This is the safest approach for embedding models
        for text in truncated_texts:
            try:
                # Process a single document at a time to avoid context overflow
                result = self.embedder.create_embedding(input=[text])

                # Extract embedding from the response
                if result and "data" in result and result["data"]:
                    data_items = result["data"]
                    embedding = data_items[0].get("embedding", [])
                    if embedding:
                        all_embeddings.append(embedding)
            except Exception:
                # If we encounter any error, just skip this document
                # Avoid complex logging to prevent syntax errors
                pass

        from typing import cast

        # Return single embedding if single input
        if single_input and all_embeddings:
            return cast(List[float], all_embeddings[0])

        return cast(List[List[float]], all_embeddings)

    def get_dimension(self) -> int:
        """
        Get the dimension of the embeddings.

        Returns:
            Embedding dimension
        """
        return self.dimension

    def encode_callback(self, text: str) -> List[float]:
        """
        Callback function for Rust integration.

        This function is designed to be called from Rust to get embeddings.

        Args:
            text: Text to encode

        Returns:
            List of floats representing the embedding
        """
        return self.encode(text)  # type: ignore[return-value]
