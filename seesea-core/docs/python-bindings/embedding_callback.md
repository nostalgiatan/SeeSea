# 嵌入回调模块

嵌入回调模块提供Python嵌入模型回调的注册和调用功能，支持标准模式和Pro模式的文本向量化。该模块允许将Python端的嵌入模型集成到Rust系统中，实现灵活的文本嵌入功能。

## 核心功能

### 嵌入模式

模块支持两种嵌入模式：

**标准模式 (Standard)**: 使用轻量级模型，适合对性能要求较高的场景

**Pro模式 (Pro)**: 使用高质量模型，适合对嵌入质量要求较高的场景

### 并发控制

内置并发控制机制，防止嵌入请求过载：
- 支持配置最大并发数
- 自动管理并发许可
- 提供异步批量处理能力

## 函数

### register_embedding_callback()

注册嵌入回调函数。

**函数签名:**
```python
register_embedding_callback(
    callback: callable,
    dimension: int,
    mode: str,
    max_concurrency: Optional[int] = 4
) -> None
```

**参数:**

**callback** (callable): Python嵌入函数，接收文本字符串，返回浮点数列表（向量）

**dimension** (int): 嵌入向量的维度

**mode** (str): 嵌入模式，支持以下值：
- `"standard"`: 标准模式（默认）
- `"pro"`: Pro模式

**max_concurrency** (int, 可选): 最大并发数，默认为4

**说明:**
该函数用于注册Python端的嵌入模型回调。回调函数应该是一个同步函数，接收文本字符串作为输入，返回对应的嵌入向量（浮点数列表）。

**回调函数要求:**
```python
def embedding_callback(text: str) -> List[float]:
    # 将文本转换为嵌入向量
    # 返回浮点数列表，长度必须等于指定的dimension
    return [0.1, 0.2, 0.3, ...]  # 长度为dimension
```

**示例:**
```python
import numpy as np
from typing import List

def simple_embedding(text: str) -> List[float]:
    """简单的词频嵌入示例"""
    # 简单的词频统计作为嵌入
    words = text.lower().split()
    # 创建固定维度的向量
    vector = np.zeros(128)
    for i, word in enumerate(words[:128]):
        vector[i] = len(word) / 10.0  # 简单的特征
    return vector.tolist()

# 注册标准模式嵌入回调
register_embedding_callback(
    callback=simple_embedding,
    dimension=128,
    mode="standard",
    max_concurrency=8
)
```

### unregister_embedding_callback()

取消注册当前的嵌入回调函数。

**函数签名:**
```python
unregister_embedding_callback() -> None
```

**说明:**
移除当前注册的嵌入回调函数，清理相关资源。

**示例:**
```python
# 取消注册嵌入回调
unregister_embedding_callback()
```

### is_embedding_callback_registered()

检查是否已注册嵌入回调函数。

**函数签名:**
```python
is_embedding_callback_registered() -> bool
```

**返回值:**
- `bool`: 如果已注册嵌入回调函数返回True，否则返回False

**示例:**
```python
# 检查注册状态
if is_embedding_callback_registered():
    print("嵌入回调已注册")
else:
    print("嵌入回调未注册")
```

### get_embedding_mode()

获取当前注册的嵌入模式。

**函数签名:**
```python
get_embedding_mode() -> str
```

**返回值:**
- `str`: 当前嵌入模式（"standard"、"pro"或"none"）

**说明:**
返回当前注册的嵌入模式，如果未注册回调函数则返回"none"。

**示例:**
```python
# 获取当前模式
mode = get_embedding_mode()
print(f"当前嵌入模式: {mode}")
```

### get_embedding_dimension()

获取当前注册的嵌入维度。

**函数签名:**
```python
get_embedding_dimension() -> int
```

**返回值:**
- `int`: 当前嵌入向量的维度

**异常:**
- `RuntimeError`: 如果未注册嵌入回调函数

