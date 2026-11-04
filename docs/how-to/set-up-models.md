# 准备本地模型

seesea 用两个模型提高排序和章节选择的质量：bge-m3 做嵌入，bge-reranker-v2-m3 做交叉编码重排。没有模型时退回词法排序，搜索仍可用。模型为什么这样用，见[排序与学习](../explanation/ranking-and-learning.md)。

## 查看当前状态

```bash
seesea models
```

输出三行：渲染器（`renderer`）、嵌入模型（`embedding`）和重排模型（`reranker`），每行显示 `ok` 或 `missing`，以及期望的文件路径。

## 下载

```bash
seesea models download
```

不带名字时下载缺失的全部模型（约 420 MB 加 440 MB），默认从 ModelScope 下载。只下载一个：`seesea models download embedding` 或 `seesea models download reranker`。下载先写入 `.part` 文件，完整后才改名。

下载渲染器（Lightpanda）用 `seesea models download renderer`。

## 使用已有的模型文件

把文件复制到模型目录，并使用 `seesea models` 显示的**确切文件名**，大小写要一致：

```bash
mkdir -p ~/.local/share/seesea/models
cp bge-m3-q4_0.gguf ~/.local/share/seesea/models/bge-m3-Q4_0.gguf
cp bge-reranker-v2-m3-q4_k_m.gguf ~/.local/share/seesea/models/bge-reranker-v2-m3-Q4_K_M.gguf
seesea models
```

两行都显示 `ok` 就会被自动使用。文件放在别处时，在配置文件的 `[models]` 里写 `embedding` 和 `reranker` 的路径，或者用 `--model-dir` 与 `--rerank-model`。

## 确认模型起作用

```bash
seesea model ~/.local/share/seesea/models
```

这个命令加载模型、嵌入几个句子，报告向量维度、速度，以及相关与无关句子的相似度，用来判断模型是否能区分它们。

之后做一次 `standard` 搜索，响应里的 `degraded` 不应出现 `semantic rerank failed`，`score_parts.relevance` 应有非零值。

## 控制设备和内存

模型默认在 GPU（Vulkan）上运行，没有可用显卡、显存不够或显卡丢失时自动改用 CPU。

| 目标 | 做法 |
|---|---|
| 完全不用 GPU | 配置里写 `device = "cpu"`，或设 `SEESEA_EMBED_DEVICE=cpu`；CPU 版需要约 3.8 GB 内存 |
| 空闲时释放内存 | `model_idle_minutes`（默认 10，0 表示一直保留） |
| 少用一半内存 | `SEESEA_EMBED_QUANTIZED=1`，评测分数低约 0.01，速度更慢 |
| 让 `fast` 模式之外的搜索更快 | 降低 `--semantic-passages` 或 `--rerank-passages` |

## 调整排序权重

下面的参数都有默认值，只在评测结果提示需要时才改：

```bash
seesea --rerank-weight 0.6 --rerank-center -3 search "query"
```

`--rerank-center` 是映射到相关度 0.5 的 logit。模型很严格，答案的 logit 常在 -4 附近，所以默认值是 -4。`--rerank-scale` 越大，分数越分散。改完用 `seesea eval` 在带标注的查询上比较，见[检查搜索质量](review-search-quality.md)。
