# Set up local models

seesea uses two models to improve ranking and section choice: bge-m3 for embeddings and bge-reranker-v2-m3 as a cross-encoder reranker. Without them it falls back to word-based ranking and search still works. How they are used is in [Ranking and learning](../explanation/ranking-and-learning.md).

## See the current state

```bash
seesea models
```

It prints three lines: the renderer (`renderer`), the embedding model (`embedding`) and the reranker (`reranker`). Each shows `ok` or `missing` and the file path it expects.

## Download

```bash
seesea models download
```

With no name it downloads every missing model (about 420 MB and 440 MB), from ModelScope by default. To fetch one: `seesea models download embedding` or `seesea models download reranker`. A download goes to a `.part` file and is renamed only when complete.

To get the renderer (Lightpanda): `seesea models download renderer`.

## Use model files you already have

Copy the files into the models directory under the **exact file names** `seesea models` shows. The case must match:

```bash
mkdir -p ~/.local/share/seesea/models
cp bge-m3-q4_0.gguf ~/.local/share/seesea/models/bge-m3-Q4_0.gguf
cp bge-reranker-v2-m3-q4_k_m.gguf ~/.local/share/seesea/models/bge-reranker-v2-m3-Q4_K_M.gguf
seesea models
```

When both lines show `ok`, the models are used automatically. If the files live elsewhere, set `embedding` and `reranker` under `[models]` in the config file, or use `--model-dir` and `--rerank-model`.

## Confirm the models work

```bash
seesea model ~/.local/share/seesea/models
```

This loads the model, embeds a few sentences and reports the vector width, the speed, and the similarity of related and unrelated sentences, so you can tell whether it separates them.

Then run a `standard` search. `degraded` in the response should not contain `semantic rerank failed`, and `score_parts.relevance` should be non-zero.

## Control the device and memory

The models run on the GPU (Vulkan) by default and fall back to the CPU when there is no usable card, memory runs out or the card is lost.

| Goal | How |
|---|---|
| Keep the GPU out of it | `device = "cpu"` in the config, or `SEESEA_EMBED_DEVICE=cpu`; the CPU copy needs about 3.8 GB of RAM |
| Free memory when idle | `model_idle_minutes` (default 10, 0 keeps the models loaded) |
| Use half the memory | `SEESEA_EMBED_QUANTIZED=1`; the evaluation score is about 0.01 lower and it is slower |
| Make non-`fast` searches quicker | Lower `--semantic-passages` or `--rerank-passages` |

## Tune the ranking weights

These options have defaults. Change them only when an evaluation suggests it:

```bash
seesea --rerank-weight 0.6 --rerank-center -3 search "query"
```

`--rerank-center` is the logit that maps to relevance 0.5. The model is strict and an answer's logit often sits near -4, which is why the default is -4. A larger `--rerank-scale` spreads scores out. After a change, compare with `seesea eval` on judged queries; see [Check search quality](review-search-quality.md).