**示例:**
```python
try:
    dimension = get_embedding_dimension()
    print(f"当前嵌入维度: {dimension}")
except RuntimeError as e:
    print(f"错误: {e}")
```

## 使用示例

### 基本使用

```python
from typing import List
import hashlib

# 定义简单的哈希嵌入函数
def hash_embedding(text: str) -> List[float]:
    """基于文本哈希的嵌入函数"""
    # 使用SHA256生成哈希
    hash_obj = hashlib.sha256(text.encode())
    hash_bytes = hash_obj.digest()
    
    # 转换为浮点数向量
    vector = []
    for i in range(0, min(256, len(hash_bytes)), 4):
        # 每4个字节转换为一个浮点数
        value = int.from_bytes(hash_bytes[i:i+4], 'big') / (2**32 - 1)
        vector.append(value)
    
    return vector

# 注册嵌入回调
register_embedding_callback(
    callback=hash_embedding,
    dimension=64,  # SHA256哈希的前64个浮点数
    mode="standard",
    max_concurrency=4
)

# 检查状态
print(f"嵌入回调注册状态: {is_embedding_callback_registered()}")
print(f"当前嵌入模式: {get_embedding_mode()}")
print(f"当前嵌入维度: {get_embedding_dimension()}")

# 使用嵌入功能（通过其他模块调用）
# 实际使用时会通过Rust内部调用
```

### 高级嵌入模型

```python
import numpy as np
from typing import List
import re

class AdvancedEmbeddingModel:
    """高级嵌入模型类"""
    
    def __init__(self, dimension: int = 256):
        self.dimension = dimension
        # 模拟词向量表
        self.word_vectors = {}
        
    def build_vocabulary(self, texts: List[str]):
        """构建词汇表"""
        words = set()
        for text in texts:
            # 简单的分词
            words.update(re.findall(r'\b\w+\b', text.lower()))
        
        # 为每个词生成随机向量
        for word in words:
            if word not in self.word_vectors:
                self.word_vectors[word] = np.random.randn(self.dimension)
    
    def embed(self, text: str) -> List[float]:
        """嵌入文本"""
        words = re.findall(r'\b\w+\b', text.lower())
        
        if not words:
            return np.zeros(self.dimension).tolist()
        
        # 平均词向量
        vectors = []
        for word in words:
            if word in self.word_vectors:
                vectors.append(self.word_vectors[word])
        
        if vectors:
            return np.mean(vectors, axis=0).tolist()
        else:
            return np.zeros(self.dimension).tolist()

# 创建嵌入模型
embedding_model = AdvancedEmbeddingModel(dimension=256)

# 构建词汇表（模拟训练过程）
sample_texts = [
    "machine learning is a subset of artificial intelligence",
    "deep learning uses neural networks",
    "natural language processing helps computers understand text",
    "computer vision enables machines to see and interpret images"
]
embedding_model.build_vocabulary(sample_texts)

# 定义嵌入回调函数
def advanced_embedding(text: str) -> List[float]:
    return embedding_model.embed(text)

# 注册Pro模式嵌入回调
register_embedding_callback(
    callback=advanced_embedding,
    dimension=256,
    mode="pro",
    max_concurrency=2  # Pro模式通常更消耗资源
)

print(f"高级嵌入模型已注册，维度: {get_embedding_dimension()}")
```

### 并发控制

