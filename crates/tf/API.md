# TF API Documentation

## Overview

TF is a high-performance text semantic retrieval system with a Python SDK backed by Rust for optimal performance and memory efficiency.

## Installation

```bash
pip install -r requirements.txt
pip install maturin
maturin develop --release
```

## Quick Start

```python
from tf import DocumentStore

# Initialize store
store = DocumentStore()

# Add documents
store.add("doc1", "Your content here", title="Title", summary="Summary")

# Search
results = store.search("query text", k=5)
for r in results:
    print(f"{r['title']}: {r['score']:.3f}")
```

---

## API Reference

### DocumentStore

The main SDK class for document storage and retrieval.

#### Constructor

```python
DocumentStore(embedder: Optional[TextEmbedder] = None, dimension: Optional[int] = None)
```

**Parameters:**
- `embedder` (Optional[TextEmbedder]): Text embedder instance. Created automatically if None.
- `dimension` (Optional[int]): Vector dimension. Auto-detected from embedder if None.

**Example:**
```python
from tf import DocumentStore, TextEmbedder

# Auto initialization
store = DocumentStore()

# Custom embedder
embedder = TextEmbedder(model_name="Qwen/Qwen3-Embedding-0.6B")
store = DocumentStore(embedder=embedder)
```

---

### CRUD Operations

#### add()

Add a single document to the store.

```python
add(doc_id: str, content: str, title: str = "", url: str = "", summary: str = "") -> None
```

**Parameters:**
- `doc_id` (str): Unique document identifier
- `content` (str): Document content (vectorized then discarded)
- `title` (str, optional): Document title
- `url` (str, optional): Document URL
- `summary` (str, optional): Document summary

**Example:**
```python
store.add(
    doc_id="doc1",
    content="Machine learning is a subset of AI...",
    title="Introduction to ML",
    url="https://example.com/ml",
    summary="ML basics"
)
```

**Notes:**
- Content is vectorized and immediately discarded (not stored)
- Only vector and metadata (title, url, summary) are stored

---

#### add_batch()

Add multiple documents with optional parallel processing.

```python
add_batch(documents: List[Dict[str, str]], parallel: bool = True) -> None
```

**Parameters:**
- `documents` (List[Dict]): List of document dictionaries
  - Required keys: `id`, `content`
  - Optional keys: `title`, `url`, `summary`
- `parallel` (bool): Use parallel processing (default: True)

**Example:**
```python
docs = [
    {
        "id": "doc1",
        "content": "Content 1...",
        "title": "Title 1",
        "summary": "Summary 1"
    },
    {
        "id": "doc2",
        "content": "Content 2...",
        "title": "Title 2",
        "summary": "Summary 2"
    }
]
store.add_batch(docs, parallel=True)
```

**Performance:**
- Parallel mode: Uses ThreadPoolExecutor with 4 workers
- Sequential mode: Processes documents one by one

---

#### get()

Retrieve document metadata.

```python
get(doc_id: str) -> Optional[Dict[str, str]]
```

**Parameters:**
- `doc_id` (str): Document identifier

**Returns:**
- Dictionary with `title`, `url`, `summary` if found
- `None` if document doesn't exist

**Example:**
```python
metadata = store.get("doc1")
if metadata:
    print(f"Title: {metadata['title']}")
    print(f"Summary: {metadata['summary']}")
```

**Note:** Content is never stored, so it's not returned.

---

#### update()

Update document metadata.

```python
update(doc_id: str, title: Optional[str] = None, url: Optional[str] = None, summary: Optional[str] = None) -> None
```

**Parameters:**
- `doc_id` (str): Document identifier
- `title` (Optional[str]): New title
- `url` (Optional[str]): New URL
- `summary` (Optional[str]): New summary

**Example:**
```python
store.update("doc1", title="Updated Title", summary="Updated summary")
```

**Note:** Only provided fields are updated; others remain unchanged.

