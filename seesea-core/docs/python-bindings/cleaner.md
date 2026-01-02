# 清洗器模块文档

## 概述

清洗器模块提供了高性能的数据预处理和清洗功能，采用 SIMD 优化、零拷贝操作和并发处理技术。该模块专门用于文本数据的智能分割、解析和优化，为后续的向量化处理提供高质量的输入数据。

## 核心类

### PyCleaner

清洗器主类，提供文本预处理功能。

#### 构造函数

```python
cleaner = PyCleaner(max_lines_per_block=None)
```

**参数：**
- `max_lines_per_block` (int, 可选): 每个数据块的最大行数，默认为 50

**说明：**
- 自动检测当前是否存在 Tokio 运行时
- 如果已存在运行时，复用现有运行时
- 如果不存在运行时，创建新的运行时

#### 方法

##### process()

处理文本，返回清洗后的数据块列表。

**参数：**
- `text` (str): 要处理的文本内容

**返回值：**
数据块对象列表（List[PyDataBlock]），每个数据块包含处理后的信息

**示例：**
```python
text = """
# 人工智能简介

人工智能是计算机科学的一个分支，致力于创建能够执行通常需要人类智能的任务的系统。

## 主要应用领域

- 机器学习
- 自然语言处理
- 计算机视觉
- 机器人技术

## 技术挑战

人工智能面临的主要挑战包括数据质量、算法偏见和计算资源需求。
"""

blocks = cleaner.process(text)
for block in blocks:
    print(f"数据块 {block.start_line}-{block.end_line}:")
    print(f"内容: {block.content[:100]}...")
    print(f"评分: {block.score}")
    print(f"有效性: {block.is_valid}")
```

##### batch_process()

批量处理多个文本，返回每个文本对应的数据块列表。

**参数：**
- `texts` (List[str]): 要处理的文本列表

**返回值：**
二维列表，每个元素是对应文本的数据块列表（List[List[PyDataBlock]]）

**示例：**
```python
texts = [
    "文本内容1...",
    "文本内容2...",
    "文本内容3..."
]

results = cleaner.batch_process(texts)
for i, blocks in enumerate(results):
    print(f"文本 {i+1} 处理完成，生成 {len(blocks)} 个数据块")
```

##### process_with_context()

处理文本并返回清洗后的上下文内容，自动过滤无效数据块。

**参数：**
- `text` (str): 要处理的文本内容

**返回值：**
清洗后的文本内容（str），已拼接所有有效数据块

**示例：**
```python
context = cleaner.process_with_context(text)
print(f"清洗后的上下文长度: {len(context)} 字符")
```

### PyDataBlock

数据块类，表示处理后的文本片段。

#### 属性

##### content
数据块的内容文本。

**类型：** str

##### start_line
数据块在原文中的起始行号。

**类型：** int

##### end_line
数据块在原文中的结束行号。

**类型：** int

##### title_relevance
标题相关性评分（0.0-1.0）。

**类型：** float

##### coherence
数据块内部连贯性评分（0.0-1.0）。

**类型：** float

##### score
数据块的综合评分。

**类型：** float

##### links
数据块中提取的链接列表。

**类型：** List[str]

##### images
数据块中提取的图片链接列表。

**类型：** List[str]

##### is_valid
数据块是否有效。

**类型：** bool

##### title_vector
标题的向量表示（如果有）。

**类型：** Optional[List[float]]

##### content_vector
内容的向量表示（如果有）。

**类型：** Optional[List[float]]

##### keyword_similarity
关键词相似度评分。

**类型：** float

#### 方法

##### get_extracted_kv()

获取数据块中提取的键值对。

**返回值：**
字典对象，包含提取的键值对信息

**示例：**
```python
kv_pairs = block.get_extracted_kv()
for key, value in kv_pairs.items():
    print(f"{key}: {value}")
```

##### to_dict()

将数据块转换为 Python 字典格式。

**返回值：**
包含所有属性的字典对象

**示例：**
```python
data_dict = block.to_dict()
print(f"数据块字典: {data_dict}")
```