```python
import time
import threading
from typing import List

# 模拟耗时嵌入函数
def slow_embedding(text: str) -> List[float]:
    """模拟耗时的嵌入过程"""
    time.sleep(0.1)  # 模拟计算时间
    # 简单的特征提取
    return [len(text) / 100.0] * 128

# 注册带并发控制的嵌入回调
register_embedding_callback(
    callback=slow_embedding,
    dimension=128,
    mode="standard",
    max_concurrency=2  # 限制最大并发数为2
)

# 并发测试
def embed_text(text: str, thread_id: int):
    """嵌入文本的线程函数"""
    start_time = time.time()
    print(f"线程 {thread_id}: 开始嵌入 '{text[:20]}...'")
    
    # 这里会通过Rust内部调用嵌入功能
    # 实际项目中会通过其他模块使用嵌入结果
    
    end_time = time.time()
    print(f"线程 {thread_id}: 完成，耗时 {end_time - start_time:.3f}秒")

# 启动多个线程测试并发控制
threads = []
for i in range(5):
    thread = threading.Thread(target=embed_text, args=(f"测试文本 {i}", i))
    threads.append(thread)
    thread.start()

# 等待所有线程完成
for thread in threads:
    thread.join()

print("所有嵌入任务完成")
```

### 模式切换

```python
from typing import List

# 定义标准模式嵌入函数
def standard_embedding(text: str) -> List[float]:
    """标准模式：快速但质量一般"""
    # 简单的TF-IDF风格嵌入
    words = text.lower().split()
    vector = [0.0] * 128
    
    for i, word in enumerate(words[:128]):
        # 简单的哈希和权重
        hash_val = hash(word) % 1000 / 1000.0
        vector[i] = hash_val
    
    return vector

# 定义Pro模式嵌入函数
def pro_embedding(text: str) -> List[float]:
    """Pro模式：高质量但较慢"""
    # 更复杂的特征提取
    words = text.lower().split()
    vector = [0.0] * 256
    
    # 更复杂的特征计算
    for i, word in enumerate(words):
        if i < 256:
            # 多维度特征
            length_feature = len(word) / 20.0
            char_feature = ord(word[0]) if word else 0 / 256.0
            freq_feature = words.count(word) / len(words)
            
            vector[i] = (length_feature + char_feature + freq_feature) / 3.0
    
    return vector

# 注册标准模式
register_embedding_callback(
    callback=standard_embedding,
    dimension=128,
    mode="standard",
    max_concurrency=8
)

print(f"标准模式已注册: {get_embedding_mode()}")

# 切换到Pro模式
unregister_embedding_callback()

register_embedding_callback(
    callback=pro_embedding,
    dimension=256,
    mode="pro",
    max_concurrency=2
)

print(f"切换到Pro模式: {get_embedding_mode()}")
print(f"Pro模式维度: {get_embedding_dimension()}")
```

### 错误处理

```python
from typing import List

def faulty_embedding(text: str) -> List[float]:
    """可能出错的嵌入函数"""
    if not text.strip():
        raise ValueError("空文本无法嵌入")
    
    if len(text) > 1000:
        raise ValueError("文本过长")
    
    # 正常处理
    return [0.5] * 128

# 注册嵌入回调
register_embedding_callback(
    callback=faulty_embedding,
    dimension=128,
    mode="standard",
    max_concurrency=4
)

# 测试错误情况
test_cases = [
    "",  # 空文本
    "正常文本",
    "a" * 1500,  # 超长文本
]

for text in test_cases:
    try:
        # 这里会通过Rust内部调用嵌入功能
        # 实际错误会在Rust端处理
        print(f"处理文本: '{text[:20]}...'")
    except Exception as e:
        print(f"处理失败: {e}")

# 清理
unregister_embedding_callback()
```

## 与其他模块集成

### 在搜索中使用嵌入

```python
from seesea_python_bindings import register_embedding_callback
from typing import List

def search_embedding(text: str) -> List[float]:
    """搜索优化的嵌入函数"""
    # 针对搜索场景优化的嵌入
    # 强调关键词权重
    words = text.lower().split()
    vector = [0.0] * 128
    
    # 简单的关键词权重
    for i, word in enumerate(words[:128]):
        # 假设某些词更重要
        weight = 2.0 if word in ["python", "machine", "learning"] else 1.0
        vector[i] = weight * (i + 1) / 128.0
    
    return vector

# 注册搜索优化的嵌入回调
register_embedding_callback(
    callback=search_embedding,
    dimension=128,
    mode="standard",
    max_concurrency=6
)

print("搜索嵌入回调已注册")
```