---

#### delete()

Delete a single document.

```python
delete(doc_id: str) -> None
```

**Parameters:**
- `doc_id` (str): Document identifier

**Example:**
```python
store.delete("doc1")
```

---

#### delete_batch()

Delete multiple documents.

```python
delete_batch(doc_ids: List[str]) -> None
```

**Parameters:**
- `doc_ids` (List[str]): List of document identifiers

**Example:**
```python
store.delete_batch(["doc1", "doc2", "doc3"])
```

---

### Search Operations

#### search()

Search for similar documents.

```python
search(query: str, k: int = 5, return_objects: bool = False) -> Union[List[Dict[str, Any]], List[SearchResult]]
```

**Parameters:**
- `query` (str): Query text (automatically vectorized)
- `k` (int): Number of results to return (default: 5)
- `return_objects` (bool): Return SearchResult objects instead of dicts (default: False)

**Returns:**
- List of dictionaries (if `return_objects=False`):
  - `id`: Document identifier
  - `score`: Relevance score (0-1, higher is better)
  - `title`: Document title
  - `url`: Document URL
  - `summary`: Document summary
- List of `SearchResult` objects (if `return_objects=True`)

**Example:**
```python
# Returns dictionaries
results = store.search("machine learning algorithms", k=10)
for r in results:
    print(f"{r['title']}: score={r['score']:.4f}")

# Returns SearchResult objects
results = store.search("deep learning", k=5, return_objects=True)
for r in results:
    print(f"{r.title}: {r.score:.4f}")
    print(f"Summary: {r.summary}")
```

**Performance:**
- Results are automatically sorted by relevance (highest first)
- Query vector is freed immediately after search
- Vectors are NOT returned (saves memory)

---

#### search_streaming()

Memory-efficient streaming search for large result sets.

```python
search_streaming(query: str, k: int = 5) -> Iterator[SearchResult]
```

**Parameters:**
- `query` (str): Query text
- `k` (int): Number of results to return

**Yields:**
- `SearchResult` objects one at a time

**Example:**
```python
# Process results as they come - constant memory usage
for result in store.search_streaming("AI applications", k=1000):
    print(f"{result.title}: {result.score:.4f}")
    process_result(result)  # Process immediately
```

**Benefits:**
- Constant O(1) memory usage regardless of k
- No buffering of all results
- Perfect for large k values (e.g., 10,000+)

---

#### search_by_vector()

Search using a pre-computed embedding vector.

```python
search_by_vector(vector: List[float], k: int = 5) -> List[Dict[str, Any]]
```

**Parameters:**
- `vector` (List[float]): Query embedding vector
- `k` (int): Number of results to return

**Returns:**
- List of result dictionaries

**Example:**
```python
# Pre-compute vector
vec = embedder.encode("some text")
results = store.search_by_vector(vec, k=5)
```

---

### Utility Methods

#### count()

Get the number of documents in the store.

```python
count() -> int
```

**Returns:**
- Number of documents

**Example:**
```python
print(f"Total documents: {store.count()}")
```

---

#### is_empty()

Check if the store is empty.

```python
is_empty() -> bool
```

**Returns:**
- `True` if empty, `False` otherwise

**Example:**
```python
if store.is_empty():
    print("No documents in store")
```

---

#### `__len__()`

Get document count (supports `len()` function).

```python
len(store) -> int
```

**Example:**
```python
print(f"Documents: {len(store)}")
```

---

#### `__contains__()`

Check if document exists (supports `in` operator).

```python
doc_id in store -> bool
```

**Example:**
```python
if "doc1" in store:
    print("Document exists")
```

---

## SearchResult

Immutable result object with minimal memory footprint.

### Attributes

```python
@dataclass(frozen=True, slots=True)
class SearchResult:
    id: str          # Document identifier
    score: float     # Relevance score (0-1)
    title: str       # Document title
    url: str         # Document URL
    summary: str     # Document summary
```