## 技术特性

### SIMD 优化
- 使用单指令多数据流技术加速文本处理
- 并行处理多个字符和字符串操作
- 显著提升大批量文本处理性能

### 零拷贝操作
- 最小化内存分配和数据复制
- 直接在原始数据上进行处理
- 减少内存占用和 GC 压力

### 并发处理
- 利用多核 CPU 并行处理数据块
- 动态调控并发度
- 支持批量处理优化

### 智能分割
- 基于内容结构的智能分割算法
- 考虑语义连贯性和主题相关性
- 支持 Markdown 等特殊格式解析

## 使用示例

### 基础文本清洗
```python
from seesea_core import PyCleaner

cleaner = PyCleaner(max_lines_per_block=30)

# 处理长文本
long_text = """
# 技术文档标题

这是第一段内容，包含一些重要的技术信息。

## 子标题1

这里是子标题1的详细内容，可能包含代码示例：

```python
def example():
    return "Hello World"
```

## 子标题2

这里是子标题2的内容，包含链接和图片：

访问 [官方网站](https://example.com) 获取更多信息。

![架构图](https://example.com/diagram.png)
"""

blocks = cleaner.process(long_text)
print(f"生成了 {len(blocks)} 个数据块")

for i, block in enumerate(blocks):
    print(f"\n数据块 {i+1}:")
    print(f"行号范围: {block.start_line}-{block.end_line}")
    print(f"内容长度: {len(block.content)} 字符")
    print(f"评分: {block.score:.3f}")
    print(f"链接数量: {len(block.links)}")
    print(f"图片数量: {len(block.images)}")
```

### 批量处理
```python
# 批量处理多个文档
documents = [
    "文档1内容...",
    "文档2内容...",
    "文档3内容..."
]

# 使用批量处理
results = cleaner.batch_process(documents)

for i, blocks in enumerate(results):
    valid_blocks = [b for b in blocks if b.is_valid]
    print(f"文档 {i+1}: {len(blocks)} 个数据块，{len(valid_blocks)} 个有效")
```

### 上下文提取
```python
# 提取清洗后的上下文
raw_text = """
# 标题

一些无关的内容...

## 重要章节

这里是真正重要的内容，需要保留。

更多无关内容...

## 另一个重要章节

这里也是需要保留的重要内容。
"""

# 获取清洗后的上下文
clean_context = cleaner.process_with_context(raw_text)
print("清洗后的上下文:")
print(clean_context)
```

## 性能优化建议

### 内存管理
- 合理设置 `max_lines_per_block` 参数
- 批量处理大量文本时使用 `batch_process()`
- 及时释放不再使用的数据块对象

### 并发控制
- 利用批量处理功能提高吞吐量
- 根据系统资源调整并发度
- 监控系统负载避免过载

### 数据质量
- 预处理输入文本，移除明显无效内容
- 合理设置评分阈值过滤低质量数据块
- 定期检查处理结果的准确性

## 错误处理

清洗器模块会抛出以下 Python 异常：

- `PyRuntimeError`: 运行时错误，如处理失败、内存不足等
- `PyValueError`: 参数值错误，如无效的文本输入

**建议的错误处理：**
```python
try:
    blocks = cleaner.process(text)
except RuntimeError as e:
    print(f"处理失败: {e}")
    # 降级处理或重试
except ValueError as e:
    print(f"参数错误: {e}")
    # 检查输入参数
```

## 相关模块

- [配置模块](config.md): 清洗器参数配置
- [系统控制模块](system_controller.md): 系统资源管理
- [搜索模块](search.md): 搜索结果预处理

## 注意事项

1. **文本长度**: 建议处理长度适中的文本片段，避免过大的单块文本
2. **内存使用**: 批量处理时注意内存占用，避免同时处理过多大文本
3. **性能调优**: 根据实际硬件配置调整并发参数
4. **数据验证**: 处理结果建议进行质量验证
5. **错误恢复**: 实现适当的错误处理和重试机制