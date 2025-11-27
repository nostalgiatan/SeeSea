"""
Embeddings module for text vectorization using Qwen3-Embedding model.
"""

import torch
from transformers import AutoTokenizer, AutoModel
from typing import List, Union
import numpy as np


class TextEmbedder:
    """
    Text embedder using Qwen3/Qwen3-Embedding-0.6B model.
    
    This class handles the conversion of text to vector embeddings
    that can be used with the Rust vector store.
    """
    
    def __init__(self, model_name: str = "Qwen/Qwen3-Embedding-0.6B", device: str = None):
        """
        Initialize the text embedder.
        
        Args:
            model_name: HuggingFace model name (default: Qwen/Qwen3-Embedding-0.6B)
            device: Device to use ('cuda', 'cpu', or None for auto-detect)
        """
        if device is None:
            self.device = "cuda" if torch.cuda.is_available() else "cpu"
        else:
            self.device = device
            
        print(f"Loading model {model_name} on {self.device}...")
        self.tokenizer = AutoTokenizer.from_pretrained(model_name, trust_remote_code=True)
        self.model = AutoModel.from_pretrained(model_name, trust_remote_code=True)
        self.model.to(self.device)
        self.model.eval()
        
        # Get embedding dimension from model
        self.dimension = self.model.config.hidden_size
        print(f"Model loaded. Embedding dimension: {self.dimension}")
    
    def encode(self, texts: Union[str, List[str]], batch_size: int = 32) -> Union[List[float], List[List[float]]]:
        """
        Encode text(s) into vector embeddings.
        
        Args:
            texts: Single text string or list of text strings
            batch_size: Batch size for processing multiple texts
            
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
        
        all_embeddings = []
        
        # Process in batches
        for i in range(0, len(texts), batch_size):
            batch_texts = texts[i:i + batch_size]
            
            # Tokenize
            inputs = self.tokenizer(
                batch_texts,
                padding=True,
                truncation=True,
                max_length=512,
                return_tensors="pt"
            ).to(self.device)
            
            # Generate embeddings
            with torch.no_grad():
                outputs = self.model(**inputs)
                # Use mean pooling on the last hidden state
                embeddings = self._mean_pooling(outputs.last_hidden_state, inputs['attention_mask'])
                # Normalize embeddings
                embeddings = torch.nn.functional.normalize(embeddings, p=2, dim=1)
            
            # Convert to list of floats
            batch_embeddings = embeddings.cpu().numpy().tolist()
            all_embeddings.extend(batch_embeddings)
        
        # Return single embedding if single input
        if single_input:
            return all_embeddings[0]
        
        return all_embeddings
    
    def _mean_pooling(self, hidden_states: torch.Tensor, attention_mask: torch.Tensor) -> torch.Tensor:
        """
        Perform mean pooling on hidden states.
        
        Args:
            hidden_states: Model output hidden states
            attention_mask: Attention mask from tokenizer
            
        Returns:
            Pooled embeddings
        """
        # Expand attention mask to match hidden states shape
        input_mask_expanded = attention_mask.unsqueeze(-1).expand(hidden_states.size()).float()
        
        # Sum hidden states with mask
        sum_embeddings = torch.sum(hidden_states * input_mask_expanded, dim=1)
        
        # Sum mask (to get proper denominator for mean)
        sum_mask = torch.clamp(input_mask_expanded.sum(dim=1), min=1e-9)
        
        # Calculate mean
        return sum_embeddings / sum_mask
    
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
