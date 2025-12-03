"""
Embeddings module for text vectorization using Qwen3-Embedding model with llama-cpp-python.
模块名称: embeddings
职责范围: 文本向量化，使用llama-cpp-python加载和运行GGUF格式的Qwen3嵌入模型
期望实现计划: 提供高效的文本嵌入服务，支持模型自动下载和加载
已实现功能: 文本嵌入、批量处理、回调函数支持、模型自动下载
使用依赖: llama-cpp-python, seesea_core
主要接口: TextEmbedder类，包含encode、get_dimension、encode_callback方法
注意事项: 首次使用时会自动下载模型到.tf/models目录
"""

from typing import List, Union, Optional
import os

class TextEmbedder:
    """
    Text embedder using Qwen3-Embedding model with llama-cpp-python backend.

    This class handles the conversion of text to vector embeddings
    that can be used with the Rust vector store, using llama-cpp-python
    to load and run the Qwen3 embedding model in GGUF format.
    """

    def __init__(self, model_path: Optional[str] = None, device: Optional[str] = None):
        """
        Initialize the text embedder with llama-cpp-python.

        Args:
            model_path: Path to the GGUF format Qwen3 embedding model
            device: Device to use ('cuda', 'cpu', or None for auto-detect)
        """
        try:
            # Import required modules
            from llama_cpp import Llama
            # Import seesea_core functions for model download
            from seesea_core import get_file

            # Model settings
            model_filename = "Qwen3-Embedding-0.6B-f16.gguf"
            # Use fixed directory for models
            tf_dir = ".tf"
            models_dir = os.path.join(tf_dir, "models")
            local_model_file = os.path.join(models_dir, model_filename)

            # Set default model path if not provided
            if model_path is None:
                # 1. Check if local model file exists
                if os.path.exists(local_model_file):
                    print(f"Local model file found. Using: {local_model_file}...")
                    model_path = local_model_file
                else:
                    # 2. Create directory if it doesn't exist
                    os.makedirs(models_dir, exist_ok=True)
                    print(f"Local model file not found. Downloading from HF mirror: {model_filename}...")
                    # Download model using seesea_core get_file function with zero-copy
                    model_url = "https://hf-mirror.com/Qwen/Qwen3-Embedding-0.6B-GGUF/resolve/main/Qwen3-Embedding-0.6B-f16.gguf?download=true"
                    
                    # Set custom headers for faster download
                    headers = {
                        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
                    }
                    
                    # Download model using seesea_core get_file (zero-copy optimized for large files)
                    print(f"Downloading model from {model_url} to {local_model_file}...")
                    download_result = get_file(model_url, local_model_file, headers)
                    
                    # Check if download was successful
                    status = download_result["status"]
                    if status != 200:
                        raise RuntimeError(f"Failed to download model. Status code: {status}")
                    
                    # Print download info
                    print(f"Download completed with status: {status}")
                    print(f"File saved to: {download_result['file']['path']}")
                    print(f"File size: {download_result['file']['size'] / (1024*1024*1024):.2f} GB")
                    model_path = local_model_file

            print(f"Loading Qwen3 embedding model from {model_path}...")

            # Configure GPU layers based on device
            n_gpu_layers = 0
            if device == "cuda":
                n_gpu_layers = -1  # Use all GPU layers
            elif device is not None and "cpu" not in device.lower():
                # Try to use GPU if device is not explicitly CPU
                n_gpu_layers = -1

            # Initialize llama-cpp-python with embedding support
            # Use correct parameter names based on llama-cpp-python API
            self.embedder = Llama(
                model_path=model_path,
                embedding=True,  # Enable embedding mode
                n_gpu_layers=n_gpu_layers,
                n_ctx=1024,  # Context size for embedding
                n_threads=4,  # Number of threads to use
                verbose=False  # Reduce verbosity
            )

            # Test embedding to get dimension
            # Call create_embedding with correct parameter name 'input'
            test_embedding = self.embedder.create_embedding(input="test")
            # Extract embedding correctly from the response
            self.dimension = len(test_embedding["data"][0]["embedding"])

            print(f"Model loaded successfully. Embedding dimension: {self.dimension}")

        except ImportError as e:
            raise ImportError(
                "Failed to import required modules. Please install llama-cpp-python and seesea_core first: "
                "'pip install llama-cpp-python seesea_core'"
            ) from e
        except Exception as e:
            raise RuntimeError(
                f"Failed to initialize Qwen3 embedding model with llama-cpp-python: {e}"
            ) from e

    def encode(
        self, texts: Union[str, List[str]], batch_size: int = 32
    ) -> Union[List[float], List[List[float]]]:
        """
        Encode text(s) into vector embeddings using llama-cpp-python.

        Args:
            texts: Single text string or list of text strings
            batch_size: Batch size for processing multiple texts (not used in llama-cpp-python)

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

        # Generate embeddings using llama-cpp-python
        # Use correct parameter name - create_embedding expects 'input' parameter
        result = self.embedder.create_embedding(input=texts)
        
        # Extract embeddings correctly from the response
        # Response format: {"data": [{"embedding": [...]}], "model": "...", "usage": {...}}
        all_embeddings = []
        for item in result.get("data", []):
            embedding = item.get("embedding", [])
            if embedding:
                all_embeddings.append(embedding)
        
        # Return single embedding if single input
        if single_input and all_embeddings:
            return all_embeddings[0]

        return all_embeddings

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
        return self.encode(text)
