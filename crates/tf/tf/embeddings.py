"""
Embeddings module for text vectorization using Qwen3-Embedding model with mistral.rs.
"""

from typing import List, Union, Optional
import os

# Import mistralrs module
try:
    import mistralrs
except ImportError:
    mistralrs = None


class TextEmbedder:
    """
    Text embedder using Qwen3-Embedding model with mistral.rs backend.

    This class handles the conversion of text to vector embeddings
    that can be used with the Rust vector store, using mistral.rs
    to load and run the Qwen3 embedding model.
    """

    def __init__(self, model_path: Optional[str] = None, device: Optional[str] = None):
        """
        Initialize the text embedder with mistral.rs.

        Args:
            model_path: Path to the GGUF format Qwen3 embedding model
            device: Device to use ('cuda', 'cpu', or None for auto-detect)
        """
        try:
            # Import mistralrs module
            import mistralrs

            # Set HF_ENDPOINT environment variable for faster model downloads
            os.environ["HF_ENDPOINT"] = "https://hf-mirror.com"

            # Default Hugging Face model ID for Qwen3 embedding
            default_hf_model_id = "Qwen/Qwen3-Embedding-0.6B-GGUF"

            # Set default model path if not provided
            if model_path is None:
                # Try to find Qwen3 embedding model in default locations
                default_paths = [
                    "./qwen3-embedding.gguf",
                    "./models/qwen3-embedding.gguf",
                    "/models/qwen3-embedding.gguf",
                ]

                for path in default_paths:
                    if os.path.exists(path):
                        model_path = path
                        break

                if model_path is None:
                    # Model not found locally, use mistralrs to download from Hugging Face
                    print("Model not found locally. Using mistralrs to load from Hugging Face...")

                    # Create models directory if it doesn't exist for caching
                    models_dir = "./models"
                    os.makedirs(models_dir, exist_ok=True)

                    # Configure embedding model using Hugging Face model ID
                    embedding_config = mistralrs.Which.Embedding(
                        model_id=default_hf_model_id,
                        arch=mistralrs.EmbeddingArchitecture.Qwen3Embedding,
                        dtype=mistralrs.ModelDType.Auto,
                        hf_cache_path=models_dir,
                    )

                    # Initialize runner with HF model ID - this will automatically download and cache the model
                    self.runner = mistralrs.Runner(which=embedding_config, max_seqs=16)

                    # Test embedding to get dimension
                    test_embedding = self.runner.send_embedding_request(
                        mistralrs.EmbeddingRequest(input="test")
                    )
                    self.dimension = len(test_embedding[0])

                    print(
                        f"Model loaded successfully from Hugging Face. Embedding dimension: {self.dimension}"
                    )
                    return

            print(f"Loading Qwen3 embedding model from {model_path}...")

            # Configure embedding model using local file path
            embedding_config = mistralrs.Which.Embedding(
                model_id=model_path,
                arch=mistralrs.EmbeddingArchitecture.Qwen3Embedding,
                dtype=mistralrs.ModelDType.Auto,
            )

            # Initialize mistralrs Runner with embedding model
            self.runner = mistralrs.Runner(which=embedding_config, max_seqs=16)

            # Test embedding to get dimension
            test_embedding = self.runner.send_embedding_request(
                mistralrs.EmbeddingRequest(input="test")
            )
            self.dimension = len(test_embedding[0])

            print(f"Model loaded successfully. Embedding dimension: {self.dimension}")

        except ImportError as e:
            raise ImportError(
                "Failed to import mistralrs module. Please install it first using "
                "'pip install mistralrs' or build from source: "
                "https://github.com/EricLBuehler/mistral.rs"
            ) from e
        except Exception as e:
            raise RuntimeError(
                f"Failed to initialize Qwen3 embedding model with mistral.rs: {e}"
            ) from e

    def encode(
        self, texts: Union[str, List[str]], batch_size: int = 32
    ) -> Union[List[float], List[List[float]]]:
        """
        Encode text(s) into vector embeddings using mistral.rs.

        Args:
            texts: Single text string or list of text strings
            batch_size: Batch size for processing multiple texts

        Returns:
            Single embedding (List[float]) if input is a string,
            or list of embeddings (List[List[float]]) if input is a list
        """
        # Import mistralrs module again to ensure it's available
        import mistralrs

        # Handle single text input
        if isinstance(texts, str):
            texts = [texts]
            single_input = True
        else:
            single_input = False

        all_embeddings = []

        # Process in batches
        for i in range(0, len(texts), batch_size):
            batch_texts = texts[i : i + batch_size]

            # Generate embeddings using mistralrs Runner
            batch_embeddings = self.runner.send_embedding_request(
                mistralrs.EmbeddingRequest(input=batch_texts)
            )

            all_embeddings.extend(batch_embeddings)

        # Return single embedding if single input
        if single_input:
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