### 在缓存中使用嵌入

```python
from seesea_python_bindings import register_embedding_callback
from typing import List

def cache_embedding(text: str) -> List[float]:
    """缓存优化的嵌入函数"""
    # 生成稳定的嵌入向量用于缓存键
    import hashlib
    
    # 使用哈希确保相同文本产生相同向量
    hash_obj = hashlib.md5(text.encode())
    hash_hex = hash_obj.hexdigest()
    
    # 转换为固定维度的向量
    vector = []
    for i in range(0, min(64, len(hash_hex)), 2):
        hex_pair = hash_hex[i:i+2]
        value = int(hex_pair, 16) / 255.0  # 归一化到0-1
        vector.append(value)
    
    return vector

# 注册缓存优化的嵌入回调
register_embedding_callback(
    callback=cache_embedding,
    dimension=64,  # 较小的维度用于缓存
    mode="standard",
    max_concurrency=10
)

print("缓存嵌入回调已注册")
```

## 性能优化

### 批量处理

```python
from typing import List
import concurrent.futures

def efficient_embedding(text: str) -> List[float]:
    """高效的嵌入函数"""
    # 使用更高效的算法
    words = text.split()
    vector = [0.0] * 128
    
    # 并行处理（如果可能）
    for i, word in enumerate(words[:128]):
        # 简单的特征提取
        vector[i] = len(word) / 10.0
    
    return vector

# 注册高效嵌入回调
register_embedding_callback(
    callback=efficient_embedding,
    dimension=128,
    mode="standard",
    max_concurrency=8
)
```

### 内存管理

```python
from typing import List
import gc

def memory_efficient_embedding(text: str) -> List[float]:
    """内存高效的嵌入函数"""
    try:
        # 处理文本
        vector = [0.5] * 128  # 简化处理
        
        # 显式内存管理
        del text
        
        return vector
    finally:
        # 强制垃圾回收
        gc.collect()

# 注册内存高效嵌入回调
register_embedding_callback(
    callback=memory_efficient_embedding,
    dimension=128,
    mode="standard",
    max_concurrency=4
)
```

## 错误处理

```python
from typing import List

def robust_embedding(text: str) -> List[float]:
    """健壮的嵌入函数"""
    try:
        # 输入验证
        if not isinstance(text, str):
            text = str(text)
        
        if not text.strip():
            # 空文本返回零向量
            return [0.0] * 128
        
        # 文本截断
        max_length = 10000
        if len(text) > max_length:
            text = text[:max_length]
        
        # 处理文本
        words = text.split()
        vector = [0.0] * 128
        
        for i, word in enumerate(words[:128]):
            vector[i] = len(word) / 20.0
        
        return vector
        
    except Exception as e:
        # 异常处理，返回默认向量
        print(f"嵌入错误: {e}")
        return [0.0] * 128

# 注册健壮嵌入回调
register_embedding_callback(
    callback=robust_embedding,
    dimension=128,
    mode="standard",
    max_concurrency=4
)
```

## 注意事项

1. **回调函数要求**: 嵌入回调函数必须是同步函数，接收字符串参数，返回浮点数列表

2. **维度一致性**: 返回的向量维度必须与注册时指定的dimension参数一致

3. **并发控制**: 合理设置max_concurrency参数，避免系统过载

4. **错误处理**: 回调函数内部应该妥善处理异常情况，避免崩溃

5. **性能考虑**: 嵌入函数的性能直接影响整个系统的响应速度

6. **内存管理**: 注意内存使用，避免内存泄漏

7. **模式选择**: 根据实际需求选择合适的嵌入模式（标准或Pro）

8. **动态切换**: 可以在运行时切换不同的嵌入模型，但需要先取消注册当前回调