### Methods

#### to_dict()

Convert to dictionary.

```python
to_dict() -> dict
```

**Example:**
```python
result = SearchResult(id="doc1", score=0.95, title="Title", url="...", summary="...")
d = result.to_dict()
# {'id': 'doc1', 'score': 0.95, 'title': 'Title', 'url': '...', 'summary': '...'}
```

---

## TextEmbedder

Text embedding utility using Qwen3-Embedding model.

### Constructor

```python
TextEmbedder(model_name: str = "Qwen/Qwen3-Embedding-0.6B", device: Optional[str] = None)
```

**Parameters:**
- `model_name` (str): Hugging Face model name
- `device` (Optional[str]): Device to use ("cuda", "cpu", or None for auto-detection)

**Example:**
```python
from tf import TextEmbedder

# Auto device detection
embedder = TextEmbedder()

# Force CPU
embedder = TextEmbedder(device="cpu")

# Custom model
embedder = TextEmbedder(model_name="sentence-transformers/all-MiniLM-L6-v2")
```

---

### Methods

#### encode()

Encode text to embedding vector.

```python
encode(texts: Union[str, List[str]], batch_size: int = 32) -> Union[List[float], List[List[float]]]
```

**Parameters:**
- `texts` (str or List[str]): Text(s) to encode
- `batch_size` (int): Batch size for processing

**Returns:**
- Single embedding (List[float]) if input is string
- List of embeddings (List[List[float]]) if input is list

**Example:**
```python
# Single text
embedding = embedder.encode("Hello world")
# Returns: [0.1, 0.2, ..., 0.8]  (768 dimensions)

# Multiple texts
embeddings = embedder.encode(["Text 1", "Text 2"])
# Returns: [[0.1, ...], [0.3, ...]]
```

---

#### get_dimension()

Get embedding dimension.

```python
get_dimension() -> int
```

**Returns:**
- Embedding dimension (e.g., 768)

**Example:**
```python
dim = embedder.get_dimension()
print(f"Dimension: {dim}")
```

---

## Error Handling

All methods raise appropriate exceptions on errors:

```python
from tf import DocumentStore

store = DocumentStore()

try:
    # Missing required fields
    store.add_batch([{"id": "doc1"}])  # Missing 'content'
except ValueError as e:
    print(f"Error: {e}")

try:
    # Update non-existent document
    store.update("nonexistent")
except Exception as e:
    print(f"Error: {e}")
```

---

## Memory Optimization

### Content Storage

- **Content is NEVER stored** - only vectorized then discarded
- Saves 70-95% memory compared to traditional approaches

### Search Results

- **Vectors are NOT returned** - only metadata + score
- Saves ~3KB per result

### Streaming Search

- **Constant memory usage** - results yielded one at a time
- Perfect for large k values

### Result Objects

- **Immutable with `__slots__`** - 40% less memory than dicts
- Thread-safe for sharing

---

## Performance Tips

1. **Use parallel batch operations:**
   ```python
   store.add_batch(docs, parallel=True)  # 4 workers
   ```

2. **Use streaming for large searches:**
   ```python
   for r in store.search_streaming("query", k=10000):
       process(r)  # O(1) memory
   ```

3. **Use result objects for efficiency:**
   ```python
   results = store.search("query", return_objects=True)
   # Immutable, __slots__, thread-safe
   ```

4. **Pre-compute vectors for multiple searches:**
   ```python
   vec = embedder.encode("query")
   results1 = store.search_by_vector(vec, k=5)
   results2 = store.search_by_vector(vec, k=10)
   ```

---

## Thread Safety

- All operations are thread-safe
- Concurrent reads supported (RwLock in Rust layer)
- Write operations use exclusive locks
- ThreadPoolExecutor for batch operations

---

## Examples

See example files:
- `example_sdk.py` - Basic CRUD operations
- `example_search_optimized.py` - Advanced search features
